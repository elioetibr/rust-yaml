//! Streaming parser implementation for efficient YAML processing
//!
//! This module provides a streaming parser that processes YAML incrementally,
//! reducing memory usage and improving performance for large documents.

use crate::{
    parser::{Event, Parser, ParserState},
    zerocopy::ScannerStats,
    BasicScanner, Error, Position, Result, Scanner, Token, TokenType, ZeroScanner, ZeroToken,
    ZeroTokenType,
};
use std::collections::VecDeque;

/// Configuration for streaming parser behavior
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    /// Maximum number of events to buffer
    pub max_buffer_size: usize,
    /// Enable zero-copy optimizations where possible
    pub use_zero_copy: bool,
    /// Maximum depth for nested structures
    pub max_depth: usize,
    /// Enable streaming statistics collection
    pub collect_stats: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_buffer_size: 64,
            use_zero_copy: true,
            max_depth: 256,
            collect_stats: false,
        }
    }
}

/// Statistics about streaming parser performance
#[derive(Debug, Clone)]
pub struct StreamingStats {
    /// Number of events processed
    pub events_processed: usize,
    /// Number of tokens processed
    pub tokens_processed: usize,
    /// Maximum buffer size reached
    pub max_buffer_size: usize,
    /// Maximum nesting depth reached
    pub max_depth: usize,
    /// Zero-copy scanner statistics (if enabled)
    pub scanner_stats: Option<ScannerStats>,
    /// Total time spent parsing (in nanoseconds)
    pub parse_time_ns: u64,
}

/// Streaming YAML parser that processes events on demand
pub struct StreamingParser<'a> {
    /// Traditional scanner for compatibility
    scanner: Option<BasicScanner>,
    /// Zero-copy scanner for optimized parsing
    zero_scanner: Option<ZeroScanner<'a>>,
    /// Configuration
    config: StreamingConfig,
    /// Event buffer for batched processing
    event_buffer: VecDeque<Event>,
    /// Current parser state
    state: ParserState,
    /// State stack for nested structures
    state_stack: Vec<ParserState>,
    /// Current position
    position: Position,
    /// Current nesting depth
    depth: usize,
    /// Pending anchor for next node
    pending_anchor: Option<String>,
    /// Pending tag for next node
    pending_tag: Option<String>,
    /// Statistics (if enabled)
    stats: Option<StreamingStats>,
    /// Start time for performance measurement
    start_time: std::time::Instant,
    /// Whether the stream has ended
    stream_ended: bool,
}

impl<'a> StreamingParser<'a> {
    /// Create a new streaming parser with traditional scanner
    pub fn new(input: String, config: StreamingConfig) -> StreamingParser<'static> {
        let scanner = BasicScanner::new(input);
        let position = scanner.position();

        StreamingParser {
            scanner: Some(scanner),
            zero_scanner: None,
            config: config.clone(),
            event_buffer: VecDeque::with_capacity(config.max_buffer_size),
            state: ParserState::StreamStart,
            state_stack: Vec::with_capacity(config.max_depth),
            position,
            depth: 0,
            pending_anchor: None,
            pending_tag: None,
            stats: if config.collect_stats {
                Some(StreamingStats {
                    events_processed: 0,
                    tokens_processed: 0,
                    max_buffer_size: 0,
                    max_depth: 0,
                    scanner_stats: None,
                    parse_time_ns: 0,
                })
            } else {
                None
            },
            start_time: std::time::Instant::now(),
            stream_ended: false,
        }
    }

    /// Create a new streaming parser with zero-copy scanner
    pub fn new_zero_copy(input: &'a str, config: StreamingConfig) -> Self {
        let zero_scanner = ZeroScanner::new(input);
        let position = zero_scanner.position;

        Self {
            scanner: None,
            zero_scanner: Some(zero_scanner),
            config: config.clone(),
            event_buffer: VecDeque::with_capacity(config.max_buffer_size),
            state: ParserState::StreamStart,
            state_stack: Vec::with_capacity(config.max_depth),
            position,
            depth: 0,
            pending_anchor: None,
            pending_tag: None,
            stats: if config.collect_stats {
                Some(StreamingStats {
                    events_processed: 0,
                    tokens_processed: 0,
                    max_buffer_size: 0,
                    max_depth: 0,
                    scanner_stats: None,
                    parse_time_ns: 0,
                })
            } else {
                None
            },
            start_time: std::time::Instant::now(),
            stream_ended: false,
        }
    }

    /// Get the next batch of events
    pub fn next_batch(&mut self) -> Result<Vec<Event>> {
        if self.stream_ended {
            return Ok(Vec::new());
        }

        let mut events = Vec::new();
        let target_size = std::cmp::min(self.config.max_buffer_size / 2, 8);

        while events.len() < target_size && !self.stream_ended {
            if let Some(event) = self.next_event_internal()? {
                events.push(event);
            } else {
                break;
            }
        }

        Ok(events)
    }

    /// Get the next event from buffer or generate a new one
    fn next_event_internal(&mut self) -> Result<Option<Event>> {
        // Return buffered event if available
        if let Some(event) = self.event_buffer.pop_front() {
            self.update_stats_for_event(&event);
            return Ok(Some(event));
        }

        // Generate new event(s)
        self.generate_events()?;

        // Return the first buffered event
        if let Some(event) = self.event_buffer.pop_front() {
            self.update_stats_for_event(&event);
            Ok(Some(event))
        } else {
            Ok(None)
        }
    }

    /// Generate events by processing tokens
    fn generate_events(&mut self) -> Result<()> {
        if self.stream_ended {
            return Ok(());
        }

        // Check depth limit
        if self.depth > self.config.max_depth {
            return Err(Error::parse(
                self.position,
                format!("Maximum nesting depth exceeded: {}", self.config.max_depth),
            ));
        }

        if self.config.use_zero_copy && self.zero_scanner.is_some() {
            self.generate_events_zero_copy()
        } else {
            self.generate_events_traditional()
        }
    }

    /// Generate events using zero-copy scanner
    fn generate_events_zero_copy(&mut self) -> Result<()> {
        // Process a small batch of characters/tokens
        let batch_size = 16;
        let mut processed = 0;

        while processed < batch_size {
            // Get current character without borrowing the entire scanner
            let current_char = if let Some(scanner) = &self.zero_scanner {
                scanner.current_char()
            } else {
                None
            };

            if current_char.is_none() {
                // End of stream
                if !matches!(self.state, ParserState::StreamEnd) {
                    self.event_buffer
                        .push_back(Event::stream_end(self.position));
                    self.stream_ended = true;
                }
                break;
            }

            // Skip whitespace efficiently
            if let Some(scanner) = &mut self.zero_scanner {
                scanner.skip_whitespace();
            }

            let ch = current_char.unwrap();
            match ch {
                '-' if self.is_document_start_candidate_simple() => {
                    self.handle_document_start();
                    // Advance past the "---"
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                        scanner.advance();
                        scanner.advance();
                    }
                }
                '.' if self.is_document_end_candidate_simple() => {
                    self.handle_document_end();
                    // Advance past the "..."
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                        scanner.advance();
                        scanner.advance();
                    }
                }
                '[' => {
                    self.handle_flow_sequence_start();
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                    }
                }
                ']' => {
                    self.handle_flow_sequence_end();
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                    }
                }
                '{' => {
                    self.handle_flow_mapping_start();
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                    }
                }
                '}' => {
                    self.handle_flow_mapping_end();
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                    }
                }
                ':' if self.is_value_indicator_simple() => {
                    self.handle_value_indicator();
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                    }
                }
                ',' => {
                    // Flow entry separator
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                    }
                }
                '#' => {
                    // Skip comments for now
                    self.skip_comment_simple();
                }
                ch if ch.is_alphabetic() || ch.is_numeric() => {
                    // Scan scalar using zero-copy
                    let scalar_token = if let Some(scanner) = &mut self.zero_scanner {
                        scanner.scan_plain_scalar_zero_copy()?
                    } else {
                        return Err(Error::parse(
                            self.position,
                            "No scanner available".to_string(),
                        ));
                    };
                    self.handle_zero_copy_scalar(scalar_token)?;
                }
                '&' => {
                    // Anchor
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance(); // Skip '&'
                        let anchor = scanner.scan_identifier_zero_copy()?;
                        self.pending_anchor = Some(anchor.as_str().to_string());
                    }
                }
                '*' => {
                    // Alias
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance(); // Skip '*'
                        let alias = scanner.scan_identifier_zero_copy()?;
                        self.event_buffer
                            .push_back(Event::alias(self.position, alias.as_str().to_string()));
                    }
                }
                _ => {
                    // Unknown character, skip it
                    if let Some(scanner) = &mut self.zero_scanner {
                        scanner.advance();
                    }
                }
            }

            processed += 1;
            if let Some(scanner) = &self.zero_scanner {
                self.position = scanner.position;
            }

            if let Some(ref mut stats) = self.stats {
                stats.tokens_processed += 1;
            }
        }

        Ok(())
    }

    /// Generate events using traditional scanner
    fn generate_events_traditional(&mut self) -> Result<()> {
        // Process a few tokens at a time
        for _ in 0..4 {
            let has_token = if let Some(scanner) = &self.scanner {
                scanner.check_token()
            } else {
                false
            };

            if !has_token {
                if !matches!(self.state, ParserState::StreamEnd) {
                    self.event_buffer
                        .push_back(Event::stream_end(self.position));
                    self.stream_ended = true;
                }
                break;
            }

            let token = if let Some(scanner) = &mut self.scanner {
                scanner.get_token()?
            } else {
                None
            };

            if let Some(token) = token {
                self.process_token(token)?;

                if let Some(ref mut stats) = self.stats {
                    stats.tokens_processed += 1;
                }
            }
        }

        Ok(())
    }

    /// Check if this position could be document start (---) - simplified version
    fn is_document_start_candidate_simple(&self) -> bool {
        if let Some(scanner) = &self.zero_scanner {
            scanner.current_char() == Some('-')
                && scanner.peek_char(1) == Some('-')
                && scanner.peek_char(2) == Some('-')
                && scanner.peek_char(3).map_or(true, |c| c.is_whitespace())
        } else {
            false
        }
    }

    /// Check if this position could be document end (...) - simplified version
    fn is_document_end_candidate_simple(&self) -> bool {
        if let Some(scanner) = &self.zero_scanner {
            scanner.current_char() == Some('.')
                && scanner.peek_char(1) == Some('.')
                && scanner.peek_char(2) == Some('.')
                && scanner.peek_char(3).map_or(true, |c| c.is_whitespace())
        } else {
            false
        }
    }

    /// Check if colon is a value indicator - simplified version
    fn is_value_indicator_simple(&self) -> bool {
        if let Some(scanner) = &self.zero_scanner {
            scanner.current_char() == Some(':')
                && scanner.peek_char(1).map_or(true, |c| c.is_whitespace())
        } else {
            false
        }
    }

    /// Handle document start
    fn handle_document_start(&mut self) {
        self.event_buffer
            .push_back(Event::document_start(self.position, None, vec![], false));
        self.state = ParserState::DocumentStart;
    }

    /// Handle document end
    fn handle_document_end(&mut self) {
        self.event_buffer
            .push_back(Event::document_end(self.position, false));
        self.state = ParserState::DocumentEnd;
    }

    /// Handle flow sequence start
    fn handle_flow_sequence_start(&mut self) {
        self.push_state(ParserState::FlowSequence);
        self.event_buffer.push_back(Event::sequence_start(
            self.position,
            self.pending_anchor.take(),
            self.pending_tag.take(),
            true,
        ));
    }

    /// Handle flow sequence end
    fn handle_flow_sequence_end(&mut self) {
        self.event_buffer
            .push_back(Event::sequence_end(self.position));
        self.pop_state();
    }

    /// Handle flow mapping start
    fn handle_flow_mapping_start(&mut self) {
        self.push_state(ParserState::FlowMapping);
        self.event_buffer.push_back(Event::mapping_start(
            self.position,
            self.pending_anchor.take(),
            self.pending_tag.take(),
            true,
        ));
    }

    /// Handle flow mapping end
    fn handle_flow_mapping_end(&mut self) {
        self.event_buffer
            .push_back(Event::mapping_end(self.position));
        self.pop_state();
    }

    /// Handle value indicator (:)
    fn handle_value_indicator(&mut self) {
        match self.state {
            ParserState::BlockMappingKey => {
                self.state = ParserState::BlockMappingValue;
            }
            ParserState::FlowMapping => {
                // In flow mapping, we don't change state on value indicator
            }
            _ => {
                // Might need to start a new mapping
                // This is simplified - full implementation would be more complex
            }
        }
    }

    /// Handle zero-copy scalar token
    fn handle_zero_copy_scalar(&mut self, token: ZeroToken) -> Result<()> {
        if let ZeroTokenType::Scalar(zero_string, quote_style) = token.token_type {
            // Convert to regular scalar event
            let style = match quote_style {
                crate::scanner::QuoteStyle::Plain => crate::parser::ScalarStyle::Plain,
                crate::scanner::QuoteStyle::Single => crate::parser::ScalarStyle::SingleQuoted,
                crate::scanner::QuoteStyle::Double => crate::parser::ScalarStyle::DoubleQuoted,
            };

            // Avoid allocation if possible
            let value = if zero_string.is_borrowed() {
                zero_string.as_str().to_string()
            } else {
                zero_string.into_owned()
            };

            self.event_buffer.push_back(Event::scalar(
                token.start_position,
                self.pending_anchor.take(),
                self.pending_tag.take(),
                value,
                style == crate::parser::ScalarStyle::Plain,
                style != crate::parser::ScalarStyle::Plain,
                style,
            ));
        }
        Ok(())
    }

    /// Skip comment line - simplified version
    fn skip_comment_simple(&mut self) {
        if let Some(scanner) = &mut self.zero_scanner {
            while let Some(ch) = scanner.current_char() {
                scanner.advance();
                if ch == '\n' || ch == '\r' {
                    break;
                }
            }
        }
    }

    /// Process a token from traditional scanner
    fn process_token(&mut self, token: Token) -> Result<()> {
        self.position = token.end_position;

        match token.token_type {
            TokenType::StreamStart => {
                self.event_buffer
                    .push_back(Event::stream_start(token.start_position));
                self.state = ParserState::ImplicitDocumentStart;
            }
            TokenType::StreamEnd => {
                self.event_buffer
                    .push_back(Event::stream_end(token.start_position));
                self.stream_ended = true;
            }
            TokenType::Scalar(value, quote_style) => {
                let style = match quote_style {
                    crate::scanner::QuoteStyle::Plain => crate::parser::ScalarStyle::Plain,
                    crate::scanner::QuoteStyle::Single => crate::parser::ScalarStyle::SingleQuoted,
                    crate::scanner::QuoteStyle::Double => crate::parser::ScalarStyle::DoubleQuoted,
                };

                self.event_buffer.push_back(Event::scalar(
                    token.start_position,
                    self.pending_anchor.take(),
                    self.pending_tag.take(),
                    value,
                    style == crate::parser::ScalarStyle::Plain,
                    style != crate::parser::ScalarStyle::Plain,
                    style,
                ));
            }
            // Add other token types as needed
            _ => {
                // Simplified implementation - full parser would handle all token types
            }
        }

        Ok(())
    }

    /// Push a new state onto the stack
    fn push_state(&mut self, new_state: ParserState) {
        self.state_stack.push(self.state);
        self.state = new_state;
        self.depth += 1;

        if let Some(ref mut stats) = self.stats {
            stats.max_depth = stats.max_depth.max(self.depth);
        }
    }

    /// Pop state from the stack
    fn pop_state(&mut self) {
        if let Some(prev_state) = self.state_stack.pop() {
            self.state = prev_state;
            self.depth = self.depth.saturating_sub(1);
        }
    }

    /// Update statistics for processed event
    fn update_stats_for_event(&mut self, _event: &Event) {
        if let Some(ref mut stats) = self.stats {
            stats.events_processed += 1;
            stats.max_buffer_size = stats.max_buffer_size.max(self.event_buffer.len());
        }
    }

    /// Get current parsing statistics
    pub fn get_stats(&mut self) -> Option<StreamingStats> {
        if let Some(ref mut stats) = self.stats {
            stats.parse_time_ns = self.start_time.elapsed().as_nanos() as u64;

            if let Some(ref scanner) = self.zero_scanner {
                stats.scanner_stats = Some(scanner.stats());
            }

            Some(stats.clone())
        } else {
            None
        }
    }

    /// Check if more events are available
    pub fn has_more_events(&self) -> bool {
        !self.stream_ended || !self.event_buffer.is_empty()
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.event_buffer.len()
    }
}

impl<'a> Parser for StreamingParser<'a> {
    fn check_event(&self) -> bool {
        !self.event_buffer.is_empty() || !self.stream_ended
    }

    fn peek_event(&self) -> Result<Option<&Event>> {
        Ok(self.event_buffer.front())
    }

    fn get_event(&mut self) -> Result<Option<Event>> {
        self.next_event_internal()
    }

    fn reset(&mut self) {
        self.event_buffer.clear();
        self.state = ParserState::StreamStart;
        self.state_stack.clear();
        self.depth = 0;
        self.pending_anchor = None;
        self.pending_tag = None;
        self.stream_ended = false;
        self.start_time = std::time::Instant::now();

        if let Some(ref mut scanner) = self.scanner {
            scanner.reset();
        }
        if let Some(ref mut scanner) = self.zero_scanner {
            scanner.reset();
        }
    }

    fn position(&self) -> Position {
        self.position
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EventType;

    #[test]
    fn test_streaming_parser_basic() {
        // Use zero-copy parser for this test since traditional implementation is simplified
        let input = "42";
        let config = StreamingConfig {
            use_zero_copy: true,
            collect_stats: true,
            ..Default::default()
        };
        let mut parser = StreamingParser::new_zero_copy(input, config);

        // Should be able to get events
        assert!(parser.check_event());

        // Get multiple batches to ensure we process the scalar
        let mut all_events = Vec::new();
        for _ in 0..5 {
            let batch = parser.next_batch().unwrap();
            if batch.is_empty() {
                break;
            }
            all_events.extend(batch);
        }

        assert!(!all_events.is_empty(), "Should generate at least one event");

        // Should contain a scalar event
        let has_scalar = all_events.iter().any(|e| {
            if let EventType::Scalar { value, .. } = &e.event_type {
                value == "42"
            } else {
                false
            }
        });
        assert!(has_scalar, "Should find scalar event with value '42'");
    }

    #[test]
    fn test_zero_copy_streaming() {
        let input = "key: value";
        let config = StreamingConfig {
            use_zero_copy: true,
            collect_stats: true,
            ..Default::default()
        };

        let mut parser = StreamingParser::new_zero_copy(input, config);

        // Process some events
        let batch = parser.next_batch().unwrap();
        assert!(!batch.is_empty());

        // Should have some statistics
        let stats = parser.get_stats();
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert!(stats.events_processed > 0);
    }

    #[test]
    fn test_streaming_config() {
        let config = StreamingConfig {
            max_buffer_size: 32,
            use_zero_copy: false,
            max_depth: 10,
            collect_stats: true,
        };

        let parser = StreamingParser::new("test".to_string(), config);
        assert_eq!(parser.config.max_buffer_size, 32);
        assert!(!parser.config.use_zero_copy);
        assert_eq!(parser.config.max_depth, 10);
        assert!(parser.config.collect_stats);
    }

    #[test]
    fn test_flow_collections_streaming() {
        let input = "[1, 2, 3]";
        let config = StreamingConfig::default();

        let mut parser = StreamingParser::new_zero_copy(input, config);

        let mut all_events = Vec::new();
        while parser.has_more_events() {
            let batch = parser.next_batch().unwrap();
            if batch.is_empty() {
                break;
            }
            all_events.extend(batch);
        }

        // Should have sequence start and some scalars
        let has_sequence_start = all_events.iter().any(|e| {
            matches!(
                e.event_type,
                EventType::SequenceStart {
                    flow_style: true,
                    ..
                }
            )
        });
        assert!(has_sequence_start, "Should find flow sequence start");
    }
}
