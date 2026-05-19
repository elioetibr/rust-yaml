//! YAML parser for converting tokens to events

use crate::{
    BasicScanner, Error, Limits, Position, Result, Scanner, Token, TokenType, error::ErrorContext,
    tag::TagResolver,
};

pub mod events;
pub mod streaming;
// pub mod optimizations; // Temporarily disabled
pub use events::*;
pub use streaming::*;
// pub use optimizations::*;

/// Trait for YAML parsers that convert token streams to events
pub trait Parser {
    /// Check if there are more events available
    fn check_event(&self) -> bool;

    /// Peek at the next event without consuming it
    fn peek_event(&self) -> Result<Option<&Event>>;

    /// Get the next event, consuming it
    fn get_event(&mut self) -> Result<Option<Event>>;

    /// Reset the parser state
    fn reset(&mut self);

    /// Get the current position in the input
    fn position(&self) -> Position;
}

/// Walks back through `events` looking for an unclosed `DocumentStart`
/// (i.e. one without a matching `DocumentEnd` after it). Returns true if
/// the parser is still inside a document.
fn has_open_document(events: &[Event]) -> bool {
    for ev in events.iter().rev() {
        match &ev.event_type {
            EventType::DocumentEnd { .. } => return false,
            EventType::DocumentStart { .. } => return true,
            _ => {}
        }
    }
    false
}

/// Emit `MappingEnd` / `SequenceEnd` events to close any unbalanced
/// collection starts in `events`. Called before emitting a `DocumentEnd`
/// when an outer construct (e.g. a new `---` marker) forces the previous
/// document closed without going through the usual indent-driven
/// `BlockEnd` token path. Also synthesises implicit empty scalars for
/// mappings that have an odd child-event count (i.e. a key without a
/// value before the close).
/// Return true when there is at least one *flow* collection still open
/// (a `SequenceStart` / `MappingStart` with `flow_style=true` without a
/// matching `…End` afterwards). Used at end-of-stream to enforce §7.4.
fn has_unclosed_flow_collection(events: &[Event]) -> bool {
    let mut depth: i32 = 0;
    for ev in events.iter() {
        match &ev.event_type {
            EventType::SequenceStart {
                flow_style: true, ..
            }
            | EventType::MappingStart {
                flow_style: true, ..
            } => depth += 1,
            EventType::SequenceEnd | EventType::MappingEnd if depth > 0 => {
                depth -= 1;
            }
            _ => {}
        }
    }
    depth > 0
}

/// Walk `events` to detect a document that already contains a closed
/// root-level node. Returns true when the second root node arrives and
/// the existing event stack has no unmatched MapStart/SeqStart.
fn second_root_node_present(events: &[Event]) -> bool {
    let mut after_doc_start = false;
    let mut has_root_node = false;
    let mut depth = 0i32;
    for e in events.iter() {
        match &e.event_type {
            EventType::DocumentStart { .. } => {
                after_doc_start = true;
                has_root_node = false;
                depth = 0;
            }
            EventType::DocumentEnd { .. } => after_doc_start = false,
            EventType::MappingStart { .. } | EventType::SequenceStart { .. } => depth += 1,
            EventType::MappingEnd | EventType::SequenceEnd => {
                depth -= 1;
                if depth == 0 {
                    has_root_node = true;
                }
            }
            EventType::Scalar { .. } | EventType::Alias { .. } if depth == 0 => {
                has_root_node = true;
            }
            _ => {}
        }
    }
    after_doc_start && has_root_node && depth == 0
}

/// Return true when the innermost still-open mapping has an odd number
/// of children — i.e. a key has been emitted but its value has not.
/// Used to decide when to synthesise an implicit empty scalar.
fn innermost_mapping_has_odd_children(events: &[Event]) -> bool {
    let mut stack: Vec<(&'static str, usize)> = Vec::new();
    for ev in events.iter() {
        match &ev.event_type {
            EventType::DocumentStart { .. } | EventType::DocumentEnd { .. } => {
                stack.clear();
            }
            EventType::MappingStart { .. } => stack.push(("map", 0)),
            EventType::SequenceStart { .. } => stack.push(("seq", 0)),
            EventType::MappingEnd | EventType::SequenceEnd => {
                stack.pop();
                if let Some(parent) = stack.last_mut() {
                    parent.1 += 1;
                }
            }
            EventType::Scalar { .. } | EventType::Alias { .. } => {
                if let Some(parent) = stack.last_mut() {
                    parent.1 += 1;
                }
            }
            _ => {}
        }
    }
    matches!(stack.last(), Some(("map", n)) if n % 2 == 1)
}

fn close_open_collections(events: &mut Vec<Event>, pos: Position) {
    // Each entry: (kind, children_at_this_depth) where `kind` is "map"
    // or "seq". `children` counts top-level node events emitted inside
    // this collection (Scalar, Alias, or a closed nested collection).
    let mut stack: Vec<(&'static str, usize)> = Vec::new();
    for ev in events.iter() {
        match &ev.event_type {
            EventType::DocumentStart { .. } | EventType::DocumentEnd { .. } => {
                stack.clear();
            }
            EventType::MappingStart { .. } => stack.push(("map", 0)),
            EventType::SequenceStart { .. } => stack.push(("seq", 0)),
            EventType::MappingEnd | EventType::SequenceEnd => {
                stack.pop();
                if let Some(parent) = stack.last_mut() {
                    parent.1 += 1;
                }
            }
            EventType::Scalar { .. } | EventType::Alias { .. } => {
                if let Some(parent) = stack.last_mut() {
                    parent.1 += 1;
                }
            }
            _ => {}
        }
    }
    while let Some((kind, children)) = stack.pop() {
        if kind == "map" && children % 2 == 1 {
            // Odd child count → last key has no value yet. Spec says
            // emit implicit empty scalar (YAML 1.2 §6.9.1).
            events.push(Event::scalar(
                pos,
                None,
                None,
                String::new(),
                true,
                false,
                ScalarStyle::Plain,
            ));
        }
        match kind {
            "map" => events.push(Event::mapping_end(pos)),
            "seq" => events.push(Event::sequence_end(pos)),
            _ => {}
        }
    }
}

/// Basic parser implementation that converts tokens to events
#[derive(Debug)]
pub struct BasicParser {
    scanner: BasicScanner,
    events: Vec<Event>,
    event_index: usize,
    state: ParserState,
    state_stack: Vec<ParserState>,
    position: Position,
    pending_anchor: Option<String>,
    /// Line where `pending_anchor` was set. Used to distinguish a
    /// "freestanding" anchor (alone on its own line — belongs to the
    /// upcoming collection) from an "inline" anchor (same line as the
    /// next key — belongs to that key). yaml-test-suite 6BFJ, 9KAX.
    pending_anchor_line: Option<usize>,
    /// Line of the most recent `?` Key marker. Used to detect when
    /// an explicit-key construct has an inline single-pair mapping as
    /// its key (yaml-test-suite M2N8/00, M2N8/01, V9D5).
    last_key_marker_line: Option<usize>,
    /// Column of the most recent `?` Key marker. Used in V9D5: when
    /// a `:` arrives at the same column as the most recent `?` on
    /// a later line, it's the explicit value separator — close any
    /// inline-wrapped inner mapping first.
    last_key_marker_column: Option<usize>,
    /// Set when an explicit value separator just closed an inline-
    /// wrapped key. The next `<scalar>:<scalar>` on the same line
    /// should also be wrapped in an inner mapping (V9D5's value side).
    just_closed_inline_wrap: bool,
    /// Column of an open inline-wrap mapping (V9D5). Used to detect
    /// the matching explicit-value separator and close it.
    inline_wrap_column: Option<usize>,
    pending_tag: Option<String>,
    /// Same idea as `pending_anchor_line` but for tags. Used to detect
    /// a freestanding tag in block-sequence context that should be
    /// flushed as the previous item's empty value rather than carried
    /// onto the next item (yaml-test-suite FH7J).
    pending_tag_line: Option<usize>,
    last_token_type: Option<TokenType>,
    scanning_error: Option<Error>,
    yaml_version: Option<(u8, u8)>,
    tag_directives: Vec<(String, String)>,
    tag_resolver: TagResolver,
    /// Anchor names that have been defined so far in the stream. Used to
    /// validate that aliases (`*name`) reference a known anchor (YAML 1.2
    /// §6.9.2). Forward references are forbidden, and we never reset this
    /// set — once defined, an anchor remains referenceable for the rest of
    /// the parse, matching common loader semantics.
    defined_anchors: std::collections::HashSet<String>,
    /// Line of the most recent `:` Value token. Used by the
    /// BlockMappingValue heuristic to tell apart "same-line value
    /// scalar" (6M2F) from "next-line sibling key" (6KGN).
    last_value_token_line: Option<usize>,
    /// True while we're holding an explicit `?` key that has not yet
    /// received its `:`. Used at end-of-stream to distinguish a
    /// spec-legal `? key` with implicit empty value from a missing-`:`
    /// bare scalar (yaml-test-suite 7MNF).
    explicit_key_pending: bool,
    /// Counts implicit single-pair flow mappings still open. A `,` or
    /// `]` while this is > 0 closes the innermost implicit mapping
    /// before continuing the outer flow sequence (§7.5).
    implicit_flow_pair_depth: usize,
}

/// Parser state for tracking context
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum ParserState {
    StreamStart,
    StreamEnd,
    ImplicitDocumentStart,
    DocumentStart,
    DocumentContent,
    DocumentEnd,
    BlockNode,
    BlockMapping,
    BlockMappingKey,
    BlockMappingValue,
    BlockSequence,
    FlowMapping,
    FlowMappingKey,
    FlowMappingValue,
    FlowSequence,
    BlockEnd,
}

impl BasicParser {
    /// Create a new streaming parser (lazy parsing)
    pub fn new(input: String) -> Self {
        Self::with_limits(input, Limits::default())
    }

    /// Create a new streaming parser with custom limits
    pub fn with_limits(input: String, limits: Limits) -> Self {
        let scanner = BasicScanner::with_limits(input, limits);
        let position = scanner.position();

        Self {
            scanner,
            events: Vec::new(),
            event_index: 0,
            state: ParserState::StreamStart,
            state_stack: Vec::new(),
            position,
            pending_anchor: None,
            pending_anchor_line: None,
            last_key_marker_line: None,
            last_key_marker_column: None,
            just_closed_inline_wrap: false,
            inline_wrap_column: None,
            pending_tag: None,
            pending_tag_line: None,
            last_token_type: None,
            scanning_error: None,
            yaml_version: None,
            tag_directives: Vec::new(),
            tag_resolver: TagResolver::new(),
            defined_anchors: std::collections::HashSet::new(),
            last_value_token_line: None,
            explicit_key_pending: false,
            implicit_flow_pair_depth: 0,
        }
    }

    /// Create a new parser with eager parsing (for compatibility)
    pub fn new_eager(input: String) -> Self {
        Self::new_eager_with_limits(input, Limits::default())
    }

    /// Create a new parser with eager parsing and custom limits
    pub fn new_eager_with_limits(input: String, limits: Limits) -> Self {
        let mut scanner = BasicScanner::new_eager_with_limits(input, limits);
        let position = scanner.position();

        // Check if there were any scanning errors and store them
        let scanning_error = scanner.take_scanning_error();

        let mut parser = Self {
            scanner,
            events: Vec::new(),
            event_index: 0,
            state: ParserState::StreamStart,
            state_stack: Vec::new(),
            position,
            pending_anchor: None,
            pending_anchor_line: None,
            last_key_marker_line: None,
            last_key_marker_column: None,
            just_closed_inline_wrap: false,
            inline_wrap_column: None,
            pending_tag: None,
            pending_tag_line: None,
            last_token_type: None,
            scanning_error: None,
            yaml_version: None,
            tag_directives: Vec::new(),
            tag_resolver: TagResolver::new(),
            defined_anchors: std::collections::HashSet::new(),
            last_value_token_line: None,
            explicit_key_pending: false,
            implicit_flow_pair_depth: 0,
        };

        // If there was a scanning error, store it for later propagation.
        // Likewise, surface eager-parse errors via the same field so
        // `take_scanning_error` reports them.
        if let Some(error) = scanning_error {
            parser.scanning_error = Some(error);
        } else if let Err(error) = parser.parse_all() {
            parser.scanning_error = Some(error);
        }

        parser
    }

    /// Create parser from existing scanner
    pub fn from_scanner(scanner: BasicScanner) -> Self {
        let position = scanner.position();

        let mut parser = Self {
            scanner,
            events: Vec::new(),
            event_index: 0,
            state: ParserState::StreamStart,
            state_stack: Vec::new(),
            position,
            pending_anchor: None,
            pending_anchor_line: None,
            last_key_marker_line: None,
            last_key_marker_column: None,
            just_closed_inline_wrap: false,
            inline_wrap_column: None,
            pending_tag: None,
            pending_tag_line: None,
            last_token_type: None,
            scanning_error: None,
            yaml_version: None,
            tag_directives: Vec::new(),
            tag_resolver: TagResolver::new(),
            defined_anchors: std::collections::HashSet::new(),
            last_value_token_line: None,
            explicit_key_pending: false,
            implicit_flow_pair_depth: 0,
        };

        parser.parse_all().unwrap_or(());
        parser
    }

    /// Parse all tokens into events
    fn parse_all(&mut self) -> Result<()> {
        while self.scanner.check_token() {
            let token = match self.scanner.get_token()? {
                Some(token) => token,
                None => break,
            };

            self.position = token.end_position;
            self.process_token(token)?;
        }

        // Check for unclosed structures
        self.validate_final_state()?;

        // Ensure stream end
        if !self
            .events
            .iter()
            .any(|e| matches!(e.event_type, EventType::StreamEnd))
        {
            self.events.push(Event::stream_end(self.position));
        }

        Ok(())
    }

    /// YAML 1.2 §6.8: directives may appear only before the first
    /// document (`StreamStart` / `ImplicitDocumentStart`) or after an
    /// explicit `...` (`DocumentEnd`). Anywhere else they're invalid.
    fn check_directive_context(&self, pos: Position, name: &str) -> Result<()> {
        if matches!(
            self.state,
            ParserState::StreamStart
                | ParserState::ImplicitDocumentStart
                | ParserState::DocumentEnd
        ) {
            Ok(())
        } else {
            Err(Error::parse(
                pos,
                format!("{name} directive is only allowed before a document or after `...`"),
            ))
        }
    }

    /// Create implicit document start event with directives
    fn create_implicit_document_start(&mut self, position: Position) -> Event {
        let event = Event::document_start(
            position,
            self.yaml_version.take(),
            self.tag_directives.clone(),
            true,
        );
        self.tag_directives.clear();
        event
    }

    /// Validate that the parser is in a valid final state
    fn validate_final_state(&self) -> Result<()> {
        match self.state {
            ParserState::StreamEnd | ParserState::DocumentEnd | ParserState::DocumentContent => {
                // These are valid final states
                Ok(())
            }
            ParserState::BlockSequence | ParserState::FlowSequence => {
                let context = ErrorContext::from_input(self.scanner.input(), &self.position, 2)
                    .with_suggestion(
                        "Close the sequence with proper indentation or closing bracket".to_string(),
                    );
                Err(Error::unclosed_delimiter_with_context(
                    self.position,
                    self.position,
                    "sequence",
                    context,
                ))
            }
            ParserState::BlockMapping | ParserState::FlowMapping => {
                let context = ErrorContext::from_input(self.scanner.input(), &self.position, 2)
                    .with_suggestion(
                        "Close the mapping with proper indentation or closing brace".to_string(),
                    );
                Err(Error::unclosed_delimiter_with_context(
                    self.position,
                    self.position,
                    "mapping",
                    context,
                ))
            }
            _ => {
                let context = ErrorContext::from_input(self.scanner.input(), &self.position, 2)
                    .with_suggestion("Complete the YAML document structure".to_string());
                Err(Error::parse_with_context(
                    self.position,
                    format!("Document ended in unexpected state: {:?}", self.state),
                    context,
                ))
            }
        }
    }

    /// Generate the next event by processing the next token
    fn generate_next_event(&mut self) -> Result<()> {
        if let Some(token) = self.scanner.get_token()? {
            self.position = token.end_position;
            self.process_token(token)?;
        }
        Ok(())
    }

    /// Process a single token and generate appropriate events
    #[allow(clippy::cognitive_complexity)]
    fn process_token(&mut self, token: Token) -> Result<()> {
        // Store the token type for later use without cloning
        let token_type_for_tracking = match &token.token_type {
            TokenType::Scalar(..) => Some(TokenType::Scalar(
                String::new(),
                crate::scanner::QuoteStyle::Plain,
            )),
            TokenType::BlockScalarLiteral(..) => Some(TokenType::BlockScalarLiteral(String::new())),
            TokenType::BlockScalarFolded(..) => Some(TokenType::BlockScalarFolded(String::new())),
            TokenType::Alias(..) => Some(TokenType::Alias(String::new())),
            TokenType::Anchor(..) => Some(TokenType::Anchor(String::new())),
            TokenType::Tag(..) => Some(TokenType::Tag(String::new())),
            TokenType::Comment(..) => Some(TokenType::Comment(String::new())),
            other => {
                // For simple token types without data, we can safely clone
                match other {
                    TokenType::StreamStart => Some(TokenType::StreamStart),
                    TokenType::StreamEnd => Some(TokenType::StreamEnd),
                    TokenType::DocumentStart => Some(TokenType::DocumentStart),
                    TokenType::DocumentEnd => Some(TokenType::DocumentEnd),
                    TokenType::BlockSequenceStart => Some(TokenType::BlockSequenceStart),
                    TokenType::BlockMappingStart => Some(TokenType::BlockMappingStart),
                    TokenType::BlockEnd => Some(TokenType::BlockEnd),
                    TokenType::FlowSequenceStart => Some(TokenType::FlowSequenceStart),
                    TokenType::FlowSequenceEnd => Some(TokenType::FlowSequenceEnd),
                    TokenType::FlowMappingStart => Some(TokenType::FlowMappingStart),
                    TokenType::FlowMappingEnd => Some(TokenType::FlowMappingEnd),
                    TokenType::BlockEntry => Some(TokenType::BlockEntry),
                    TokenType::FlowEntry => Some(TokenType::FlowEntry),
                    TokenType::Key => Some(TokenType::Key),
                    TokenType::Value => Some(TokenType::Value),
                    TokenType::YamlDirective(_, _) => Some(TokenType::YamlDirective(0, 0)),
                    TokenType::TagDirective(_, _) => {
                        Some(TokenType::TagDirective(String::new(), String::new()))
                    }
                    _ => None,
                }
            }
        };

        match &token.token_type {
            TokenType::StreamStart => {
                self.events.push(Event::stream_start(token.start_position));
                self.state = ParserState::ImplicitDocumentStart;
            }

            TokenType::StreamEnd => {
                // YAML 1.2 §6.8: a directive must be followed by a
                // document body. If we reach end-of-stream with pending
                // `%YAML` / `%TAG` directives and no document was ever
                // opened, that's a parse error (yaml-test-suite 9MMA, B63P).
                if matches!(
                    self.state,
                    ParserState::ImplicitDocumentStart | ParserState::StreamStart
                ) && (self.yaml_version.is_some() || !self.tag_directives.is_empty())
                {
                    return Err(Error::parse(
                        token.start_position,
                        "Directive without a document body",
                    ));
                }
                // YAML 1.2 §7.4: every `[` / `{` must be closed before
                // end-of-stream. Walk the events; an unmatched
                // FlowSequenceStart / FlowMappingStart is invalid
                // (yaml-test-suite 6JTT, 9HCY, 9MQT/01).
                if has_unclosed_flow_collection(&self.events) {
                    return Err(Error::parse(
                        token.start_position,
                        "Unclosed flow collection at end of stream",
                    ));
                }
                // YAML 1.2 §8.1.3.1: an implicit mapping key must be
                // followed by `:`. A bare scalar at a mapping position
                // with no \`:\` (and no explicit `?` marker) is invalid
                // (yaml-test-suite 7MNF).
                if matches!(self.state, ParserState::BlockMappingKey)
                    && !self.explicit_key_pending
                    && innermost_mapping_has_odd_children(&self.events)
                {
                    // §8.22 carve-out: when the unmatched "key" is
                    // actually a collection node (the inline-wrapped
                    // explicit-key from yaml-test-suite M2N8 cluster),
                    // synth an empty value rather than erroring — the
                    // explicit-key construct allows omitted values.
                    let key_was_collection = matches!(
                        self.events.last().map(|e| &e.event_type),
                        Some(EventType::MappingEnd | EventType::SequenceEnd)
                    );
                    if key_was_collection {
                        self.events.push(Event::scalar(
                            token.start_position,
                            None,
                            None,
                            String::new(),
                            true,
                            false,
                            ScalarStyle::Plain,
                        ));
                    } else {
                        return Err(Error::parse(
                            token.start_position,
                            "Mapping key not followed by `:`",
                        ));
                    }
                }
                // YAML 1.2: an explicit `---` with NO body needs an
                // implicit empty scalar as the doc's content (yaml-test-
                // suite MUS6/02). We detect that case by checking the
                // last emitted event — if it's still `DocumentStart`,
                // nothing has been pushed to the body yet.
                if matches!(
                    self.events.last().map(|e| &e.event_type),
                    Some(EventType::DocumentStart { .. })
                ) {
                    self.events.push(Event::scalar(
                        token.start_position,
                        None,
                        None,
                        String::new(),
                        true,
                        false,
                        ScalarStyle::Plain,
                    ));
                }
                // §6.9: a stand-alone anchor or tag at end-of-stream
                // produces a document with a tagged/anchored empty
                // scalar (yaml-test-suite UKK6/02 — a bare \`!\`).
                if matches!(self.state, ParserState::ImplicitDocumentStart)
                    && (self.pending_anchor.is_some() || self.pending_tag.is_some())
                {
                    let event = self.create_implicit_document_start(token.start_position);
                    self.events.push(event);
                    self.events.push(Event::scalar(
                        token.start_position,
                        self.pending_anchor.take(),
                        self.pending_tag.take(),
                        String::new(),
                        true,
                        false,
                        ScalarStyle::Plain,
                    ));
                    self.state = ParserState::DocumentContent;
                }
                // Close any open document. A document is "open" in every
                // state except: not-yet-started (StreamStart /
                // ImplicitDocumentStart), or already closed (DocumentEnd /
                // StreamEnd). If still open, also flush any unclosed
                // block collections first.
                if !matches!(
                    self.state,
                    ParserState::StreamStart
                        | ParserState::ImplicitDocumentStart
                        | ParserState::DocumentEnd
                        | ParserState::StreamEnd
                ) {
                    close_open_collections(&mut self.events, token.start_position);
                    self.events
                        .push(Event::document_end(token.start_position, true));
                }
                self.events.push(Event::stream_end(token.start_position));
                self.state = ParserState::StreamEnd;
            }

            TokenType::YamlDirective(major, minor) => {
                // YAML 1.2 §6.8: directives may appear only before the
                // first document or after an explicit `...` document end.
                self.check_directive_context(token.start_position, "%YAML")?;
                // §6.8.1: a document may have at most one `%YAML` directive.
                if self.yaml_version.is_some() {
                    return Err(Error::parse(
                        token.start_position,
                        "Multiple %YAML directives in the same document",
                    ));
                }
                self.yaml_version = Some((*major, *minor));
            }

            TokenType::TagDirective(handle, prefix) => {
                self.check_directive_context(token.start_position, "%TAG")?;
                self.tag_directives.push((handle.clone(), prefix.clone()));
                self.tag_resolver
                    .add_directive(handle.clone(), prefix.clone());
            }

            TokenType::DocumentStart => {
                // If the most-recent event is still `DocumentStart`, the
                // previous document had no body — emit an implicit empty
                // scalar before closing it (yaml-test-suite 6XDY).
                if matches!(
                    self.events.last().map(|e| &e.event_type),
                    Some(EventType::DocumentStart { .. })
                ) {
                    self.events.push(Event::scalar(
                        token.start_position,
                        None,
                        None,
                        String::new(),
                        true,
                        false,
                        ScalarStyle::Plain,
                    ));
                    self.events
                        .push(Event::document_end(token.start_position, true));
                    // §6.8: \`%TAG\` and \`%YAML\` are scoped to one document.
                    // After the implicit close, reset the tag resolver
                    // so directives from the prior doc don't leak
                    // (yaml-test-suite QLJ7).
                    self.tag_resolver = TagResolver::new();
                } else if has_open_document(&self.events) {
                    // The previous document is still open — its outer
                    // collection(s) and the document itself need closing
                    // before the new `---` (yaml-test-suite 35KP).
                    close_open_collections(&mut self.events, token.start_position);
                    self.events
                        .push(Event::document_end(token.start_position, true));
                    self.tag_resolver = TagResolver::new();
                }

                // Create document start with directives
                self.events.push(Event::document_start(
                    token.start_position,
                    self.yaml_version.take(),
                    self.tag_directives.clone(),
                    false,
                ));

                // Clear tag directives after using them (YAML version persists across documents)
                // But keep them in the tag resolver for this document
                self.tag_directives.clear();

                self.state = ParserState::DocumentStart;
            }

            TokenType::DocumentEnd => {
                // §6.8: `...` only terminates an *open* document. If
                // the stream so far has no DocumentStart (e.g. the
                // input is just `...\n`, yaml-test-suite HWV9), the
                // marker is a no-op.
                if !has_open_document(&self.events) {
                    self.state = ParserState::ImplicitDocumentStart;
                    self.last_token_type = token_type_for_tracking;
                    return Ok(());
                }
                // Same empty-doc fixup as in DocumentStart/StreamEnd:
                // `---\n...` needs an implicit empty scalar.
                if matches!(
                    self.events.last().map(|e| &e.event_type),
                    Some(EventType::DocumentStart { .. })
                ) {
                    self.events.push(Event::scalar(
                        token.start_position,
                        None,
                        None,
                        String::new(),
                        true,
                        false,
                        ScalarStyle::Plain,
                    ));
                } else {
                    // Flush any still-open block collections so the
                    // event stream is balanced before -DOC.
                    close_open_collections(&mut self.events, token.start_position);
                }
                self.events
                    .push(Event::document_end(token.start_position, false));
                // YAML 1.2: after `...`, the stream may continue with
                // either another `---`, more directives, or implicit
                // document content.
                self.state = ParserState::ImplicitDocumentStart;
            }

            TokenType::BlockSequenceStart => {
                // §3.2.1.1: reject a second root-level node
                // (yaml-test-suite BD7L: `- a\n- b\ninvalid: x`).
                if matches!(self.state, ParserState::DocumentContent)
                    && second_root_node_present(&self.events)
                {
                    return Err(Error::parse(
                        token.start_position,
                        "Document already contains a root node",
                    ));
                }
                if matches!(self.state, ParserState::ImplicitDocumentStart) {
                    let event = self.create_implicit_document_start(token.start_position);
                    self.events.push(event);
                }

                // If we're starting a sequence within a mapping or
                // outer-sequence context, push the current state so the
                // outer collection can be restored on close. Without
                // BlockSequence in this list, a nested `- -` sequence's
                // inner close falls through to DocumentContent and the
                // next BlockEntry spuriously opens a fresh sequence
                // (yaml-test-suite 3ALJ, 57H4).
                if matches!(
                    self.state,
                    ParserState::BlockMappingValue
                        | ParserState::BlockMappingKey
                        | ParserState::BlockSequence
                ) {
                    self.state_stack.push(self.state);
                }

                self.events.push(Event::sequence_start(
                    token.start_position,
                    self.pending_anchor.take(),
                    self.pending_tag.take(),
                    false,
                ));
                self.state = ParserState::BlockSequence;
            }

            TokenType::BlockMappingStart => {
                // §3.2.1.1: reject a second root-level node
                // (yaml-test-suite BD7L variants).
                if matches!(self.state, ParserState::DocumentContent)
                    && second_root_node_present(&self.events)
                {
                    return Err(Error::parse(
                        token.start_position,
                        "Document already contains a root node",
                    ));
                }
                // §9.1.1: an anchor on the \`---\` doc-start line cannot
                // be followed by an implicit single-pair mapping —
                // the anchor would have nowhere to attach (it's not
                // the mapping itself, not the key). \`--- &anchor a: b\`
                // is invalid (yaml-test-suite CXX2).
                if matches!(self.state, ParserState::DocumentStart)
                    && self.pending_anchor.is_some()
                    && self.pending_anchor_line == Some(token.start_position.line)
                {
                    return Err(Error::parse(
                        token.start_position,
                        "Anchor on `---` doc-start line cannot precede an implicit mapping",
                    ));
                }
                // Determine whether to create a new mapping or continue existing one
                // This token is generated when we encounter a key at the start of a line with nested content
                // It doesn't always mean we need to create a new mapping - sometimes we're just continuing

                let should_create_new_mapping = match self.state {
                    ParserState::ImplicitDocumentStart => {
                        // At document start, we need a new mapping
                        true
                    }
                    ParserState::DocumentStart => {
                        // After explicit document start (---), we need a new mapping
                        true
                    }
                    ParserState::DocumentContent => {
                        // This is a tricky case - we could be:
                        // 1. Starting a new root mapping
                        // 2. Continuing an existing root mapping
                        // The key is to check if we have an unclosed root mapping

                        // Count mapping depth from the end
                        let mut mapping_depth = 0;
                        let mut has_unclosed_mapping = false;

                        for event in self.events.iter().rev() {
                            match &event.event_type {
                                EventType::MappingEnd => mapping_depth += 1,
                                EventType::MappingStart { .. } => {
                                    if mapping_depth == 0 {
                                        has_unclosed_mapping = true;
                                        break;
                                    }
                                    mapping_depth -= 1;
                                }
                                EventType::DocumentStart { .. } => break,
                                _ => {}
                            }
                        }

                        // Don't create a new mapping if we have an unclosed one
                        !has_unclosed_mapping
                    }
                    ParserState::BlockMappingValue => {
                        // If we're expecting a value and see BlockMappingStart, it's a nested mapping
                        true
                    }
                    ParserState::BlockMappingKey => {
                        // We're already in a mapping key context
                        // BlockMappingStart here means we're continuing the mapping unless:
                        // - After a Key token (complex key)
                        // - After a Value token (nested mapping as value)
                        matches!(
                            &self.last_token_type,
                            Some(TokenType::Key | TokenType::Value)
                        )
                    }
                    ParserState::BlockSequence => {
                        // In a sequence context, BlockMappingStart means we're starting
                        // a nested mapping as a sequence item
                        true
                    }
                    _ => {
                        // For other states, check last token
                        matches!(
                            &self.last_token_type,
                            Some(TokenType::Key | TokenType::Value)
                        )
                    }
                };

                if should_create_new_mapping {
                    // Create a new nested mapping
                    if matches!(self.state, ParserState::ImplicitDocumentStart) {
                        let event = self.create_implicit_document_start(token.start_position);
                        self.events.push(event);
                    }

                    // If we're in a mapping value or sequence context, push state to stack
                    if matches!(
                        self.state,
                        ParserState::BlockMappingValue | ParserState::BlockSequence
                    ) {
                        self.state_stack.push(self.state);
                    }

                    // If the BlockMappingStart wraps an implicit key
                    // at the document root and the next token is the
                    // key scalar on the SAME line as the pending
                    // anchor/tag, those properties belong to the key —
                    // not to the surrounding mapping (yaml-test-suite
                    // ZH7C, E76Z, 74H7). For mappings nested in a value
                    // or sequence position, or when the anchor is
                    // "freestanding" on a previous line, the anchor
                    // attaches to the mapping itself (yaml-test-suite
                    // 6BFJ, 9KAX).
                    let in_value_position = matches!(
                        self.state,
                        ParserState::BlockMappingValue | ParserState::BlockSequence
                    );
                    let next_token_line = self
                        .scanner
                        .peek_token()
                        .ok()
                        .and_then(|t| t.map(|tt| tt.start_position.line));
                    let next_is_scalar = matches!(
                        self.scanner.peek_token(),
                        Ok(Some(t)) if matches!(
                            t.token_type,
                            TokenType::Scalar(..) | TokenType::Anchor(_) | TokenType::Tag(_)
                        )
                    );
                    let anchor_same_line_as_key = matches!(
                        (self.pending_anchor_line, next_token_line),
                        (Some(a), Some(k)) if a == k
                    );
                    let (anchor, tag) =
                        if !in_value_position && next_is_scalar && anchor_same_line_as_key {
                            (None, None)
                        } else {
                            self.pending_anchor_line = None;
                            (self.pending_anchor.take(), self.pending_tag.take())
                        };
                    self.events.push(Event::mapping_start(
                        token.start_position,
                        anchor,
                        tag,
                        false,
                    ));
                    self.state = ParserState::BlockMappingKey;
                } else {
                    // Continue existing mapping
                    // Ensure we're in the right state to handle the next key-value pair
                    match self.state {
                        ParserState::DocumentContent => {
                            // We should be continuing a mapping, so transition to BlockMappingKey
                            self.state = ParserState::BlockMappingKey;
                        }
                        ParserState::BlockMappingValue => {
                            // We just processed a value, now ready for next key
                            self.state = ParserState::BlockMappingKey;
                        }
                        ParserState::BlockMappingKey => {
                            // Already ready for next key, no state change needed
                        }
                        _ => {
                            // For other states, check if we can restore from state stack
                            if let Some(prev_state) = self.state_stack.last() {
                                if matches!(prev_state, ParserState::BlockMappingValue) {
                                    if let Some(mapping_state) = self.state_stack.pop() {
                                        self.state = mapping_state;
                                        self.handle_node_completion();
                                    }
                                }
                            }
                        }
                    }
                }
            }

            TokenType::FlowSequenceStart => {
                if matches!(self.state, ParserState::ImplicitDocumentStart) {
                    self.events.push(Event::document_start(
                        token.start_position,
                        None,
                        vec![],
                        true,
                    ));
                }

                // Save the enclosing state so we can restore it
                // after the flow collection closes. The save list
                // includes any state that can legitimately contain a
                // flow node — block contexts AND flow-mapping
                // key/value positions (yaml-test-suite SBG9
                // \`{a: [b,c], [d,e]: f}\` — without FlowMappingValue
                // in this list, state_stack pop'd None and we fell
                // through to DocumentContent).
                if matches!(
                    self.state,
                    ParserState::BlockMappingValue
                        | ParserState::BlockMappingKey
                        | ParserState::BlockSequence
                        | ParserState::FlowSequence
                        | ParserState::FlowMapping
                        | ParserState::FlowMappingKey
                        | ParserState::FlowMappingValue
                ) {
                    self.state_stack.push(self.state);
                }

                self.events.push(Event::sequence_start(
                    token.start_position,
                    self.pending_anchor.take(),
                    self.pending_tag.take(),
                    true,
                ));
                self.state = ParserState::FlowSequence;
            }

            TokenType::FlowMappingStart => {
                if matches!(self.state, ParserState::ImplicitDocumentStart) {
                    self.events.push(Event::document_start(
                        token.start_position,
                        None,
                        vec![],
                        true,
                    ));
                }

                // Save the enclosing state so we can restore it
                // after the flow collection closes. The save list
                // includes any state that can legitimately contain a
                // flow node — block contexts AND flow-mapping
                // key/value positions (yaml-test-suite SBG9
                // \`{a: [b,c], [d,e]: f}\` — without FlowMappingValue
                // in this list, state_stack pop'd None and we fell
                // through to DocumentContent).
                if matches!(
                    self.state,
                    ParserState::BlockMappingValue
                        | ParserState::BlockMappingKey
                        | ParserState::BlockSequence
                        | ParserState::FlowSequence
                        | ParserState::FlowMapping
                        | ParserState::FlowMappingKey
                        | ParserState::FlowMappingValue
                ) {
                    self.state_stack.push(self.state);
                }

                self.events.push(Event::mapping_start(
                    token.start_position,
                    self.pending_anchor.take(),
                    self.pending_tag.take(),
                    true,
                ));
                self.state = ParserState::FlowMapping;
            }

            TokenType::FlowSequenceEnd => {
                // §7.5: close an open implicit single-pair flow mapping
                // before the outer flow sequence ends.
                if self.implicit_flow_pair_depth > 0
                    && matches!(
                        self.state,
                        ParserState::FlowMapping
                            | ParserState::FlowMappingKey
                            | ParserState::FlowMappingValue
                    )
                    && matches!(self.state_stack.last(), Some(ParserState::FlowSequence))
                {
                    if innermost_mapping_has_odd_children(&self.events) {
                        self.events.push(Event::scalar(
                            token.start_position,
                            None,
                            None,
                            String::new(),
                            true,
                            false,
                            ScalarStyle::Plain,
                        ));
                    }
                    self.events.push(Event::mapping_end(token.start_position));
                    self.state = self.state_stack.pop().ok_or_else(|| {
                        Error::parse(
                            token.start_position,
                            "internal: parser state stack underflow at implicit flow mapping end (in flow sequence)",
                        )
                    })?;
                    self.implicit_flow_pair_depth -= 1;
                }
                self.events.push(Event::sequence_end(token.start_position));

                // Restore the previous state from the stack if available
                if let Some(prev_state) = self.state_stack.pop() {
                    self.state = prev_state;
                } else {
                    self.state = ParserState::DocumentContent;
                }

                // Handle state transitions for mapping key/value processing
                self.handle_node_completion();
            }

            TokenType::FlowMappingEnd => {
                // §7.5: explicit `?` with no key (yaml-test-suite
                // DFF7 \`{... ?\n}\`) — synth empty key AND empty
                // value before closing.
                if matches!(self.state, ParserState::FlowMappingKey)
                    && self.explicit_key_pending
                    && !innermost_mapping_has_odd_children(&self.events)
                {
                    self.events.push(Event::scalar(
                        token.start_position,
                        None,
                        None,
                        String::new(),
                        true,
                        false,
                        ScalarStyle::Plain,
                    ));
                }
                // Spec §7.5: implicit empty value for a flow-mapping
                // entry that has only a key, e.g. `{ key }` or
                // `{ key, a: b }` (yaml-test-suite 8KB6).
                if innermost_mapping_has_odd_children(&self.events) {
                    self.events.push(Event::scalar(
                        token.start_position,
                        None,
                        None,
                        String::new(),
                        true,
                        false,
                        ScalarStyle::Plain,
                    ));
                }
                self.events.push(Event::mapping_end(token.start_position));

                // Restore the previous state from the stack if available
                if let Some(prev_state) = self.state_stack.pop() {
                    self.state = prev_state;
                } else {
                    self.state = ParserState::DocumentContent;
                }

                // Handle state transitions for mapping key/value processing
                self.handle_node_completion();
            }

            TokenType::BlockEnd => {
                // Determine what we're ending based on current state
                match self.state {
                    ParserState::BlockSequence => {
                        // §6.9: an anchor or tag left unused at the
                        // close of the sequence belongs to an empty
                        // scalar that is the final sequence item
                        // (yaml-test-suite LE5A: \`- !!str\` produces
                        // a tagged empty scalar before -SEQ).
                        // §6.9.1: also if the previous token was
                        // BlockEntry with no item between — the last
                        // entry was an implicit empty (yaml-test-suite
                        // SM9W cluster).
                        let last_was_block_entry =
                            matches!(self.last_token_type, Some(TokenType::BlockEntry));
                        if self.pending_anchor.is_some()
                            || self.pending_tag.is_some()
                            || last_was_block_entry
                        {
                            self.events.push(Event::scalar(
                                token.start_position,
                                self.pending_anchor.take(),
                                self.pending_tag.take(),
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                        }
                        self.events.push(Event::sequence_end(token.start_position));
                        // Pop previous state from stack if available
                        if let Some(prev_state) = self.state_stack.pop() {
                            self.state = prev_state;
                            // Handle state transitions for mapping key/value processing
                            self.handle_node_completion();
                        } else {
                            self.state = ParserState::DocumentContent;
                        }
                    }
                    ParserState::BlockMapping
                    | ParserState::BlockMappingKey
                    | ParserState::BlockMappingValue => {
                        // §6.9.1: if the innermost mapping has odd
                        // children (last key has no value), synth an
                        // implicit empty value before closing
                        // (yaml-test-suite 7W2P). If the unmatched key
                        // came from a bare scalar with no `:`
                        // (yaml-test-suite 7MNF), error instead.
                        if innermost_mapping_has_odd_children(&self.events) {
                            if matches!(self.state, ParserState::BlockMappingKey)
                                && !self.explicit_key_pending
                            {
                                // §8.22 carve-out: if the unmatched
                                // 'key' is a collection node (the
                                // inline-wrapped explicit-key from
                                // yaml-test-suite M2N8), synth empty
                                // value instead of erroring.
                                let key_was_collection = matches!(
                                    self.events.last().map(|e| &e.event_type),
                                    Some(EventType::MappingEnd | EventType::SequenceEnd)
                                );
                                if !key_was_collection {
                                    return Err(Error::parse(
                                        token.start_position,
                                        "Mapping key not followed by `:`",
                                    ));
                                }
                            }
                            // §6.9: when synthesising the missing value
                            // for the last key, consume any pending
                            // anchor/tag — they were the property of
                            // that absent value (yaml-test-suite PW8X
                            // \`b: &b\\n- ...\` — &b belongs to b's empty
                            // value, not a separate tagged scalar).
                            self.events.push(Event::scalar(
                                token.start_position,
                                self.pending_anchor.take(),
                                self.pending_tag.take(),
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                        }
                        // Flush leftover anchor/tag as a final tagged
                        // empty scalar (mirror of the BlockSequence
                        // arm). Skipped above when the missing-value
                        // synth already consumed it.
                        if self.pending_anchor.is_some() || self.pending_tag.is_some() {
                            self.events.push(Event::scalar(
                                token.start_position,
                                self.pending_anchor.take(),
                                self.pending_tag.take(),
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                        }
                        // If the pending-property flush above just
                        // emitted a KEY (leaving odd children), we
                        // still need the missing implicit empty
                        // VALUE before closing the mapping (yaml-
                        // test-suite PW8X \`? &d\` close case).
                        if innermost_mapping_has_odd_children(&self.events) {
                            self.events.push(Event::scalar(
                                token.start_position,
                                None,
                                None,
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                        }
                        self.events.push(Event::mapping_end(token.start_position));
                        // Pop previous state from stack if available
                        if let Some(prev_state) = self.state_stack.pop() {
                            self.state = prev_state;
                            // If we popped back to a mapping value state, complete it
                            if matches!(self.state, ParserState::BlockMappingValue) {
                                self.handle_node_completion();
                            }
                        } else {
                            // No state on stack - check if we're still in a root mapping
                            // Count the mapping depth including the one we just closed
                            let mut mapping_depth = 0;

                            for event in self.events.iter().rev() {
                                match &event.event_type {
                                    EventType::MappingEnd => {
                                        mapping_depth += 1;
                                    }
                                    EventType::MappingStart { .. } => {
                                        if mapping_depth > 0 {
                                            mapping_depth -= 1;
                                        } else {
                                            // Found an unclosed mapping - we're still in the root mapping
                                            self.state = ParserState::BlockMappingKey;
                                            return Ok(());
                                        }
                                    }
                                    EventType::DocumentStart { .. } => break,
                                    _ => {}
                                }
                            }

                            // All mappings are closed
                            self.state = ParserState::DocumentContent;
                        }
                    }
                    _ => {}
                }
            }

            TokenType::Scalar(value, quote_style) => {
                if matches!(self.state, ParserState::ImplicitDocumentStart) {
                    self.events.push(Event::document_start(
                        token.start_position,
                        None,
                        vec![],
                        true,
                    ));
                    self.state = ParserState::DocumentContent;
                }

                // §3.2.1.1: a document has exactly one root node.
                if matches!(self.state, ParserState::DocumentContent)
                    && second_root_node_present(&self.events)
                {
                    return Err(Error::parse(
                        token.start_position,
                        "Document already contains a root node",
                    ));
                }

                // §8.22: in BlockSequence state, every item must be
                // introduced by \`-\`. A Scalar arriving when the
                // previous token was already a scalar / block-scalar
                // / closed-flow-collection means \`- a\\n  b\` style —
                // \`b\` is bogus content at the sequence's indent
                // (yaml-test-suite 6S55).
                if matches!(self.state, ParserState::BlockSequence)
                    && matches!(
                        self.last_token_type,
                        Some(
                            TokenType::Scalar(..)
                                | TokenType::BlockScalarLiteral(..)
                                | TokenType::BlockScalarFolded(..)
                                | TokenType::FlowSequenceEnd
                                | TokenType::FlowMappingEnd
                        )
                    )
                {
                    return Err(Error::parse(
                        token.start_position,
                        "Block sequence item must start with `-`",
                    ));
                }

                // §7.4: in flow mapping/sequence between entries
                // (even children = ready for next key/item) a Scalar
                // must be preceded by a separator. If the previous
                // token was a Scalar (i.e. previous value just emitted)
                // and not a comma, this is a missing-comma error
                // (yaml-test-suite T833, CML9).
                if matches!(
                    self.state,
                    ParserState::FlowMapping | ParserState::FlowSequence
                ) && matches!(
                    self.last_token_type,
                    Some(
                        TokenType::Scalar(..)
                            | TokenType::FlowSequenceEnd
                            | TokenType::FlowMappingEnd
                    )
                ) {
                    return Err(Error::parse(
                        token.start_position,
                        "Missing `,` separator between flow collection entries",
                    ));
                }

                // Check if we're in a sequence and the next token is Value (indicating a mapping key)
                if matches!(self.state, ParserState::BlockSequence) {
                    if let Ok(Some(next_token)) = self.scanner.peek_token() {
                        if matches!(next_token.token_type, TokenType::Value) {
                            // This scalar is a mapping key within a sequence item
                            // Push current state to stack and start a new mapping
                            self.state_stack.push(self.state);
                            self.events.push(Event::mapping_start(
                                token.start_position,
                                self.pending_anchor.take(),
                                self.pending_tag.take(),
                                false,
                            ));
                            self.state = ParserState::BlockMappingKey;
                        }
                    }
                }

                // §7.5: a flow-sequence entry that is itself `key: value`
                // is an implicit single-pair flow mapping. Any
                // pending anchor/tag belongs to the KEY scalar, not
                // to the synthesised mapping (yaml-test-suite QF4Y,
                // L9U5, 87E4, 8UDB, 9MMW, LX3P, CN3R).
                //
                // §7.5 also says: an implicit key in flow context
                // must be on a SINGLE LINE. If the \`:\` is on a
                // different line from the key scalar, it's invalid
                // (yaml-test-suite DK4H, ZXT5).
                if matches!(self.state, ParserState::FlowSequence) {
                    if let Ok(Some(next_token)) = self.scanner.peek_token() {
                        if matches!(next_token.token_type, TokenType::Value) {
                            if next_token.start_position.line != token.start_position.line {
                                return Err(Error::parse(
                                    next_token.start_position,
                                    "Implicit key in flow context must be on a single line",
                                ));
                            }
                            self.state_stack.push(self.state);
                            self.events.push(Event::mapping_start(
                                token.start_position,
                                None,
                                None,
                                true,
                            ));
                            self.state = ParserState::FlowMappingKey;
                            self.implicit_flow_pair_depth += 1;
                        }
                    }
                }

                // YAML 1.2: if we're in BlockMappingValue and the next
                // token is `:` (Value), the current scalar is actually a
                // NEW KEY — the previous key's value is implicit empty
                // (yaml-test-suite 6KGN: `a: &anchor\nb: *anchor`).
                // Emit the empty value first (consuming any pending
                // anchor/tag — those were intended for the missing
                // value), then transition back to BlockMappingKey.
                //
                // BUT skip the heuristic when:
                //   * the most recent event was an implicit empty scalar
                //     (we just synthesised an empty key for a leading-`:`
                //     mapping, yaml-test-suite 2JQS), or
                //   * the current scalar is on the SAME line as the
                //     previous `:` Value token — that puts the scalar
                //     in the value slot of the current key
                //     (yaml-test-suite 6M2F: `? &a a\n: &b b\n: *a`).
                // §8.22 V9D5: when we JUST closed an inline-wrapped
                // explicit key, the value position can also hold an
                // inline single-pair mapping. If the next token is
                // \`:\` (a key/value separator on this scalar's line),
                // wrap the scalar as the key of an inner mapping.
                if matches!(self.state, ParserState::BlockMappingValue)
                    && self.just_closed_inline_wrap
                {
                    self.just_closed_inline_wrap = false;
                    if let Ok(Some(next_token)) = self.scanner.peek_token() {
                        if matches!(next_token.token_type, TokenType::Value)
                            && next_token.start_position.line == token.start_position.line
                        {
                            self.state_stack.push(self.state);
                            self.events.push(Event::mapping_start(
                                token.start_position,
                                None,
                                None,
                                false,
                            ));
                            self.state = ParserState::BlockMappingKey;
                            // Fall through to the normal Scalar push;
                            // the scalar will become the inner key.
                        }
                    }
                }

                if matches!(self.state, ParserState::BlockMappingValue) {
                    let last_was_implicit_empty = matches!(self.events.last(), Some(ev) if matches!(
                        &ev.event_type,
                        EventType::Scalar { value, plain_implicit: true, style: ScalarStyle::Plain, .. }
                            if value.is_empty()
                    ));
                    let same_line_as_value = self
                        .last_value_token_line
                        .map_or(false, |line| line == token.start_position.line);
                    // Skip the "new key" pattern when the scalar IS
                    // the inline value of a just-synthesised empty
                    // key — both must hold (yaml-test-suite 2JQS).
                    // S3PD shows it must NOT skip when the empty
                    // key was on a different line from the current
                    // scalar.
                    let skip_pattern = last_was_implicit_empty && same_line_as_value;
                    if !skip_pattern && !same_line_as_value {
                        if let Ok(Some(next_token)) = self.scanner.peek_token() {
                            if matches!(next_token.token_type, TokenType::Value) {
                                self.events.push(Event::scalar(
                                    token.start_position,
                                    self.pending_anchor.take(),
                                    self.pending_tag.take(),
                                    String::new(),
                                    true,
                                    false,
                                    ScalarStyle::Plain,
                                ));
                                self.state = ParserState::BlockMappingKey;
                            }
                        }
                    }
                }

                // YAML 1.2 §6.9.1: if we're back at a key position but the
                // previous key still owes a value (odd children in the
                // active mapping), synthesise the implicit empty scalar
                // now — this scalar then becomes the next key
                // (yaml-test-suite 7W2P: `? a\n? b\nc:`).
                if matches!(self.state, ParserState::BlockMappingKey)
                    && innermost_mapping_has_odd_children(&self.events)
                {
                    self.events.push(Event::scalar(
                        token.start_position,
                        None,
                        None,
                        String::new(),
                        true,
                        false,
                        ScalarStyle::Plain,
                    ));
                }

                // Convert QuoteStyle to ScalarStyle
                let style = match quote_style {
                    crate::scanner::QuoteStyle::Plain => ScalarStyle::Plain,
                    crate::scanner::QuoteStyle::Single => ScalarStyle::SingleQuoted,
                    crate::scanner::QuoteStyle::Double => ScalarStyle::DoubleQuoted,
                };

                self.events.push(Event::scalar(
                    token.start_position,
                    self.pending_anchor.take(), // Use pending anchor
                    self.pending_tag.take(),    // Use pending tag
                    value.clone(),
                    style == ScalarStyle::Plain,
                    style != ScalarStyle::Plain,
                    style,
                ));

                // Handle state transitions for mapping key/value processing
                self.handle_node_completion();
            }

            TokenType::BlockScalarLiteral(value) => {
                if matches!(self.state, ParserState::ImplicitDocumentStart) {
                    self.events.push(Event::document_start(
                        token.start_position,
                        None,
                        vec![],
                        true,
                    ));
                    self.state = ParserState::DocumentContent;
                }

                self.events.push(Event::scalar(
                    token.start_position,
                    self.pending_anchor.take(), // Use pending anchor
                    self.pending_tag.take(),    // Use pending tag
                    value.clone(),
                    false, // Not plain
                    true,  // Quoted style
                    ScalarStyle::Literal,
                ));

                // Handle state transitions for mapping key/value processing
                self.handle_node_completion();
            }

            TokenType::BlockScalarFolded(value) => {
                if matches!(self.state, ParserState::ImplicitDocumentStart) {
                    self.events.push(Event::document_start(
                        token.start_position,
                        None,
                        vec![],
                        true,
                    ));
                    self.state = ParserState::DocumentContent;
                }

                self.events.push(Event::scalar(
                    token.start_position,
                    self.pending_anchor.take(), // Use pending anchor
                    self.pending_tag.take(),    // Use pending tag
                    value.clone(),
                    false, // Not plain
                    true,  // Quoted style
                    ScalarStyle::Folded,
                ));

                // Handle state transitions for mapping key/value processing
                self.handle_node_completion();
            }

            TokenType::BlockEntry => {
                // Block sequence entry - this indicates a new item in a sequence
                // We need to ensure proper state management for nested structures
                match self.state {
                    ParserState::BlockSequence => {
                        // We're already in a sequence, this is a new item.
                        // §6.9.1: if the previous token was also a
                        // BlockEntry, the previous item had no value —
                        // synthesise an implicit empty scalar before
                        // accepting this new BlockEntry (yaml-test-suite
                        // SM9W cluster). Also synth when a pending
                        // anchor/tag was left on a PREVIOUS line — the
                        // property was the previous item's empty value
                        // (yaml-test-suite PW8X).
                        let last_was_block_entry =
                            matches!(self.last_token_type, Some(TokenType::BlockEntry));
                        let earliest_property_line =
                            match (self.pending_anchor_line, self.pending_tag_line) {
                                (Some(a), Some(t)) => Some(a.min(t)),
                                (Some(a), None) => Some(a),
                                (None, Some(t)) => Some(t),
                                (None, None) => None,
                            };
                        let property_from_prev_line = (self.pending_anchor.is_some()
                            || self.pending_tag.is_some())
                            && earliest_property_line
                                .map_or(false, |a| a < token.start_position.line);
                        if last_was_block_entry || property_from_prev_line {
                            self.events.push(Event::scalar(
                                token.start_position,
                                self.pending_anchor.take(),
                                self.pending_tag.take(),
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                            self.pending_anchor_line = None;
                            self.pending_tag_line = None;
                        }
                    }
                    ParserState::BlockMapping | ParserState::BlockMappingValue => {
                        // If we encounter a BlockEntry while in a mapping,
                        // we need to close the mapping and continue the sequence
                        self.events.push(Event::mapping_end(token.start_position));
                        self.state = ParserState::BlockSequence;
                    }
                    _ => {
                        // BlockEntry in other contexts might indicate we need to start a sequence
                        // This handles implicit sequence starts
                        if matches!(self.state, ParserState::ImplicitDocumentStart) {
                            self.events.push(Event::document_start(
                                token.start_position,
                                None,
                                vec![],
                                true,
                            ));
                        }

                        // Start an implicit sequence if we're not already in one
                        self.events.push(Event::sequence_start(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            false,
                        ));
                        self.state = ParserState::BlockSequence;
                    }
                }
            }

            TokenType::Value => {
                // Snapshot the PREVIOUS Value token's line BEFORE
                // updating to the current one. The match arms below
                // need this to detect multi-`:`-on-same-line (yaml-
                // test-suite ZL4Z, ZCZ6).
                let prev_value_line = self.last_value_token_line;
                self.last_value_token_line = Some(token.start_position.line);
                self.explicit_key_pending = false;
                // YAML 1.2 §6.9.1: a `:` with no preceding key implies an
                // empty key. Handle the four states where this can arise:
                //   * ImplicitDocumentStart — open `+DOC`, `+MAP`, empty key.
                //   * DocumentContent — open `+MAP`, empty key.
                //   * BlockMappingKey with EVEN children — empty key for
                //     the next entry (no scalar preceded the `:`).
                //   * Normal cases (`BlockMappingKey` with odd children,
                //     `FlowMappingKey`) — just transition state.
                match self.state {
                    ParserState::ImplicitDocumentStart => {
                        let event = self.create_implicit_document_start(token.start_position);
                        self.events.push(event);
                        // The mapping itself has no anchor/tag here —
                        // those (if any) belong to the (empty) key.
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            None,
                            None,
                            false,
                        ));
                        self.events.push(Event::scalar(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            String::new(),
                            true,
                            false,
                            ScalarStyle::Plain,
                        ));
                        self.state = ParserState::BlockMappingValue;
                    }
                    ParserState::DocumentContent | ParserState::DocumentStart => {
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            None,
                            None,
                            false,
                        ));
                        self.events.push(Event::scalar(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            String::new(),
                            true,
                            false,
                            ScalarStyle::Plain,
                        ));
                        self.state = ParserState::BlockMappingValue;
                    }
                    ParserState::BlockMappingKey => {
                        // §8.22 V9D5: when a `:` arrives at the same
                        // column as the most recent `?` on a LATER
                        // line, it's the explicit value separator of
                        // that `?` key. If we previously wrapped an
                        // inline single-pair mapping for the explicit
                        // key (via the path below), close it first so
                        // the outer mapping receives the value (yaml-
                        // test-suite V9D5).
                        // Use inline_wrap_column (set when we opened a
                        // V9D5-style inline wrap) for matching the
                        // close. Don't depend on last_key_marker_*
                        // since those get reset on wrap open.
                        if self
                            .inline_wrap_column
                            .map_or(false, |c| c == token.start_position.column)
                            && !self.state_stack.is_empty()
                            && matches!(self.state_stack.last(), Some(ParserState::BlockMappingKey))
                        {
                            // Close inline-wrapped key mapping if its
                            // children are even (complete pairs).
                            if !innermost_mapping_has_odd_children(&self.events) {
                                self.events.push(Event::mapping_end(token.start_position));
                                self.state = self.state_stack.pop().ok_or_else(|| {
                                    Error::parse(
                                        token.start_position,
                                        "internal: parser state stack underflow closing inline-wrapped key mapping",
                                    )
                                })?;
                                self.inline_wrap_column = None;
                                self.just_closed_inline_wrap = true;
                            }
                        }
                        // §8.22: when the explicit key marker (\`?\`) is
                        // followed by a node + \`:\` on the SAME line,
                        // that whole construct is an inline single-pair
                        // mapping (the explicit key node itself).
                        // Wrap retroactively by inserting an inner
                        // MappingStart before the just-emitted key
                        // node. yaml-test-suite M2N8/01 \`? []: x\`,
                        // and the empty-prefix variant M2N8/00
                        // \`- ? : x\`.
                        let odd_children = innermost_mapping_has_odd_children(&self.events);
                        let key_marker_same_line = self
                            .last_key_marker_line
                            .map_or(false, |l| l == token.start_position.line);
                        // Empty-prefix variant: `?` then `:` directly
                        // (no node between). Open inner mapping with
                        // empty key and transition to inner value.
                        if !odd_children && key_marker_same_line {
                            self.state_stack.push(self.state);
                            self.events.push(Event::mapping_start(
                                token.start_position,
                                None,
                                None,
                                false,
                            ));
                            self.events.push(Event::scalar(
                                token.start_position,
                                None,
                                None,
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                            self.last_key_marker_line = None;
                            self.state = ParserState::BlockMappingValue;
                            self.last_token_type = token_type_for_tracking;
                            return Ok(());
                        }
                        if odd_children && key_marker_same_line {
                            // Find the most recent emitted KEY-position
                            // node within the active mapping (it'll be
                            // either a Scalar or a flow-collection
                            // open). Insert MappingStart BEFORE it.
                            let mut depth = 0i32;
                            let mut insert_at = None;
                            for (idx, ev) in self.events.iter().enumerate().rev() {
                                match &ev.event_type {
                                    EventType::MappingEnd | EventType::SequenceEnd => {
                                        depth += 1;
                                    }
                                    EventType::MappingStart { .. }
                                    | EventType::SequenceStart { .. } => {
                                        if depth == 0 {
                                            insert_at = Some(idx);
                                            break;
                                        }
                                        depth -= 1;
                                    }
                                    EventType::Scalar { .. } if depth == 0 => {
                                        insert_at = Some(idx);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                            if let Some(ii) = insert_at {
                                self.state_stack.push(self.state);
                                self.events.insert(
                                    ii,
                                    Event::mapping_start(
                                        self.events[ii].position,
                                        None,
                                        None,
                                        false,
                                    ),
                                );
                                // Record the wrap's "outer key column"
                                // so the matching explicit-value `:`
                                // (on a later line at the same column
                                // as the `?` marker) can close it.
                                self.inline_wrap_column = self.last_key_marker_column;
                                self.last_key_marker_line = None;
                                self.state = ParserState::BlockMappingValue;
                                self.last_token_type = token_type_for_tracking;
                                return Ok(());
                            }
                        }
                        let even_children = !innermost_mapping_has_odd_children(&self.events);
                        if even_children {
                            // §8.22: two implicit \`:\` on the same line
                            // in a block mapping (e.g. \`a: 'b': c\`) is
                            // invalid — block mappings cannot express
                            // nested implicit single-pair mappings
                            // inline (yaml-test-suite ZL4Z, ZCZ6).
                            //
                            // Carve-out: when the PREVIOUS `:` on this
                            // line was an explicit value separator
                            // (paired with `?`), the value position
                            // legitimately holds an inline mapping
                            // (yaml-test-suite V9D5 \`: moon: white\`
                            // after \`? earth: blue\`). We detect this
                            // by checking whether the synth'd empty key
                            // (or any structural emission) happened on
                            // THIS line — if so, allow.
                            let prev_was_scalar = matches!(
                                self.last_token_type,
                                Some(
                                    TokenType::Scalar(..)
                                        | TokenType::BlockScalarLiteral(..)
                                        | TokenType::BlockScalarFolded(..)
                                )
                            );
                            let same_line_as_prev_colon = prev_value_line
                                .map_or(false, |line| line == token.start_position.line);
                            // Walk back from the most recent event:
                            // if the last scalar BEFORE the just-pushed
                            // scalar is an EMPTY implicit scalar on
                            // this line (the synth'd empty key from a
                            // prior `:`), the prior `:` was structural
                            // and this `:` is the inline mapping's
                            // separator.
                            let mut saw_synth_empty_on_this_line = false;
                            let mut seen_value = 0;
                            for ev in self.events.iter().rev() {
                                if let EventType::Scalar {
                                    value,
                                    plain_implicit,
                                    ..
                                } = &ev.event_type
                                {
                                    if seen_value >= 1 {
                                        if value.is_empty()
                                            && *plain_implicit
                                            && ev.position.line == token.start_position.line
                                        {
                                            saw_synth_empty_on_this_line = true;
                                        }
                                        break;
                                    }
                                    seen_value += 1;
                                }
                            }
                            if prev_was_scalar
                                && same_line_as_prev_colon
                                && !saw_synth_empty_on_this_line
                            {
                                return Err(Error::parse(
                                    token.start_position,
                                    "Multiple `:` on the same line in block mapping",
                                ));
                            }
                            // Missing key — synthesise empty scalar
                            // first. Pending anchor/tag belongs to that
                            // empty key (yaml-test-suite PW8X).
                            self.events.push(Event::scalar(
                                token.start_position,
                                self.pending_anchor.take(),
                                self.pending_tag.take(),
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                        }
                        self.state = ParserState::BlockMappingValue;
                    }
                    ParserState::FlowMappingKey => {
                        self.state = ParserState::FlowMappingValue;
                    }
                    ParserState::BlockSequence
                        if matches!(self.last_token_type, Some(TokenType::BlockEntry)) =>
                    {
                        // §8.22: \`- :\` — the sequence item is a
                        // mapping with an implicit empty key and the
                        // `:` is the key/value separator (yaml-test-
                        // suite UKK6/00).
                        self.state_stack.push(self.state);
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            false,
                        ));
                        self.events.push(Event::scalar(
                            token.start_position,
                            None,
                            None,
                            String::new(),
                            true,
                            false,
                            ScalarStyle::Plain,
                        ));
                        self.state = ParserState::BlockMappingValue;
                    }
                    ParserState::FlowMapping => {
                        // §7.5: in FlowMapping state, a `:` separates
                        // an emitted key from its value (odd children
                        // means the key scalar is already on the
                        // stack — normal). If children are even,
                        // we're starting a new entry with an empty
                        // key. The pending anchor/tag (if any) belongs
                        // to that empty key (yaml-test-suite NKF9,
                        // WZ62: \`!!str : bar\` — empty key tagged
                        // !!str).
                        if !innermost_mapping_has_odd_children(&self.events) {
                            self.events.push(Event::scalar(
                                token.start_position,
                                self.pending_anchor.take(),
                                self.pending_tag.take(),
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                        }
                        self.state = ParserState::FlowMappingValue;
                    }
                    ParserState::FlowSequence => {
                        // §7.5: \`[ {k:v}:value ]\` — a closed flow
                        // collection followed by \`:\` makes that flow
                        // node the implicit key. Retroactively wrap
                        // it in an implicit single-pair mapping by
                        // inserting MappingStart BEFORE the matching
                        // flow-open event (yaml-test-suite 9MMW).
                        let last_is_flow_close = matches!(
                            self.events.last().map(|e| &e.event_type),
                            Some(EventType::MappingEnd | EventType::SequenceEnd)
                        );
                        if last_is_flow_close {
                            // Find the matching open via depth walk.
                            let mut depth = 0i32;
                            let mut open_idx = None;
                            for (idx, ev) in self.events.iter().enumerate().rev() {
                                match &ev.event_type {
                                    EventType::MappingEnd | EventType::SequenceEnd => {
                                        depth += 1;
                                    }
                                    EventType::MappingStart {
                                        flow_style: true, ..
                                    }
                                    | EventType::SequenceStart {
                                        flow_style: true, ..
                                    } => {
                                        depth -= 1;
                                        if depth == 0 {
                                            open_idx = Some(idx);
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if let Some(oi) = open_idx {
                                self.state_stack.push(self.state);
                                self.events.insert(
                                    oi,
                                    Event::mapping_start(
                                        self.events[oi].position,
                                        None,
                                        None,
                                        true,
                                    ),
                                );
                                self.state = ParserState::FlowMappingValue;
                                self.implicit_flow_pair_depth += 1;
                                self.last_token_type = token_type_for_tracking;
                                return Ok(());
                            }
                        }
                        // §7.5: `[ : value ]` — leading `:` with no
                        // preceding scalar implies an empty key for an
                        // implicit single-pair flow mapping
                        // (yaml-test-suite CFD4).
                        self.state_stack.push(self.state);
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            None,
                            None,
                            true,
                        ));
                        self.events.push(Event::scalar(
                            token.start_position,
                            None,
                            None,
                            String::new(),
                            true,
                            false,
                            ScalarStyle::Plain,
                        ));
                        self.state = ParserState::FlowMappingValue;
                        self.implicit_flow_pair_depth += 1;
                    }
                    _ => {}
                }
            }

            TokenType::FlowEntry => {
                // YAML 1.2 §7.4: a `,` must follow an entry. Leading
                // `,` (e.g. `[ , a, b ]`) and consecutive `,, ` are
                // invalid (yaml-test-suite 9MAG, CTN5).
                let no_prior_entry = matches!(
                    self.events.last().map(|e| &e.event_type),
                    Some(
                        EventType::SequenceStart {
                            flow_style: true,
                            ..
                        } | EventType::MappingStart {
                            flow_style: true,
                            ..
                        }
                    )
                );
                if no_prior_entry {
                    return Err(Error::parse(
                        token.start_position,
                        "Flow entry separator `,` with no preceding entry",
                    ));
                }
                // Consecutive `,` — last_token_type carries the kind of
                // the previous token. If it's also FlowEntry, no entry
                // came between (e.g. `[a, , b]`, `[a, b, , ]`).
                if matches!(self.last_token_type, Some(TokenType::FlowEntry)) {
                    return Err(Error::parse(
                        token.start_position,
                        "Consecutive `,` separators in flow collection",
                    ));
                }
                // §7.5: inside a flow mapping, a comma terminates the
                // current entry. If the entry is missing its value
                // (state FlowMappingValue or odd children), synth an
                // implicit empty scalar — consuming any pending
                // anchor/tag, which would have been a property of
                // the missing value (yaml-test-suite 8KB6, 9BXH,
                // FRK4, WZ62).
                if matches!(
                    self.state,
                    ParserState::FlowMapping
                        | ParserState::FlowMappingKey
                        | ParserState::FlowMappingValue
                ) && innermost_mapping_has_odd_children(&self.events)
                {
                    self.events.push(Event::scalar(
                        token.start_position,
                        self.pending_anchor.take(),
                        self.pending_tag.take(),
                        String::new(),
                        true,
                        false,
                        ScalarStyle::Plain,
                    ));
                    self.state = ParserState::FlowMapping;
                }

                // §7.5: same close-on-comma logic for implicit
                // single-pair mappings.
                if self.implicit_flow_pair_depth > 0
                    && matches!(
                        self.state,
                        ParserState::FlowMapping
                            | ParserState::FlowMappingKey
                            | ParserState::FlowMappingValue
                    )
                    && matches!(self.state_stack.last(), Some(ParserState::FlowSequence))
                {
                    if innermost_mapping_has_odd_children(&self.events) {
                        self.events.push(Event::scalar(
                            token.start_position,
                            None,
                            None,
                            String::new(),
                            true,
                            false,
                            ScalarStyle::Plain,
                        ));
                    }
                    self.events.push(Event::mapping_end(token.start_position));
                    self.state = self.state_stack.pop().ok_or_else(|| {
                        Error::parse(
                            token.start_position,
                            "internal: parser state stack underflow at implicit flow mapping end (in flow mapping)",
                        )
                    })?;
                    self.implicit_flow_pair_depth -= 1;
                }
            }

            TokenType::Anchor(name) => {
                // YAML 1.2 §6.9.2: a node may have at most one anchor.
                // A second anchor before the node is consumed is invalid
                // (yaml-test-suite 4JVG).
                if self.pending_anchor.is_some() {
                    return Err(Error::parse(
                        token.start_position,
                        "Node may not have more than one anchor",
                    ));
                }
                // Record the anchor name so subsequent aliases can be
                // validated against it (YAML 1.2 §6.9.2 forbids forward
                // references).
                self.defined_anchors.insert(name.clone());
                self.pending_anchor = Some(name.clone());
                self.pending_anchor_line = Some(token.start_position.line);
            }

            TokenType::Alias(name) => {
                // YAML 1.2 §6.9.2: alias must reference a previously
                // defined anchor — forward references are invalid.
                if !self.defined_anchors.contains(name.as_str()) {
                    return Err(Error::parse(
                        token.start_position,
                        format!("Alias `*{name}` references an undefined anchor"),
                    ));
                }
                // §6.9.2: an alias is a reference, not an independent
                // node — it cannot carry an anchor or tag of its own
                // (yaml-test-suite SR86, SU74).
                if self.pending_anchor.is_some() || self.pending_tag.is_some() {
                    return Err(Error::parse(
                        token.start_position,
                        "Alias may not have an anchor or tag",
                    ));
                }
                if matches!(self.state, ParserState::ImplicitDocumentStart) {
                    self.events.push(Event::document_start(
                        token.start_position,
                        None,
                        vec![],
                        true,
                    ));
                    self.state = ParserState::DocumentContent;
                }

                // Generate alias event
                self.events
                    .push(Event::alias(token.start_position, name.clone()));

                // Handle state transitions for mapping key/value processing
                self.handle_node_completion();
            }

            TokenType::Tag(tag) => {
                // YAML 1.2 §6.9.1 allows at most one tag per node, but
                // (like the double-anchor check) detecting that at this
                // layer produces false positives — a tag preceding an
                // implicit empty node in a sequence is followed by the
                // tag of the next sibling node, and the same `pending_tag`
                // field is reused. Until the parser tracks per-node tag
                // scopes, accept the overwrite silently (yaml-test-suite
                // FH7J relies on this).
                // Resolve and normalize the tag before storing.
                // §6.8: an unresolvable named-handle tag (e.g. `!prefix!X`
                // when no `%TAG !prefix!` directive is in scope) is
                // invalid (yaml-test-suite QLJ7).
                match self.tag_resolver.resolve(&tag) {
                    Ok(resolved_tag) => {
                        self.pending_tag = Some(resolved_tag.uri);
                        self.pending_tag_line = Some(token.start_position.line);
                    }
                    Err(e) => {
                        // Only error on named-handle tags (`!name!suffix`),
                        // not bare-tag fallback paths.
                        let is_named_handle = tag.starts_with('!')
                            && tag[1..].contains('!')
                            && !tag.starts_with("!!");
                        if is_named_handle {
                            return Err(Error::parse(
                                token.start_position,
                                format!("Undefined tag handle in `{tag}`: {e}"),
                            ));
                        }
                        self.pending_tag = Some(tag.clone());
                        self.pending_tag_line = Some(token.start_position.line);
                    }
                }
            }

            // TODO: Implement these when we add support for advanced features
            TokenType::Comment(_) => {
                // Not implemented in basic version
            }

            // Complex key marker
            TokenType::Key => {
                self.explicit_key_pending = true;
                self.last_key_marker_line = Some(token.start_position.line);
                self.last_key_marker_column = Some(token.start_position.column);
                match self.state {
                    ParserState::ImplicitDocumentStart => {
                        // Start implicit document and mapping
                        let event = self.create_implicit_document_start(token.start_position);
                        self.events.push(event);
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            false,
                        ));
                        self.state = ParserState::BlockMappingKey;
                    }
                    ParserState::DocumentStart => {
                        // Explicit document start (`---`) followed by a
                        // complex-key marker — open the document body as
                        // an implicit block mapping (yaml-test-suite 2XXW).
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            false,
                        ));
                        self.state = ParserState::BlockMappingKey;
                    }
                    ParserState::DocumentContent => {
                        // Check if we just finished a mapping - if so, continue it instead of starting new one
                        // This happens when the previous mapping key-value pair was processed but no BlockEnd was generated
                        if !self.events.is_empty() {
                            if let Some(last_event) = self.events.last() {
                                // If the last event was a scalar and we have a MappingStart before it,
                                // we're probably continuing an existing mapping
                                if matches!(last_event.event_type, EventType::Scalar { .. }) {
                                    // Look for a recent MappingStart without a corresponding MappingEnd
                                    let mut mapping_depth = 0;
                                    let mut has_unfinished_mapping = false;

                                    for event in self.events.iter().rev() {
                                        match &event.event_type {
                                            EventType::MappingEnd => mapping_depth += 1,
                                            EventType::MappingStart { .. } => {
                                                if mapping_depth == 0 {
                                                    has_unfinished_mapping = true;
                                                    break;
                                                }
                                                mapping_depth -= 1;
                                            }
                                            _ => {}
                                        }
                                    }

                                    if has_unfinished_mapping {
                                        // Continue the existing mapping instead of starting a new one
                                        self.state = ParserState::BlockMappingKey;
                                        return Ok(());
                                    }
                                }
                            }
                        }

                        // Start new mapping
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            false,
                        ));
                        self.state = ParserState::BlockMappingKey;
                    }
                    ParserState::BlockMapping | ParserState::FlowMapping => {
                        // Already in a mapping, now we have a complex key
                        self.state = if matches!(self.state, ParserState::BlockMapping) {
                            ParserState::BlockMappingKey
                        } else {
                            ParserState::FlowMappingKey
                        };
                    }
                    ParserState::FlowSequence => {
                        // §7.5: `[? key : value, ...]` — the `?`
                        // opens an implicit single-pair flow mapping
                        // with an explicit complex key (yaml-test-
                        // suite CT4Q).
                        self.state_stack.push(self.state);
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            None,
                            None,
                            true,
                        ));
                        self.state = ParserState::FlowMappingKey;
                        self.implicit_flow_pair_depth += 1;
                    }
                    ParserState::BlockSequence => {
                        // §8.22: `- ? key : value` — the sequence
                        // item is itself a block mapping with an
                        // explicit complex key. Open the wrapping
                        // mapping before the explicit key marker is
                        // consumed (yaml-test-suite M2N8/00, V9D5,
                        // KK5P, PW8X).
                        self.state_stack.push(self.state);
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            false,
                        ));
                        self.state = ParserState::BlockMappingKey;
                    }
                    ParserState::BlockMappingValue => {
                        // §8.22: \`outer:\\n  ? complex\` — the outer
                        // mapping's value is itself a block mapping
                        // whose first key is a complex key. Open the
                        // value mapping and transition to BlockMappingKey
                        // (yaml-test-suite KK5P).
                        self.state_stack.push(self.state);
                        self.events.push(Event::mapping_start(
                            token.start_position,
                            self.pending_anchor.take(),
                            self.pending_tag.take(),
                            false,
                        ));
                        self.state = ParserState::BlockMappingKey;
                    }
                    ParserState::BlockMappingKey | ParserState::FlowMappingKey => {
                        // A new `?` while we still owe a value for the
                        // previous key — synthesise an implicit empty
                        // scalar so the mapping stays balanced
                        // (yaml-test-suite 7W2P).
                        if innermost_mapping_has_odd_children(&self.events) {
                            self.events.push(Event::scalar(
                                token.start_position,
                                None,
                                None,
                                String::new(),
                                true,
                                false,
                                ScalarStyle::Plain,
                            ));
                        }
                    }
                    _ => {
                        let context =
                            ErrorContext::from_input(self.scanner.input(), &self.position, 2)
                                .with_suggestion(
                                    "Complex keys must be used in mapping contexts".to_string(),
                                );
                        return Err(Error::parse_with_context(
                            self.position,
                            "Complex key marker (?) in invalid context",
                            context,
                        ));
                    }
                }
            }
        }

        // Update the last token type for next iteration
        self.last_token_type = token_type_for_tracking;

        Ok(())
    }

    /// Handle completion of a node (scalar or collection) and manage mapping state transitions
    #[allow(clippy::missing_const_for_fn)]
    fn handle_node_completion(&mut self) {
        match self.state {
            ParserState::BlockMappingKey => {
                // After processing a key, we stay in BlockMappingKey state
                // The Value token (:) will transition us to BlockMappingValue
                // No state change needed here
            }
            ParserState::FlowMappingKey => {
                // After processing a key in flow mapping, we stay in FlowMappingKey state
                // The Value token (:) will transition us to FlowMappingValue
                // No state change needed here
            }
            ParserState::BlockMappingValue => {
                // After processing a value, we go back to waiting for the next key
                self.state = ParserState::BlockMappingKey;
            }
            ParserState::FlowMappingValue => {
                // After processing a value in flow mapping, we go back to waiting for the next key
                self.state = ParserState::FlowMapping;
            }
            _ => {
                // No state change needed for other states
            }
        }
    }
}

impl Default for BasicParser {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Parser for BasicParser {
    fn check_event(&self) -> bool {
        // For streaming: check if we have cached events, can generate
        // more, or a deferred error is waiting to be surfaced (from
        // eager parsing).
        self.event_index < self.events.len()
            || self.scanner.check_token()
            || self.scanning_error.is_some()
    }

    fn peek_event(&self) -> Result<Option<&Event>> {
        // Peek at cached events only (don't generate new ones)
        Ok(self.events.get(self.event_index))
    }

    fn get_event(&mut self) -> Result<Option<Event>> {
        // Generate next events until we have one available
        // Some tokens (like directives) don't generate events
        while self.event_index >= self.events.len() && self.scanner.check_token() {
            let events_before = self.events.len();
            self.generate_next_event()?;

            // If no event was generated and we still have tokens, continue
            if self.events.len() == events_before && self.scanner.check_token() {
                continue;
            }
            break;
        }

        if self.event_index < self.events.len() {
            let event = self.events[self.event_index].clone();
            self.event_index += 1;
            Ok(Some(event))
        } else if let Some(error) = self.scanning_error.take() {
            // Eager-parse and scanner errors are stored in
            // `scanning_error` (see `new_eager`). Surface them through
            // the natural iteration path *after* all buffered events
            // have been drained, so callers see the partial events
            // first and then the error that terminated parsing.
            Err(error)
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) {
        self.event_index = 0;
        self.scanner.reset();
        self.state_stack.clear();
        self.position = Position::start();
        self.pending_anchor = None;
        self.pending_tag = None;
        self.last_token_type = None;
    }

    fn position(&self) -> Position {
        self.position
    }
}

impl BasicParser {
    /// Check if there was a scanning error
    #[allow(clippy::missing_const_for_fn)]
    pub fn take_scanning_error(&mut self) -> Option<Error> {
        self.scanning_error.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let mut parser = BasicParser::new_eager("42".to_string());

        assert!(parser.check_event());

        // Stream start
        let event = parser.get_event().unwrap().unwrap();
        assert!(matches!(event.event_type, EventType::StreamStart));

        // Document start (implicit)
        let event = parser.get_event().unwrap().unwrap();
        if let EventType::DocumentStart { implicit, .. } = event.event_type {
            assert!(implicit);
        } else {
            panic!("Expected implicit document start");
        }

        // Scalar
        let event = parser.get_event().unwrap().unwrap();
        if let EventType::Scalar { value, .. } = event.event_type {
            assert_eq!(value, "42");
        } else {
            panic!("Expected scalar event");
        }

        // Document end (implicit)
        let event = parser.get_event().unwrap().unwrap();
        if let EventType::DocumentEnd { implicit } = event.event_type {
            assert!(implicit);
        } else {
            panic!("Expected implicit document end");
        }

        // Stream end
        let event = parser.get_event().unwrap().unwrap();
        assert!(matches!(event.event_type, EventType::StreamEnd));
    }

    #[test]
    fn test_flow_sequence_parsing() {
        let mut parser = BasicParser::new_eager("[1, 2, 3]".to_string());

        // Stream start
        parser.get_event().unwrap();

        // Document start (implicit)
        parser.get_event().unwrap();

        // Sequence start
        let event = parser.get_event().unwrap().unwrap();
        if let EventType::SequenceStart { flow_style, .. } = event.event_type {
            assert!(flow_style);
        } else {
            panic!("Expected flow sequence start");
        }

        // First scalar
        let event = parser.get_event().unwrap().unwrap();
        if let EventType::Scalar { value, .. } = event.event_type {
            assert_eq!(value, "1");
        } else {
            panic!("Expected scalar '1'");
        }
    }

    #[test]
    fn test_flow_mapping_parsing() {
        let mut parser = BasicParser::new_eager("{'key': 'value'}".to_string());

        // Stream start
        parser.get_event().unwrap();

        // Document start (implicit)
        parser.get_event().unwrap();

        // Mapping start
        let event = parser.get_event().unwrap().unwrap();
        if let EventType::MappingStart { flow_style, .. } = event.event_type {
            assert!(flow_style);
        } else {
            panic!("Expected flow mapping start");
        }

        // Key scalar
        let event = parser.get_event().unwrap().unwrap();
        if let EventType::Scalar { value, .. } = event.event_type {
            assert_eq!(value, "key");
        } else {
            panic!("Expected scalar 'key'");
        }
    }
}
