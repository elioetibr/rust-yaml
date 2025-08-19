//! Zero-copy parsing optimizations for YAML processing
//!
//! This module provides data structures and utilities for minimizing allocations
//! during YAML parsing by using string slices where possible instead of owned strings.

use crate::{Position, Result};
use std::borrow::Cow;

/// A zero-copy string that can either borrow from the input or own its data
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ZeroString<'a> {
    data: Cow<'a, str>,
}

impl<'a> ZeroString<'a> {
    /// Create a borrowed zero-copy string
    pub fn borrowed(s: &'a str) -> Self {
        Self {
            data: Cow::Borrowed(s),
        }
    }

    /// Create an owned zero-copy string
    pub fn owned(s: String) -> Self {
        Self {
            data: Cow::Owned(s),
        }
    }

    /// Get the string content as a &str
    pub fn as_str(&self) -> &str {
        &self.data
    }

    /// Convert to owned String
    pub fn into_owned(self) -> String {
        self.data.into_owned()
    }

    /// Check if this is borrowed data
    pub fn is_borrowed(&self) -> bool {
        matches!(self.data, Cow::Borrowed(_))
    }

    /// Get the length in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<'a> From<&'a str> for ZeroString<'a> {
    fn from(s: &'a str) -> Self {
        Self::borrowed(s)
    }
}

impl<'a> From<String> for ZeroString<'a> {
    fn from(s: String) -> Self {
        Self::owned(s)
    }
}

impl<'a> AsRef<str> for ZeroString<'a> {
    fn as_ref(&self) -> &str {
        &self.data
    }
}

/// Zero-copy token types that use string slices where possible
#[derive(Debug, Clone, PartialEq)]
pub enum ZeroTokenType<'a> {
    /// Stream start marker
    StreamStart,
    /// Stream end marker
    StreamEnd,
    /// Document start marker (---)
    DocumentStart,
    /// Document end marker (...)
    DocumentEnd,
    /// Block sequence start ([)
    BlockSequenceStart,
    /// Block mapping start ({)
    BlockMappingStart,
    /// Block end marker
    BlockEnd,
    /// Flow sequence start ([)
    FlowSequenceStart,
    /// Flow sequence end (])
    FlowSequenceEnd,
    /// Flow mapping start ({)
    FlowMappingStart,
    /// Flow mapping end (})
    FlowMappingEnd,
    /// Block entry marker (-)
    BlockEntry,
    /// Flow entry separator (,)
    FlowEntry,
    /// Key marker (?)
    Key,
    /// Value separator (:)
    Value,
    /// Scalar value with quote style
    Scalar(ZeroString<'a>, crate::scanner::QuoteStyle),
    /// Literal block scalar (|)
    BlockScalarLiteral(ZeroString<'a>),
    /// Folded block scalar (>)
    BlockScalarFolded(ZeroString<'a>),
    /// Anchor definition (&name)
    Anchor(ZeroString<'a>),
    /// Alias reference (*name)
    Alias(ZeroString<'a>),
    /// Tag (!tag)
    Tag(ZeroString<'a>),
    /// Comment (# comment)
    Comment(ZeroString<'a>),
}

/// Zero-copy token with position information
#[derive(Debug, Clone, PartialEq)]
pub struct ZeroToken<'a> {
    /// The type of token
    pub token_type: ZeroTokenType<'a>,
    /// Starting position in the input
    pub start_position: Position,
    /// Ending position in the input
    pub end_position: Position,
}

impl<'a> ZeroToken<'a> {
    /// Create a new zero-copy token
    pub fn new(
        token_type: ZeroTokenType<'a>,
        start_position: Position,
        end_position: Position,
    ) -> Self {
        Self {
            token_type,
            start_position,
            end_position,
        }
    }

    /// Create a simple token without data
    pub fn simple(token_type: ZeroTokenType<'a>, position: Position) -> Self {
        Self::new(token_type, position, position)
    }

    /// Convert to owned token (for compatibility)
    pub fn into_owned(self) -> crate::scanner::Token {
        use crate::scanner::{Token, TokenType};

        let token_type = match self.token_type {
            ZeroTokenType::StreamStart => TokenType::StreamStart,
            ZeroTokenType::StreamEnd => TokenType::StreamEnd,
            ZeroTokenType::DocumentStart => TokenType::DocumentStart,
            ZeroTokenType::DocumentEnd => TokenType::DocumentEnd,
            ZeroTokenType::BlockSequenceStart => TokenType::BlockSequenceStart,
            ZeroTokenType::BlockMappingStart => TokenType::BlockMappingStart,
            ZeroTokenType::BlockEnd => TokenType::BlockEnd,
            ZeroTokenType::FlowSequenceStart => TokenType::FlowSequenceStart,
            ZeroTokenType::FlowSequenceEnd => TokenType::FlowSequenceEnd,
            ZeroTokenType::FlowMappingStart => TokenType::FlowMappingStart,
            ZeroTokenType::FlowMappingEnd => TokenType::FlowMappingEnd,
            ZeroTokenType::BlockEntry => TokenType::BlockEntry,
            ZeroTokenType::FlowEntry => TokenType::FlowEntry,
            ZeroTokenType::Key => TokenType::Key,
            ZeroTokenType::Value => TokenType::Value,
            ZeroTokenType::Scalar(s, style) => TokenType::Scalar(s.into_owned(), style),
            ZeroTokenType::BlockScalarLiteral(s) => TokenType::BlockScalarLiteral(s.into_owned()),
            ZeroTokenType::BlockScalarFolded(s) => TokenType::BlockScalarFolded(s.into_owned()),
            ZeroTokenType::Anchor(s) => TokenType::Anchor(s.into_owned()),
            ZeroTokenType::Alias(s) => TokenType::Alias(s.into_owned()),
            ZeroTokenType::Tag(s) => TokenType::Tag(s.into_owned()),
            ZeroTokenType::Comment(s) => TokenType::Comment(s.into_owned()),
        };

        Token::new(token_type, self.start_position, self.end_position)
    }
}

/// Memory pool for token allocation to reduce heap allocations
pub struct TokenPool<'a> {
    /// Pool of reusable tokens
    tokens: Vec<ZeroToken<'a>>,
    /// Current index in the pool
    index: usize,
}

impl<'a> TokenPool<'a> {
    /// Create a new token pool with initial capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tokens: Vec::with_capacity(capacity),
            index: 0,
        }
    }

    /// Get a token from the pool or create a new one
    pub fn get_token(&mut self) -> &mut ZeroToken<'a> {
        if self.index >= self.tokens.len() {
            // Need to allocate a new token
            self.tokens.push(ZeroToken::simple(
                ZeroTokenType::StreamStart,
                Position::start(),
            ));
        }

        let token = &mut self.tokens[self.index];
        self.index += 1;
        token
    }

    /// Reset the pool for reuse
    pub fn reset(&mut self) {
        self.index = 0;
    }

    /// Get the number of tokens currently allocated
    pub fn allocated_count(&self) -> usize {
        self.tokens.len()
    }

    /// Get the number of tokens currently in use
    pub fn used_count(&self) -> usize {
        self.index
    }
}

/// Zero-copy string scanner that operates on slices
pub struct ZeroScanner<'a> {
    /// Reference to the input string
    input: &'a str,
    /// Current position in the input
    pub position: Position,
    /// Current character index
    char_index: usize,
    /// Cached character indices for faster access
    char_indices: Vec<(usize, char)>,
    /// Token pool for allocation optimization
    token_pool: TokenPool<'a>,
}

impl<'a> ZeroScanner<'a> {
    /// Create a new zero-copy scanner
    pub fn new(input: &'a str) -> Self {
        let char_indices: Vec<(usize, char)> = input.char_indices().collect();

        Self {
            input,
            position: Position::start(),
            char_index: 0,
            char_indices,
            token_pool: TokenPool::with_capacity(128), // Start with reasonable capacity
        }
    }

    /// Get the current character
    pub fn current_char(&self) -> Option<char> {
        self.char_indices.get(self.char_index).map(|(_, ch)| *ch)
    }

    /// Advance to the next character
    pub fn advance(&mut self) -> Option<char> {
        if let Some((_byte_index, ch)) = self.char_indices.get(self.char_index) {
            self.position = self.position.advance(*ch);
            self.char_index += 1;
            self.char_indices.get(self.char_index).map(|(_, ch)| *ch)
        } else {
            None
        }
    }

    /// Peek at a character at the given offset
    pub fn peek_char(&self, offset: isize) -> Option<char> {
        if offset >= 0 {
            let index = self.char_index + offset as usize;
            self.char_indices.get(index).map(|(_, ch)| *ch)
        } else {
            let offset_abs = (-offset) as usize;
            if self.char_index >= offset_abs {
                let index = self.char_index - offset_abs;
                self.char_indices.get(index).map(|(_, ch)| *ch)
            } else {
                None
            }
        }
    }

    /// Get a slice of the input from start position to current position
    pub fn slice_from(&self, start_position: Position) -> Result<&'a str> {
        let start_byte = start_position.index;
        let end_byte = self.position.index;

        if start_byte <= end_byte && end_byte <= self.input.len() {
            Ok(&self.input[start_byte..end_byte])
        } else {
            Err(crate::Error::parse(
                self.position,
                "Invalid slice bounds".to_string(),
            ))
        }
    }

    /// Get a slice between two positions
    pub fn slice_between(&self, start: Position, end: Position) -> Result<&'a str> {
        let start_byte = start.index;
        let end_byte = end.index;

        if start_byte <= end_byte && end_byte <= self.input.len() {
            Ok(&self.input[start_byte..end_byte])
        } else {
            Err(crate::Error::parse(
                self.position,
                "Invalid slice bounds".to_string(),
            ))
        }
    }

    /// Reset the scanner to the beginning
    pub fn reset(&mut self) {
        self.position = Position::start();
        self.char_index = 0;
        self.token_pool.reset();
    }

    /// Get scanner statistics for performance monitoring
    pub fn stats(&self) -> ScannerStats {
        ScannerStats {
            input_length: self.input.len(),
            chars_processed: self.char_index,
            tokens_allocated: self.token_pool.allocated_count(),
            tokens_used: self.token_pool.used_count(),
            position: self.position,
        }
    }

    /// Scan a plain scalar using zero-copy slicing
    pub fn scan_plain_scalar_zero_copy(&mut self) -> Result<ZeroToken<'a>> {
        let start_pos = self.position;

        // Find the end of the scalar without allocating
        while let Some(ch) = self.current_char() {
            // Stop at structural characters (same logic as regular scanner)
            match ch {
                '\n' | '\r' => break,
                ':' if self.peek_char(1).map_or(true, |c| c.is_whitespace()) => break,
                '#' if self.char_index == 0
                    || self.peek_char(-1).map_or(false, |c| c.is_whitespace()) =>
                {
                    break;
                }
                ',' | '[' | ']' | '{' | '}' => break,
                _ => {
                    self.advance();
                }
            }
        }

        // Get the slice without allocation
        let slice = self.slice_from(start_pos)?;
        let trimmed_slice = slice.trim_end();

        // Use borrowed string if possible
        let zero_string = if trimmed_slice.len() == slice.len() {
            // No trimming needed, can use borrowed slice directly
            ZeroString::borrowed(trimmed_slice)
        } else {
            // Need to allocate for trimmed version
            ZeroString::owned(trimmed_slice.to_string())
        };

        Ok(ZeroToken::new(
            ZeroTokenType::Scalar(zero_string, crate::scanner::QuoteStyle::Plain),
            start_pos,
            self.position,
        ))
    }

    /// Scan a simple identifier using zero-copy slicing (for anchors/aliases)
    pub fn scan_identifier_zero_copy(&mut self) -> Result<ZeroString<'a>> {
        let start_pos = self.position;

        // Scan identifier characters
        while let Some(ch) = self.current_char() {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                self.advance();
            } else {
                break;
            }
        }

        let slice = self.slice_from(start_pos)?;
        Ok(ZeroString::borrowed(slice))
    }

    /// Skip whitespace efficiently
    pub fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char() {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }
}

/// Statistics about scanner performance
#[derive(Debug, Clone)]
pub struct ScannerStats {
    /// Total input length in bytes
    pub input_length: usize,
    /// Number of characters processed
    pub chars_processed: usize,
    /// Number of tokens allocated in the pool
    pub tokens_allocated: usize,
    /// Number of tokens currently used
    pub tokens_used: usize,
    /// Current position
    pub position: Position,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_string_borrowed() {
        let s = "hello world";
        let zs = ZeroString::borrowed(s);

        assert!(zs.is_borrowed());
        assert_eq!(zs.as_str(), "hello world");
        assert_eq!(zs.len(), 11);
        assert!(!zs.is_empty());
    }

    #[test]
    fn test_zero_string_owned() {
        let s = String::from("hello world");
        let zs = ZeroString::owned(s);

        assert!(!zs.is_borrowed());
        assert_eq!(zs.as_str(), "hello world");
        assert_eq!(zs.len(), 11);
    }

    #[test]
    fn test_zero_scanner_basic() {
        let input = "hello: world";
        let mut scanner = ZeroScanner::new(input);

        assert_eq!(scanner.current_char(), Some('h'));
        assert_eq!(scanner.advance(), Some('e'));
        assert_eq!(scanner.current_char(), Some('e'));

        // Test peeking
        assert_eq!(scanner.peek_char(1), Some('l'));
        assert_eq!(scanner.peek_char(-1), Some('h'));
    }

    #[test]
    fn test_zero_scanner_slicing() {
        let input = "hello: world";
        let mut scanner = ZeroScanner::new(input);

        let start = scanner.position;

        // Advance past "hello"
        for _ in 0..5 {
            scanner.advance();
        }

        let slice = scanner.slice_from(start).unwrap();
        assert_eq!(slice, "hello");
    }

    #[test]
    fn test_token_pool() {
        let mut pool = TokenPool::with_capacity(2);

        assert_eq!(pool.allocated_count(), 0);
        assert_eq!(pool.used_count(), 0);

        let _token1 = pool.get_token();
        assert_eq!(pool.allocated_count(), 1);
        assert_eq!(pool.used_count(), 1);

        let _token2 = pool.get_token();
        assert_eq!(pool.allocated_count(), 2);
        assert_eq!(pool.used_count(), 2);

        pool.reset();
        assert_eq!(pool.allocated_count(), 2); // Still allocated
        assert_eq!(pool.used_count(), 0); // But not in use
    }

    #[test]
    fn test_zero_copy_scalar_scanning() {
        let input = "hello world: test";
        let mut scanner = ZeroScanner::new(input);

        let token = scanner.scan_plain_scalar_zero_copy().unwrap();

        if let ZeroTokenType::Scalar(value, _) = token.token_type {
            assert_eq!(value.as_str(), "hello world");
            assert!(value.is_borrowed()); // Should be zero-copy
        } else {
            panic!("Expected scalar token");
        }
    }

    #[test]
    fn test_zero_copy_identifier_scanning() {
        let input = "my_anchor_123 ";
        let mut scanner = ZeroScanner::new(input);

        let identifier = scanner.scan_identifier_zero_copy().unwrap();
        assert_eq!(identifier.as_str(), "my_anchor_123");
        assert!(identifier.is_borrowed()); // Should be zero-copy
    }

    #[test]
    fn test_zero_copy_trimming() {
        let input = "hello   \n";
        let mut scanner = ZeroScanner::new(input);

        let token = scanner.scan_plain_scalar_zero_copy().unwrap();

        if let ZeroTokenType::Scalar(value, _) = token.token_type {
            assert_eq!(value.as_str(), "hello");
            // Should be owned because trimming was needed
            assert!(!value.is_borrowed());
        } else {
            panic!("Expected scalar token");
        }
    }
}
