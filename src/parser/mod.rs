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
    pending_tag: Option<String>,
    last_token_type: Option<TokenType>,
    scanning_error: Option<Error>,
    yaml_version: Option<(u8, u8)>,
    tag_directives: Vec<(String, String)>,
    tag_resolver: TagResolver,
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
            pending_tag: None,
            last_token_type: None,
            scanning_error: None,
            yaml_version: None,
            tag_directives: Vec::new(),
            tag_resolver: TagResolver::new(),
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
            pending_tag: None,
            last_token_type: None,
            scanning_error: None,
            yaml_version: None,
            tag_directives: Vec::new(),
            tag_resolver: TagResolver::new(),
        };

        // If there was a scanning error, store it for later propagation
        if let Some(error) = scanning_error {
            parser.scanning_error = Some(error);
        } else {
            // Parse all events immediately only if there were no scanning errors
            parser.parse_all().unwrap_or(());
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
            pending_tag: None,
            last_token_type: None,
            scanning_error: None,
            yaml_version: None,
            tag_directives: Vec::new(),
            tag_resolver: TagResolver::new(),
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
                // Close any open document
                if matches!(
                    self.state,
                    ParserState::DocumentContent | ParserState::BlockNode
                ) {
                    self.events
                        .push(Event::document_end(token.start_position, true));
                }
                self.events.push(Event::stream_end(token.start_position));
                self.state = ParserState::StreamEnd;
            }

            TokenType::YamlDirective(major, minor) => {
                // Store YAML version directive
                self.yaml_version = Some((*major, *minor));
                // Stay in stream state waiting for document
            }

            TokenType::TagDirective(handle, prefix) => {
                // Store tag directive and update tag resolver
                self.tag_directives.push((handle.clone(), prefix.clone()));
                self.tag_resolver
                    .add_directive(handle.clone(), prefix.clone());
                // Stay in stream state waiting for document
            }

            TokenType::DocumentStart => {
                // Close previous document if needed
                if matches!(
                    self.state,
                    ParserState::DocumentContent | ParserState::BlockNode
                ) {
                    self.events
                        .push(Event::document_end(token.start_position, true));
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
                self.events
                    .push(Event::document_end(token.start_position, false));
                self.state = ParserState::DocumentEnd;
            }

            TokenType::BlockSequenceStart => {
                if matches!(self.state, ParserState::ImplicitDocumentStart) {
                    let event = self.create_implicit_document_start(token.start_position);
                    self.events.push(event);
                }

                // If we're starting a sequence within a mapping context, push the current state
                if matches!(
                    self.state,
                    ParserState::BlockMappingValue | ParserState::BlockMappingKey
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

                    self.events.push(Event::mapping_start(
                        token.start_position,
                        self.pending_anchor.take(),
                        self.pending_tag.take(),
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
                self.events.push(Event::mapping_start(
                    token.start_position,
                    self.pending_anchor.take(),
                    self.pending_tag.take(),
                    true,
                ));
                self.state = ParserState::FlowMapping;
            }

            TokenType::FlowSequenceEnd => {
                self.events.push(Event::sequence_end(token.start_position));
                self.state = ParserState::DocumentContent;

                // Handle state transitions for mapping key/value processing
                self.handle_node_completion();
            }

            TokenType::FlowMappingEnd => {
                self.events.push(Event::mapping_end(token.start_position));
                self.state = ParserState::DocumentContent;

                // Handle state transitions for mapping key/value processing
                self.handle_node_completion();
            }

            TokenType::BlockEnd => {
                // Determine what we're ending based on current state
                match self.state {
                    ParserState::BlockSequence => {
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
                        // We're already in a sequence, this is a new item
                        // No event needed, but we should be ready for the next item
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
                // Key-value separator in mappings
                match self.state {
                    ParserState::BlockMappingKey => {
                        self.state = ParserState::BlockMappingValue;
                    }
                    ParserState::FlowMappingKey => {
                        self.state = ParserState::FlowMappingValue;
                    }
                    _ => {
                        // In other contexts, Value token doesn't change state
                        // It's handled by the scanner's mapping detection logic
                    }
                }
            }

            TokenType::FlowEntry => {
                // Flow collection separator, no specific event needed
            }

            TokenType::Anchor(name) => {
                // Store the anchor name to be used with the next node
                self.pending_anchor = Some(name.clone());
            }

            TokenType::Alias(name) => {
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
                // Resolve and normalize the tag before storing
                match self.tag_resolver.resolve(&tag) {
                    Ok(resolved_tag) => {
                        self.pending_tag = Some(resolved_tag.uri);
                    }
                    Err(_) => {
                        // If tag resolution fails, store the original tag
                        self.pending_tag = Some(tag.clone());
                    }
                }
            }

            // TODO: Implement these when we add support for advanced features
            TokenType::Comment(_) => {
                // Not implemented in basic version
            }

            // Complex key marker
            TokenType::Key => {
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
                    ParserState::BlockMappingKey | ParserState::FlowMappingKey => {
                        // Already in a mapping and expecting a key, this is another complex key
                        // Don't start a new mapping, just continue with the current state
                        // The state is already correct for expecting a key
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
        // For streaming: check if we have cached events or can generate more
        self.event_index < self.events.len() || self.scanner.check_token()
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
