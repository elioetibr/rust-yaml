//! Token processing and generation for YAML scanner

use super::{QuoteStyle, Token, TokenType};
use crate::Position;

/// Token generation helper functions
pub(super) struct TokenProcessor;

impl TokenProcessor {
    /// Create a simple token without data
    pub fn simple_token(token_type: TokenType, position: Position) -> Token {
        Token::simple(token_type, position)
    }

    /// Create a scalar token
    pub fn scalar_token(value: String, quote_style: QuoteStyle, position: Position) -> Token {
        Token::new(TokenType::Scalar(value, quote_style), position, position)
    }

    /// Create an anchor token
    pub fn anchor_token(name: String, position: Position) -> Token {
        Token::new(TokenType::Anchor(name), position, position)
    }

    /// Create an alias token
    pub fn alias_token(name: String, position: Position) -> Token {
        Token::new(TokenType::Alias(name), position, position)
    }

    /// Create a tag token
    pub fn tag_token(tag: String, position: Position) -> Token {
        Token::new(TokenType::Tag(tag), position, position)
    }

    /// Create a comment token
    pub fn comment_token(comment: String, position: Position) -> Token {
        Token::new(TokenType::Comment(comment), position, position)
    }

    /// Create a literal block scalar token
    pub fn literal_block_token(value: String, position: Position) -> Token {
        Token::new(TokenType::BlockScalarLiteral(value), position, position)
    }

    /// Create a folded block scalar token
    pub fn folded_block_token(value: String, position: Position) -> Token {
        Token::new(TokenType::BlockScalarFolded(value), position, position)
    }
}

/// Character classification helpers
pub(super) struct CharClassifier;

impl CharClassifier {
    /// Check if character is a flow indicator
    pub fn is_flow_indicator(ch: char) -> bool {
        matches!(ch, '[' | ']' | '{' | '}' | ',' | ':')
    }

    /// Check if character can start an identifier
    pub fn is_identifier_start(ch: char) -> bool {
        ch.is_ascii_alphabetic() || ch == '_'
    }

    /// Check if character can be in an identifier
    pub fn is_identifier_char(ch: char) -> bool {
        ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_')
    }

    /// Check if character is a document indicator
    pub fn is_document_indicator(ch: char) -> bool {
        matches!(ch, '-' | '.')
    }

    /// Check if character is whitespace (YAML definition)
    pub fn is_yaml_whitespace(ch: char) -> bool {
        matches!(ch, ' ' | '\t')
    }

    /// Check if character is a line break
    pub fn is_line_break(ch: char) -> bool {
        matches!(ch, '\n' | '\r')
    }

    /// Check if character is printable ASCII
    pub fn is_printable_ascii(ch: char) -> bool {
        ch.is_ascii() && !ch.is_ascii_control() || ch == '\t'
    }

    /// Check if character is a digit
    pub fn is_digit(ch: char) -> bool {
        ch.is_ascii_digit()
    }

    /// Check if character is hex digit
    pub fn is_hex_digit(ch: char) -> bool {
        ch.is_ascii_hexdigit()
    }

    /// Check if character is octal digit
    pub fn is_octal_digit(ch: char) -> bool {
        matches!(ch, '0'..='7')
    }
}

/// Pattern matching helpers
pub(super) struct PatternMatcher;

impl PatternMatcher {
    /// Check for document start pattern (---)
    pub fn is_document_start(input: &str, pos: usize) -> bool {
        let chars: Vec<char> = input.chars().collect();
        if pos + 2 >= chars.len() {
            return false;
        }

        chars[pos] == '-'
            && chars[pos + 1] == '-'
            && chars[pos + 2] == '-'
            && (pos + 3 >= chars.len()
                || CharClassifier::is_yaml_whitespace(chars[pos + 3])
                || CharClassifier::is_line_break(chars[pos + 3]))
    }

    /// Check for document end pattern (...)
    pub fn is_document_end(input: &str, pos: usize) -> bool {
        let chars: Vec<char> = input.chars().collect();
        if pos + 2 >= chars.len() {
            return false;
        }

        chars[pos] == '.'
            && chars[pos + 1] == '.'
            && chars[pos + 2] == '.'
            && (pos + 3 >= chars.len()
                || CharClassifier::is_yaml_whitespace(chars[pos + 3])
                || CharClassifier::is_line_break(chars[pos + 3]))
    }

    /// Check if we're at the start of a tag
    pub fn is_tag_start(ch: char) -> bool {
        ch == '!'
    }

    /// Check if we're at the start of an anchor
    pub fn is_anchor_start(ch: char) -> bool {
        ch == '&'
    }

    /// Check if we're at the start of an alias
    pub fn is_alias_start(ch: char) -> bool {
        ch == '*'
    }

    /// Check if we're at the start of a comment
    pub fn is_comment_start(ch: char) -> bool {
        ch == '#'
    }
}
