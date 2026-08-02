// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Source-preserving, byte-for-byte round-trip documents.
//!
//! [`RoundTripDocument`] keeps the **original source text verbatim** and pairs
//! it with a node -> byte-span index built from the scanner's token stream, so:
//!
//! - re-emitting an unmodified document reproduces the input **byte-for-byte**
//!   (comments, blank lines, indentation, quote style, block scalars, flow
//!   style, anchors, line endings, and trailing whitespace all survive), and
//! - an edit is a **surgical splice**: [`RoundTripDocument::set`] replaces only
//!   the bytes of the addressed node and copies every other byte unchanged.
//!
//! This is different from the emitter-based [`LoaderType::RoundTrip`] path
//! (`load_str_with_comments` / `dump_str_with_comments`), which re-serializes
//! a value tree and therefore normalizes formatting. Only `RoundTripDocument`
//! guarantees byte-for-byte output.
//!
//! ```
//! use rust_yaml::{PathSegment, RoundTripDocument};
//!
//! let input = "# deployment\nreplicas: 3   # scale here\nname: web\n";
//! let mut doc = RoundTripDocument::parse(input).unwrap();
//!
//! // Identity round-trip is byte-for-byte.
//! assert_eq!(doc.to_string(), input);
//!
//! // A surgical edit touches only the addressed bytes.
//! doc.set(&[PathSegment::Key("replicas")], "5").unwrap();
//! assert_eq!(
//!     doc.to_string(),
//!     "# deployment\nreplicas: 5   # scale here\nname: web\n",
//! );
//! ```
//!
//! [`LoaderType::RoundTrip`]: crate::LoaderType::RoundTrip

use std::collections::HashMap;
use std::fmt;

use crate::parser::ScalarStyle;
use crate::scanner::QuoteStyle;
use crate::{
    BasicParser, BasicScanner, Error, Event, EventType, Parser, Position, Result, Scanner, Token,
    TokenType, Value, Yaml,
};

/// A half-open byte range `[start, end)` into [`RoundTripDocument::source`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Inclusive start byte offset.
    pub start: usize,
    /// Exclusive end byte offset.
    pub end: usize,
}

impl Span {
    /// Create a span from start/end byte offsets.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Length of the span in bytes.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.end - self.start
    }

    /// Whether the span covers zero bytes.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// One step of a concrete path from the document root to a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSegment<'a> {
    /// A mapping key, given as the key's resolved scalar text (unquoted,
    /// escapes decoded) -- the same string the composed [`Value`] uses.
    Key(&'a str),
    /// A zero-based sequence index.
    Index(usize),
}

/// The structural span index of one document: mirrors the node tree but
/// stores source byte spans instead of typed values.
#[derive(Debug, Clone)]
enum SpanNode {
    /// A scalar. `span` is `None` for nodes the parser fabricated with no
    /// backing source bytes (e.g. the implicit null in `key:`).
    Scalar { span: Option<Span> },
    /// An alias reference; the span covers the `*name` bytes.
    Alias { span: Span },
    /// A sequence with per-item child nodes.
    Sequence {
        span: Option<Span>,
        children: Vec<SpanNode>,
    },
    /// A mapping with per-entry `(key text, value node)` pairs in source
    /// order. The key text is `None` for complex (non-scalar) keys, which are
    /// not addressable through [`PathSegment::Key`].
    Mapping {
        span: Option<Span>,
        entries: Vec<(Option<String>, SpanNode)>,
    },
}

impl SpanNode {
    /// The byte span of this node, when it has original source bytes.
    const fn span(&self) -> Option<Span> {
        match self {
            SpanNode::Scalar { span } | SpanNode::Sequence { span, .. } => *span,
            SpanNode::Alias { span } => Some(*span),
            SpanNode::Mapping { span, .. } => *span,
        }
    }
}

/// A single YAML document whose original source bytes are preserved verbatim
/// and can be surgically edited span-by-span.
///
/// See the [module documentation](self) for the round-trip guarantee and how
/// this differs from the emitter-based `LoaderType::RoundTrip` path.
#[derive(Debug, Clone)]
pub struct RoundTripDocument {
    /// The document's original bytes, kept verbatim.
    source: String,
    /// Typed view of the document, composed by the regular loading pipeline.
    value: Value,
    /// Structural span index over `source`.
    root: Option<SpanNode>,
}

/// UTF-8 byte-order mark.
const BOM: &str = "\u{feff}";

impl RoundTripDocument {
    /// Parse a string containing exactly one YAML document.
    ///
    /// The original bytes are kept verbatim: `doc.to_string() == input` holds
    /// for every accepted input. Inputs with more than one document are
    /// rejected; use [`RoundTripDocument::parse_all`] for streams.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not valid YAML or contains more
    /// than one document.
    pub fn parse(input: &str) -> Result<Self> {
        let mut docs = Self::parse_all(input)?;
        if docs.len() != 1 {
            return Err(Error::value_error(
                Position::new(),
                format!(
                    "expected a single YAML document, found {}; use parse_all for streams",
                    docs.len()
                ),
            ));
        }
        // `ok_or_else` rather than `expect`: the length check above already
        // proves this is `Some`, but a panic here would abort the process
        // rather than surface an error (see `[profile.release] panic = "abort"`),
        // so the unreachable branch is spelled as an error instead.
        docs.pop().ok_or_else(|| {
            Error::parse(
                Position::new(),
                "internal: single-document check passed but no document was produced".to_string(),
            )
        })
    }

    /// Parse a (possibly multi-document) YAML stream.
    ///
    /// Each returned document owns its exact byte slice of the input;
    /// concatenating every document's [`source`](Self::source) reproduces the
    /// input byte-for-byte. Leading trivia (a BOM, comments, directives)
    /// belongs to the first document; trivia after a document stays with that
    /// document until the next `---` marker or content begins.
    ///
    /// # Errors
    ///
    /// Returns an error when the input is not valid YAML.
    pub fn parse_all(input: &str) -> Result<Vec<Self>> {
        // A leading BOM is a zero-width stream prefix: the scanner has no BOM
        // handling and would fold it into the first token, so scan without it
        // and keep it in the stored source (all spans shift by its length).
        let bom_len = if input.starts_with(BOM) { BOM.len() } else { 0 };
        let scanned = &input[bom_len..];

        // Typed values come from the regular, battle-tested loading pipeline
        // (resource limits, alias-cycle guards, merge keys). The span index
        // built below never disagrees with it dangerously: a path the index
        // cannot mirror faithfully resolves to no span instead of a wrong one.
        let values = Yaml::new().load_all_str(scanned)?;

        let tokens = drain_tokens(scanned)?;
        let events = drain_events(scanned)?;

        let lexemes = LexemeIndex::build(&tokens, input, bom_len);
        let boundaries = document_boundaries(&tokens, bom_len, input.len());
        let trees = document_trees(&events, &lexemes, bom_len);

        if trees.is_empty() && values.len() == 1 && boundaries.len() == 1 {
            // `load_all_str` maps an empty/trivia-only stream to one Null
            // document; mirror that with a spanless document owning all bytes.
            return Ok(vec![Self {
                source: input.to_string(),
                value: Value::Null,
                root: None,
            }]);
        }
        if boundaries.len() != trees.len() || trees.len() != values.len() {
            return Err(Error::parse(
                Position::new(),
                format!(
                    "internal document accounting mismatch: {} byte ranges, {} span trees, {} values",
                    boundaries.len(),
                    trees.len(),
                    values.len()
                ),
            ));
        }

        let mut docs = Vec::with_capacity(trees.len());
        for (i, (tree, value)) in trees.into_iter().zip(values).enumerate() {
            let start = boundaries[i];
            let end = boundaries.get(i + 1).copied().unwrap_or(input.len());
            docs.push(Self {
                source: input[start..end].to_string(),
                value,
                root: Some(rebase_node(tree, start)),
            });
        }
        Ok(docs)
    }

    /// The document's original bytes, verbatim.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The typed value of the document, as composed by the regular loading
    /// pipeline. This view is intentionally lossy (comments, formatting, and
    /// scalar spellings are normalized); fidelity lives in
    /// [`source`](Self::source) and the span index.
    #[must_use]
    pub const fn value(&self) -> &Value {
        &self.value
    }

    /// Resolve a concrete path to the byte span of the node it addresses.
    ///
    /// Returns `None` when the path does not resolve, or when the node has no
    /// original source bytes of its own: nodes the parser fabricated (the
    /// implicit null in `key:`), entries merged in through a `<<` merge key,
    /// nodes inside alias-expanded content, and complex (non-scalar) keys.
    /// An alias node itself resolves to the span of its `*name` reference.
    #[must_use]
    pub fn span_of(&self, path: &[PathSegment<'_>]) -> Option<Span> {
        self.resolve(path).and_then(SpanNode::span)
    }

    /// The original source bytes of the node addressed by `path`.
    ///
    /// This is a verbatim slice of [`source`](Self::source) -- quote
    /// characters, block-scalar headers, and interior formatting are included
    /// exactly as written.
    #[must_use]
    pub fn get(&self, path: &[PathSegment<'_>]) -> Option<&str> {
        self.span_of(path).map(|s| &self.source[s.start..s.end])
    }

    /// Splice `replacement` over the byte range `span`, leaving every other
    /// byte of the document untouched, then re-validate.
    ///
    /// On success the document's source, typed value, and span index all
    /// reflect the edit. On failure the document is left unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error when the span is out of bounds or not on UTF-8
    /// character boundaries, or when the spliced document no longer parses as
    /// a single YAML document.
    pub fn replace_span(&mut self, span: Span, replacement: &str) -> Result<()> {
        if span.start > span.end
            || span.end > self.source.len()
            || !self.source.is_char_boundary(span.start)
            || !self.source.is_char_boundary(span.end)
        {
            return Err(Error::value_error(
                self.position_at(span.start.min(self.source.len())),
                format!(
                    "replace_span range {}..{} is out of bounds or not on character boundaries",
                    span.start, span.end
                ),
            ));
        }

        let mut next = String::with_capacity(self.source.len() + replacement.len());
        next.push_str(&self.source[..span.start]);
        next.push_str(replacement);
        next.push_str(&self.source[span.end..]);

        *self = Self::parse(&next)?;
        Ok(())
    }

    /// Replace the node at `path` with a YAML `fragment`, spliced verbatim.
    ///
    /// The fragment must be valid YAML in the target position -- it is not
    /// quoted, indented, or otherwise reformatted. Every byte outside the
    /// addressed node's span is preserved unchanged, so comments and
    /// formatting around the edit survive.
    ///
    /// # Errors
    ///
    /// Returns an error when the path does not resolve, addresses a node with
    /// no source bytes of its own (see [`span_of`](Self::span_of)), or the
    /// spliced document no longer parses.
    pub fn set(&mut self, path: &[PathSegment<'_>], fragment: &str) -> Result<()> {
        let Some(node) = self.resolve(path) else {
            return Err(Error::value_error(
                Position::new(),
                format!("path {} does not resolve in this document", fmt_path(path)),
            ));
        };
        let Some(span) = node.span() else {
            return Err(Error::value_error(
                Position::new(),
                format!(
                    "path {} resolves to a node without its own source bytes \
                     (implicit null, merge-key entry, or alias-expanded content)",
                    fmt_path(path)
                ),
            ));
        };
        self.replace_span(span, fragment)
    }

    /// Walk the span index by path segments.
    fn resolve(&self, path: &[PathSegment<'_>]) -> Option<&SpanNode> {
        let mut node = self.root.as_ref()?;
        for seg in path {
            node = match (seg, node) {
                (PathSegment::Key(k), SpanNode::Mapping { entries, .. }) => {
                    // Duplicate keys: the composed value keeps the LAST
                    // occurrence's value, so resolve to the last match.
                    entries
                        .iter()
                        .rev()
                        .find(|(key, _)| key.as_deref() == Some(*k))
                        .map(|(_, v)| v)?
                }
                (PathSegment::Index(i), SpanNode::Sequence { children, .. }) => children.get(*i)?,
                _ => return None,
            };
        }
        Some(node)
    }

    /// Synthesize a line/column [`Position`] for a byte offset into `source`,
    /// so errors point at the right place. The offset is snapped back to the
    /// previous character boundary so this never panics, even when reporting
    /// an error about a mid-character offset.
    fn position_at(&self, index: usize) -> Position {
        let mut index = index.min(self.source.len());
        while index > 0 && !self.source.is_char_boundary(index) {
            index -= 1;
        }
        let prefix = &self.source[..index];
        let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
        let column = prefix
            .rsplit_once('\n')
            .map_or(prefix.chars().count(), |(_, tail)| tail.chars().count())
            + 1;
        Position::at(line, column, index)
    }
}

impl fmt::Display for RoundTripDocument {
    /// Emits the document's original bytes verbatim: for an unmodified parse,
    /// `doc.to_string()` equals the input byte-for-byte.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.source)
    }
}

/// Render a path for error messages, jq-style (`.a.b[0]`).
fn fmt_path(path: &[PathSegment<'_>]) -> String {
    if path.is_empty() {
        return ".".to_string();
    }
    let mut out = String::new();
    for seg in path {
        match seg {
            PathSegment::Key(k) => {
                out.push('.');
                out.push_str(k);
            }
            PathSegment::Index(i) => {
                out.push('[');
                out.push_str(&i.to_string());
                out.push(']');
            }
        }
    }
    out
}

/// Eagerly scan `input` into its full token stream, surfacing any deferred
/// scanner error.
fn drain_tokens(input: &str) -> Result<Vec<Token>> {
    let mut scanner = BasicScanner::new_eager(input.to_string());
    let mut tokens = Vec::new();
    while let Some(token) = scanner.get_token()? {
        let is_end = matches!(token.token_type, TokenType::StreamEnd);
        tokens.push(token);
        if is_end {
            break;
        }
    }
    if let Some(err) = scanner.take_scanning_error() {
        return Err(err);
    }
    Ok(tokens)
}

/// Eagerly parse `input` into its full event stream, surfacing any deferred
/// error.
fn drain_events(input: &str) -> Result<Vec<Event>> {
    let mut parser = BasicParser::new_eager(input.to_string());
    let mut events = Vec::new();
    while let Some(event) = parser.get_event()? {
        let is_end = matches!(event.event_type, EventType::StreamEnd);
        events.push(event);
        if is_end {
            break;
        }
    }
    if let Some(err) = parser.take_scanning_error() {
        return Err(err);
    }
    Ok(events)
}

/// What kind of source lexeme a span-bearing token is, used to match parser
/// events (which carry only a start position) back to their tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexemeKind {
    Plain,
    SingleQuoted,
    DoubleQuoted,
    Literal,
    Folded,
    Alias,
}

/// A span-bearing lexeme, keyed in [`LexemeIndex`] by its start byte offset.
#[derive(Debug, Clone)]
struct Lexeme {
    end: usize,
    value: String,
    kind: LexemeKind,
}

/// Index of scalar/alias tokens by start byte offset. Offsets are rebased by
/// the BOM length so they index the ORIGINAL input string.
struct LexemeIndex {
    by_start: HashMap<usize, Lexeme>,
}

impl LexemeIndex {
    fn build(tokens: &[Token], original: &str, bom_len: usize) -> Self {
        let mut by_start = HashMap::new();
        for token in tokens {
            let (value, kind) = match &token.token_type {
                TokenType::Scalar(v, style) => (
                    v.clone(),
                    match style {
                        QuoteStyle::Plain => LexemeKind::Plain,
                        QuoteStyle::Single => LexemeKind::SingleQuoted,
                        QuoteStyle::Double => LexemeKind::DoubleQuoted,
                    },
                ),
                TokenType::BlockScalarLiteral(v) => (v.clone(), LexemeKind::Literal),
                TokenType::BlockScalarFolded(v) => (v.clone(), LexemeKind::Folded),
                TokenType::Alias(name) => (name.clone(), LexemeKind::Alias),
                _ => continue,
            };
            let start = token.start_position.index + bom_len;
            let mut end = token.end_position.index + bom_len;
            // Plain-scalar token spans include trailing spaces/tabs that
            // precede a comment or line break (the token VALUE is trimmed).
            // Trim the span too, so a splice cannot swallow the padding
            // before an end-of-line comment.
            if kind == LexemeKind::Plain {
                end = start + original[start..end].trim_end_matches([' ', '\t']).len();
            }
            by_start.insert(start, Lexeme { end, value, kind });
        }
        Self { by_start }
    }

    /// Match a scalar event back to its token span. Returns `None` for
    /// fabricated events (implicit nulls) whose position points at some other
    /// token, which must not inherit that token's span.
    fn scalar_span(&self, start: usize, value: &str, style: ScalarStyle) -> Option<Span> {
        let lexeme = self.by_start.get(&start)?;
        let style_matches = matches!(
            (lexeme.kind, style),
            (LexemeKind::Plain, ScalarStyle::Plain)
                | (LexemeKind::SingleQuoted, ScalarStyle::SingleQuoted)
                | (LexemeKind::DoubleQuoted, ScalarStyle::DoubleQuoted)
                | (LexemeKind::Literal, ScalarStyle::Literal)
                | (LexemeKind::Folded, ScalarStyle::Folded)
        );
        if !style_matches || lexeme.value != value {
            return None;
        }
        Some(Span::new(start, lexeme.end))
    }

    /// Match an alias event back to its `*name` token span.
    fn alias_span(&self, start: usize, name: &str) -> Option<Span> {
        let lexeme = self.by_start.get(&start)?;
        if lexeme.kind != LexemeKind::Alias || lexeme.value != name {
            return None;
        }
        Some(Span::new(start, lexeme.end))
    }
}

/// Compute each document's starting byte offset in the original input.
///
/// The first document always starts at byte 0 (owning any leading BOM,
/// comments, and directives). A later document starts at its `---` marker,
/// or -- after an explicit `...` terminator -- at its first content token.
fn document_boundaries(tokens: &[Token], bom_len: usize, input_len: usize) -> Vec<usize> {
    let mut boundaries = vec![0];
    let mut has_open_doc = false;
    let mut content_seen = false;
    let mut doc_closed = false;

    for token in tokens {
        let start = token.start_position.index + bom_len;
        match &token.token_type {
            TokenType::StreamStart | TokenType::StreamEnd => {}
            TokenType::DocumentStart => {
                if has_open_doc || content_seen || doc_closed {
                    boundaries.push(start);
                }
                has_open_doc = true;
                content_seen = false;
                doc_closed = false;
            }
            TokenType::DocumentEnd => {
                has_open_doc = false;
                content_seen = false;
                doc_closed = true;
            }
            TokenType::Comment(_) | TokenType::YamlDirective(..) | TokenType::TagDirective(..) => {}
            _ => {
                if doc_closed {
                    boundaries.push(start);
                    doc_closed = false;
                }
                content_seen = true;
            }
        }
    }

    boundaries.retain(|&b| b <= input_len);
    boundaries.dedup();
    boundaries
}

/// Split the event stream into one span tree per document, mirroring the
/// composer's per-document loop (consume leading `DocumentStart` events, walk
/// one node, consume trailing `DocumentEnd` events).
fn document_trees(events: &[Event], lexemes: &LexemeIndex, bom_len: usize) -> Vec<SpanNode> {
    let mut cursor = Cursor { events, pos: 0 };
    let mut trees = Vec::new();

    if matches!(
        cursor.peek().map(|e| &e.event_type),
        Some(EventType::StreamStart)
    ) {
        cursor.advance();
    }

    loop {
        while matches!(
            cursor.peek().map(|e| &e.event_type),
            Some(EventType::DocumentStart { .. })
        ) {
            cursor.advance();
        }
        match cursor.peek().map(|e| &e.event_type) {
            None | Some(EventType::StreamEnd) => break,
            _ => {}
        }
        let Some(tree) = walk_node(&mut cursor, lexemes, bom_len) else {
            break;
        };
        trees.push(tree);
        while matches!(
            cursor.peek().map(|e| &e.event_type),
            Some(EventType::DocumentEnd { .. })
        ) {
            cursor.advance();
        }
    }

    trees
}

/// A simple forward cursor over the parsed event stream.
struct Cursor<'a> {
    events: &'a [Event],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn peek(&self) -> Option<&'a Event> {
        self.events.get(self.pos)
    }

    fn advance(&mut self) -> Option<&'a Event> {
        let event = self.events.get(self.pos)?;
        self.pos += 1;
        Some(event)
    }

    /// Whether the next event terminates the enclosing structure without
    /// belonging to it (a document boundary or the end of the stream).
    fn at_boundary(&self) -> bool {
        matches!(
            self.peek().map(|e| &e.event_type),
            None | Some(
                EventType::DocumentStart { .. }
                    | EventType::DocumentEnd { .. }
                    | EventType::StreamEnd
            )
        )
    }
}

/// Recursively build the span tree for one node, mirroring the composer's
/// event walk (including its rule of breaking WITHOUT consuming document
/// boundary events that appear mid-collection).
fn walk_node(cursor: &mut Cursor<'_>, lexemes: &LexemeIndex, bom_len: usize) -> Option<SpanNode> {
    let event = cursor.advance()?;
    let start = event.position.index + bom_len;
    match &event.event_type {
        EventType::Scalar { value, style, .. } => Some(SpanNode::Scalar {
            span: lexemes.scalar_span(start, value, *style),
        }),
        EventType::Alias { anchor } => Some(
            lexemes
                .alias_span(start, anchor)
                .map_or(SpanNode::Scalar { span: None }, |span| SpanNode::Alias {
                    span,
                }),
        ),
        EventType::SequenceStart { flow_style, .. } => {
            let mut children = Vec::new();
            let mut closer = None;
            loop {
                if matches!(
                    cursor.peek().map(|e| &e.event_type),
                    Some(EventType::SequenceEnd)
                ) {
                    closer = cursor.advance().map(|e| e.position.index + bom_len);
                    break;
                }
                if cursor.at_boundary() {
                    break;
                }
                match walk_node(cursor, lexemes, bom_len) {
                    Some(child) => children.push(child),
                    None => break,
                }
            }
            let end = collection_end(*flow_style, closer, children.iter());
            Some(SpanNode::Sequence {
                span: end.map(|e| Span::new(start, e)),
                children,
            })
        }
        EventType::MappingStart { flow_style, .. } => {
            let mut entries: Vec<(Option<String>, SpanNode)> = Vec::new();
            let mut key_nodes: Vec<SpanNode> = Vec::new();
            let mut closer = None;
            loop {
                if matches!(
                    cursor.peek().map(|e| &e.event_type),
                    Some(EventType::MappingEnd)
                ) {
                    closer = cursor.advance().map(|e| e.position.index + bom_len);
                    break;
                }
                if cursor.at_boundary() {
                    break;
                }
                // Key: remember its scalar text for path resolution before
                // walking it as a node.
                let key_text = match cursor.peek().map(|e| &e.event_type) {
                    Some(EventType::Scalar { value, .. }) => Some(value.clone()),
                    _ => None,
                };
                let Some(key_node) = walk_node(cursor, lexemes, bom_len) else {
                    break;
                };
                let entry_key = key_text.filter(|_| matches!(key_node, SpanNode::Scalar { .. }));
                // Value: a missing value is fabricated by the parser as an
                // implicit null event, so a well-formed stream always has
                // one; guard against boundary truncation anyway.
                let value_node = if cursor.at_boundary()
                    || matches!(
                        cursor.peek().map(|e| &e.event_type),
                        Some(EventType::MappingEnd)
                    ) {
                    SpanNode::Scalar { span: None }
                } else {
                    walk_node(cursor, lexemes, bom_len).unwrap_or(SpanNode::Scalar { span: None })
                };
                key_nodes.push(key_node);
                entries.push((entry_key, value_node));
            }
            // A block mapping's span must cover its KEY bytes too: the last
            // entry may have a spanless implicit-null value, in which case
            // the key's span still extends the mapping.
            let end = collection_end(
                *flow_style,
                closer,
                entries.iter().map(|(_, v)| v).chain(key_nodes.iter()),
            );
            Some(SpanNode::Mapping {
                span: end.map(|e| Span::new(start, e)),
                entries,
            })
        }
        EventType::StreamStart
        | EventType::StreamEnd
        | EventType::DocumentStart { .. }
        | EventType::DocumentEnd { .. }
        | EventType::SequenceEnd
        | EventType::MappingEnd => None,
    }
}

/// Compute where a collection's span ends: flow collections end at their
/// one-byte closing bracket (whose start the end event carries); block
/// collections extend to the last spanned byte of any child node.
fn collection_end<'a>(
    flow_style: bool,
    closer: Option<usize>,
    nodes: impl Iterator<Item = &'a SpanNode>,
) -> Option<usize> {
    if flow_style {
        return closer.map(|p| p + 1);
    }
    nodes.filter_map(|n| n.span().map(|s| s.end)).max()
}

/// Rebase a span tree so offsets index into a document's own source slice.
fn rebase_node(node: SpanNode, base: usize) -> SpanNode {
    let rebase = |s: Span| Span::new(s.start - base, s.end - base);
    match node {
        SpanNode::Scalar { span } => SpanNode::Scalar {
            span: span.map(rebase),
        },
        SpanNode::Alias { span } => SpanNode::Alias { span: rebase(span) },
        SpanNode::Sequence { span, children } => SpanNode::Sequence {
            span: span.map(rebase),
            children: children.into_iter().map(|c| rebase_node(c, base)).collect(),
        },
        SpanNode::Mapping { span, entries } => SpanNode::Mapping {
            span: span.map(rebase),
            entries: entries
                .into_iter()
                .map(|(k, v)| (k, rebase_node(v, base)))
                .collect(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trip_preserves_bytes() {
        let cases: &[&str] = &[
            "# header\nname: my-app   # inline\n\n# section\nreplicas: 3\n",
            "a: 1\n\nb: 2\n\n\nc: 3\n",
            "root:\n    child:\n        leaf: value\n    sibling: other\n",
            "bare: hello\nsingle: 'hello world'\ndouble: \"hello world\"\nforced: \"123\"\n",
            "literal: |\n  line one\n  line two\nfolded: >\n  this is\n  folded text\n",
            "replicas: 3\nratio: 1.0\nzip: 007\nbig: 12345678901234567\n",
            "flow_map: {a: 1, b: 2}\nflow_seq: [1, 2, 3]\n",
            "zebra: 1\napple: 2\nmango: 3\n",
            "defaults: &defaults\n  timeout: 30\nservice:\n  <<: *defaults\n  name: web\n",
            "a: 1\r\nb: 2\r\n",
            "a: 1   \nb: 2\t\n",
            "\u{feff}a: 1\nb: 2\n",
        ];
        for case in cases {
            let doc = RoundTripDocument::parse(case).expect(case);
            assert_eq!(&doc.to_string(), case, "byte fidelity for {case:?}");
        }
    }

    #[test]
    fn parse_all_reproduces_multi_document_streams() {
        let cases: &[&str] = &[
            "---\na: 1\n---\nb: 2\n",
            "a: 1\n---\nb: 2\n",
            "# lead\n---\na: 1\n...\n---\nb: 2\n",
        ];
        for case in cases {
            let docs = RoundTripDocument::parse_all(case).expect(case);
            let joined: String = docs.iter().map(RoundTripDocument::source).collect();
            assert_eq!(&joined, case, "stream fidelity for {case:?}");
        }
    }

    #[test]
    fn parse_rejects_multi_document_input() {
        assert!(RoundTripDocument::parse("---\na: 1\n---\nb: 2\n").is_err());
    }

    #[test]
    fn span_of_resolves_scalars() {
        let input = "name: web\nreplicas: 3\n";
        let doc = RoundTripDocument::parse(input).unwrap();
        let span = doc.span_of(&[PathSegment::Key("replicas")]).unwrap();
        assert_eq!(&input[span.start..span.end], "3");
        assert_eq!(doc.get(&[PathSegment::Key("name")]), Some("web"));
    }

    #[test]
    fn span_of_resolves_nested_and_indexed() {
        let input = "spec:\n  items:\n    - alpha\n    - beta\n";
        let doc = RoundTripDocument::parse(input).unwrap();
        let path = [
            PathSegment::Key("spec"),
            PathSegment::Key("items"),
            PathSegment::Index(1),
        ];
        assert_eq!(doc.get(&path), Some("beta"));
    }

    #[test]
    fn quoted_scalar_span_includes_quotes() {
        let doc = RoundTripDocument::parse("s: 'hello world'\n").unwrap();
        assert_eq!(doc.get(&[PathSegment::Key("s")]), Some("'hello world'"));
    }

    #[test]
    fn plain_scalar_span_excludes_padding_before_comment() {
        let doc = RoundTripDocument::parse("name: web        # frontend\n").unwrap();
        assert_eq!(doc.get(&[PathSegment::Key("name")]), Some("web"));
    }

    #[test]
    fn block_mapping_span_covers_all_entries() {
        let input = "m:\n  a: 1\n  b:\n";
        let doc = RoundTripDocument::parse(input).unwrap();
        let span = doc.span_of(&[PathSegment::Key("m")]).unwrap();
        // The span must reach past key `b` even though its value is an
        // implicit null with no bytes.
        assert!(input[span.start..span.end].contains('b'));
    }

    #[test]
    fn set_replaces_only_the_addressed_bytes() {
        let input = "# deploy\nreplicas: 3   # scale\nname: web\n";
        let mut doc = RoundTripDocument::parse(input).unwrap();
        doc.set(&[PathSegment::Key("replicas")], "5").unwrap();
        assert_eq!(
            doc.to_string(),
            "# deploy\nreplicas: 5   # scale\nname: web\n"
        );
        assert_eq!(doc.value().get_str("replicas"), Some(&Value::Int(5)));
    }

    #[test]
    fn set_on_missing_path_is_an_error_and_leaves_doc_unchanged() {
        let input = "a: 1\n";
        let mut doc = RoundTripDocument::parse(input).unwrap();
        assert!(doc.set(&[PathSegment::Key("nope")], "2").is_err());
        assert_eq!(doc.to_string(), input);
    }

    #[test]
    fn replace_span_rejects_invalid_result_and_rolls_back() {
        let input = "a: 1\n";
        let mut doc = RoundTripDocument::parse(input).unwrap();
        let span = doc.span_of(&[PathSegment::Key("a")]).unwrap();
        assert!(doc.replace_span(span, "[unclosed").is_err());
        assert_eq!(doc.to_string(), input);
    }

    #[test]
    fn replace_span_rejects_non_char_boundaries() {
        let input = "k: \u{e9}t\u{e9}\n"; // 'k: été\n' -- multibyte scalars
        let mut doc = RoundTripDocument::parse(input).unwrap();
        assert!(doc.replace_span(Span::new(4, 5), "x").is_err());
        assert_eq!(doc.to_string(), input);
    }

    #[test]
    fn implicit_null_has_no_span_but_typed_value_exists() {
        let doc = RoundTripDocument::parse("key:\nother: 1\n").unwrap();
        assert_eq!(doc.span_of(&[PathSegment::Key("key")]), None);
        assert_eq!(doc.value().get_str("key"), Some(&Value::Null));
    }

    #[test]
    fn merge_key_entries_are_not_addressable() {
        let input = "defaults: &d\n  timeout: 30\nservice:\n  <<: *d\n  name: web\n";
        let doc = RoundTripDocument::parse(input).unwrap();
        // The merged-in entry exists in the typed value...
        let service = doc.value().get_str("service").unwrap();
        assert_eq!(service.get_str("timeout"), Some(&Value::Int(30)));
        // ...but has no bytes inside `service`, so no span.
        let path = [PathSegment::Key("service"), PathSegment::Key("timeout")];
        assert_eq!(doc.span_of(&path), None);
        // The alias node itself is addressable.
        let alias = [PathSegment::Key("service"), PathSegment::Key("<<")];
        assert_eq!(doc.get(&alias), Some("*d"));
    }

    #[test]
    fn duplicate_keys_resolve_to_last_occurrence() {
        let doc = RoundTripDocument::parse("k: 1\nk: 2\n").unwrap();
        assert_eq!(doc.get(&[PathSegment::Key("k")]), Some("2"));
    }

    #[test]
    fn bom_document_is_preserved_and_addressable() {
        let input = "\u{feff}a: 1\nb: 2\n";
        let doc = RoundTripDocument::parse(input).unwrap();
        assert_eq!(doc.to_string(), input);
        assert_eq!(doc.get(&[PathSegment::Key("a")]), Some("1"));
    }

    #[test]
    fn empty_input_is_one_null_document() {
        let docs = RoundTripDocument::parse_all("").unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].value(), &Value::Null);
        assert_eq!(docs[0].source(), "");
    }

    #[test]
    fn invalid_yaml_is_an_error() {
        assert!(RoundTripDocument::parse("items: [1, 2").is_err());
    }

    #[test]
    fn multibyte_content_keeps_spans_byte_accurate() {
        let input = "gr\u{fc}\u{df}e: \"h\u{e9}llo\"\nkey: \u{1f600}\n";
        let doc = RoundTripDocument::parse(input).unwrap();
        assert_eq!(doc.to_string(), input);
        assert_eq!(
            doc.get(&[PathSegment::Key("gr\u{fc}\u{df}e")]),
            Some("\"h\u{e9}llo\"")
        );
        assert_eq!(doc.get(&[PathSegment::Key("key")]), Some("\u{1f600}"));
    }
}
