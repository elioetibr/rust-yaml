//! YAML token definitions

use crate::Position;
use std::fmt;

/// Quote style for string scalars
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuoteStyle {
    /// No quotes (plain scalar)
    Plain,
    /// Single quotes ('string')
    Single,
    /// Double quotes ("string")
    Double,
}

/// Represents a YAML token with position information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The type of token
    pub token_type: TokenType,
    /// Start position of the token
    pub start_position: Position,
    /// End position of the token
    pub end_position: Position,
}

/// Types of YAML tokens
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenType {
    // Stream tokens
    /// Start of stream
    StreamStart,
    /// End of stream
    StreamEnd,

    // Document tokens
    /// Document start marker (---)
    DocumentStart,
    /// Document end marker (...)
    DocumentEnd,

    // Directive tokens
    /// YAML version directive (%YAML)
    YamlDirective(u8, u8), // major, minor version
    /// Tag directive (%TAG)
    TagDirective(String, String), // handle, prefix

    // Block structure tokens
    /// Block sequence start
    BlockSequenceStart,
    /// Block mapping start
    BlockMappingStart,
    /// Block end
    BlockEnd,

    // Flow structure tokens
    /// Flow sequence start ([)
    FlowSequenceStart,
    /// Flow sequence end (])
    FlowSequenceEnd,
    /// Flow mapping start ({)
    FlowMappingStart,
    /// Flow mapping end (})
    FlowMappingEnd,

    // Entry tokens
    /// Block entry (-)
    BlockEntry,
    /// Flow entry (,)
    FlowEntry,

    // Key-value tokens
    /// Key indicator (? in complex keys)
    Key,
    /// Value indicator (:)
    Value,

    // Scalar tokens
    /// Scalar value with quote style
    Scalar(String, QuoteStyle),
    /// Literal block scalar (|)
    BlockScalarLiteral(String),
    /// Folded block scalar (>)
    BlockScalarFolded(String),

    // Reference tokens
    /// Alias (*name)
    Alias(String),
    /// Anchor (&name)
    Anchor(String),

    // Tag token
    /// Tag (!tag or !!tag)
    Tag(String),

    // Comment token (for round-trip support)
    /// Comment (# text)
    Comment(String),
}

impl Token {
    /// Create a new token
    pub const fn new(
        token_type: TokenType,
        start_position: Position,
        end_position: Position,
    ) -> Self {
        Self {
            token_type,
            start_position,
            end_position,
        }
    }

    /// Create a simple token at a single position
    pub const fn simple(token_type: TokenType, position: Position) -> Self {
        Self::new(token_type, position, position)
    }

    /// Get the token type
    pub const fn token_type(&self) -> &TokenType {
        &self.token_type
    }

    /// Get the start position
    pub const fn start_position(&self) -> Position {
        self.start_position
    }

    /// Get the end position
    pub const fn end_position(&self) -> Position {
        self.end_position
    }

    /// Check if this is a scalar token
    pub const fn is_scalar(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::Scalar(_, _)
                | TokenType::BlockScalarLiteral(_)
                | TokenType::BlockScalarFolded(_)
        )
    }

    /// Get scalar value if this is a scalar token
    pub fn as_scalar(&self) -> Option<&str> {
        match &self.token_type {
            TokenType::Scalar(s, _)
            | TokenType::BlockScalarLiteral(s)
            | TokenType::BlockScalarFolded(s) => Some(s),
            _ => None,
        }
    }

    /// Get scalar value and quote style if this is a scalar token
    pub fn as_scalar_with_style(&self) -> Option<(&str, QuoteStyle)> {
        match &self.token_type {
            TokenType::Scalar(s, style) => Some((s, style.clone())),
            TokenType::BlockScalarLiteral(s) => Some((s, QuoteStyle::Plain)), // Block scalars are considered plain
            TokenType::BlockScalarFolded(s) => Some((s, QuoteStyle::Plain)), // Block scalars are considered plain
            _ => None,
        }
    }

    /// Check if this is a flow collection start token
    pub const fn is_flow_collection_start(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::FlowSequenceStart | TokenType::FlowMappingStart
        )
    }

    /// Check if this is a flow collection end token
    pub const fn is_flow_collection_end(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::FlowSequenceEnd | TokenType::FlowMappingEnd
        )
    }

    /// Check if this is a block collection start token
    pub const fn is_block_collection_start(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::BlockSequenceStart | TokenType::BlockMappingStart
        )
    }

    /// Check if this is a document boundary token
    pub const fn is_document_boundary(&self) -> bool {
        matches!(
            self.token_type,
            TokenType::DocumentStart | TokenType::DocumentEnd
        )
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.token_type {
            TokenType::StreamStart => write!(f, "STREAM-START"),
            TokenType::StreamEnd => write!(f, "STREAM-END"),
            TokenType::DocumentStart => write!(f, "DOCUMENT-START"),
            TokenType::DocumentEnd => write!(f, "DOCUMENT-END"),
            TokenType::BlockSequenceStart => write!(f, "BLOCK-SEQUENCE-START"),
            TokenType::BlockMappingStart => write!(f, "BLOCK-MAPPING-START"),
            TokenType::BlockEnd => write!(f, "BLOCK-END"),
            TokenType::FlowSequenceStart => write!(f, "FLOW-SEQUENCE-START"),
            TokenType::FlowSequenceEnd => write!(f, "FLOW-SEQUENCE-END"),
            TokenType::FlowMappingStart => write!(f, "FLOW-MAPPING-START"),
            TokenType::FlowMappingEnd => write!(f, "FLOW-MAPPING-END"),
            TokenType::BlockEntry => write!(f, "BLOCK-ENTRY"),
            TokenType::FlowEntry => write!(f, "FLOW-ENTRY"),
            TokenType::Key => write!(f, "KEY"),
            TokenType::Value => write!(f, "VALUE"),
            TokenType::Scalar(s, style) => write!(f, "SCALAR({}, {:?})", s, style),
            TokenType::BlockScalarLiteral(s) => write!(f, "LITERAL({})", s),
            TokenType::BlockScalarFolded(s) => write!(f, "FOLDED({})", s),
            TokenType::Alias(name) => write!(f, "ALIAS({})", name),
            TokenType::Anchor(name) => write!(f, "ANCHOR({})", name),
            TokenType::Tag(tag) => write!(f, "TAG({})", tag),
            TokenType::Comment(text) => write!(f, "COMMENT({})", text),
            TokenType::YamlDirective(major, minor) => {
                write!(f, "YAML-DIRECTIVE({}.{})", major, minor)
            }
            TokenType::TagDirective(handle, prefix) => {
                write!(f, "TAG-DIRECTIVE({}, {})", handle, prefix)
            }
        }
    }
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StreamStart => write!(f, "StreamStart"),
            Self::StreamEnd => write!(f, "StreamEnd"),
            Self::DocumentStart => write!(f, "DocumentStart"),
            Self::DocumentEnd => write!(f, "DocumentEnd"),
            Self::BlockSequenceStart => write!(f, "BlockSequenceStart"),
            Self::BlockMappingStart => write!(f, "BlockMappingStart"),
            Self::BlockEnd => write!(f, "BlockEnd"),
            Self::FlowSequenceStart => write!(f, "FlowSequenceStart"),
            Self::FlowSequenceEnd => write!(f, "FlowSequenceEnd"),
            Self::FlowMappingStart => write!(f, "FlowMappingStart"),
            Self::FlowMappingEnd => write!(f, "FlowMappingEnd"),
            Self::BlockEntry => write!(f, "BlockEntry"),
            Self::FlowEntry => write!(f, "FlowEntry"),
            Self::Key => write!(f, "Key"),
            Self::Value => write!(f, "Value"),
            Self::Scalar(s, style) => write!(f, "Scalar({}, {:?})", s, style),
            Self::BlockScalarLiteral(s) => write!(f, "BlockScalarLiteral({})", s),
            Self::BlockScalarFolded(s) => write!(f, "BlockScalarFolded({})", s),
            Self::Alias(name) => write!(f, "Alias({})", name),
            Self::Anchor(name) => write!(f, "Anchor({})", name),
            Self::Tag(tag) => write!(f, "Tag({})", tag),
            Self::Comment(text) => write!(f, "Comment({})", text),
            Self::YamlDirective(major, minor) => write!(f, "YamlDirective({}.{})", major, minor),
            Self::TagDirective(handle, prefix) => write!(f, "TagDirective({}, {})", handle, prefix),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let pos1 = Position::at(1, 1, 0);
        let pos2 = Position::at(1, 5, 4);

        let token = Token::new(
            TokenType::Scalar("hello".to_string(), QuoteStyle::Plain),
            pos1,
            pos2,
        );

        assert_eq!(token.start_position(), pos1);
        assert_eq!(token.end_position(), pos2);
        assert!(token.is_scalar());
        assert_eq!(token.as_scalar(), Some("hello"));
    }

    #[test]
    fn test_token_type_checks() {
        let scalar_token = Token::simple(
            TokenType::Scalar("test".to_string(), QuoteStyle::Plain),
            Position::start(),
        );
        let flow_start = Token::simple(TokenType::FlowSequenceStart, Position::start());
        let doc_start = Token::simple(TokenType::DocumentStart, Position::start());

        assert!(scalar_token.is_scalar());
        assert!(!scalar_token.is_flow_collection_start());

        assert!(flow_start.is_flow_collection_start());
        assert!(!flow_start.is_scalar());

        assert!(doc_start.is_document_boundary());
        assert!(!doc_start.is_scalar());
    }

    #[test]
    fn test_token_display() {
        let scalar = Token::simple(
            TokenType::Scalar("hello".to_string(), QuoteStyle::Plain),
            Position::start(),
        );
        assert_eq!(format!("{}", scalar), "SCALAR(hello, Plain)");

        let flow_start = Token::simple(TokenType::FlowSequenceStart, Position::start());
        assert_eq!(format!("{}", flow_start), "FLOW-SEQUENCE-START");
    }

    #[test]
    fn test_token_type_display() {
        assert_eq!(format!("{}", TokenType::StreamStart), "StreamStart");
        assert_eq!(
            format!(
                "{}",
                TokenType::Scalar("test".to_string(), QuoteStyle::Plain)
            ),
            "Scalar(test, Plain)"
        );
        assert_eq!(
            format!("{}", TokenType::FlowSequenceStart),
            "FlowSequenceStart"
        );
    }
}
