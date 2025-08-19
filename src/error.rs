//! Error types for YAML processing

use crate::Position;
use std::fmt;

/// Result type alias for YAML operations
pub type Result<T> = std::result::Result<T, Error>;

/// Context information for error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorContext {
    /// The problematic line content
    pub line_content: String,
    /// Position within the line where the error occurred
    pub column_position: usize,
    /// Optional suggestion for fixing the error
    pub suggestion: Option<String>,
    /// Additional context lines (before and after)
    pub surrounding_lines: Vec<(usize, String)>,
}

impl ErrorContext {
    /// Create a new error context
    pub const fn new(line_content: String, column_position: usize) -> Self {
        Self {
            line_content,
            column_position,
            suggestion: None,
            surrounding_lines: Vec::new(),
        }
    }

    /// Add a suggestion for fixing the error
    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    /// Add surrounding lines for context
    pub fn with_surrounding_lines(mut self, lines: Vec<(usize, String)>) -> Self {
        self.surrounding_lines = lines;
        self
    }

    /// Create error context from input text and position
    pub fn from_input(input: &str, position: &Position, context_lines: usize) -> Self {
        let lines: Vec<&str> = input.lines().collect();
        let line_index = position.line.saturating_sub(1);

        let line_content = lines
            .get(line_index)
            .map(|s| s.to_string())
            .unwrap_or_else(|| "<EOF>".to_string());

        let mut surrounding_lines = Vec::new();

        // Add context lines before
        let start = line_index.saturating_sub(context_lines);
        for i in start..line_index {
            if let Some(line) = lines.get(i) {
                surrounding_lines.push((i + 1, line.to_string()));
            }
        }

        // Add context lines after
        let end = (line_index + context_lines + 1).min(lines.len());
        for i in (line_index + 1)..end {
            if let Some(line) = lines.get(i) {
                surrounding_lines.push((i + 1, line.to_string()));
            }
        }

        Self {
            line_content,
            column_position: position.column,
            suggestion: None,
            surrounding_lines,
        }
    }
}

/// Comprehensive error type for YAML processing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Parsing errors with position information
    Parse {
        /// Position where error occurred
        position: Position,
        /// Error message
        message: String,
        /// Additional context for better error reporting
        context: Option<ErrorContext>,
    },

    /// Scanning errors during tokenization
    Scan {
        /// Position where error occurred
        position: Position,
        /// Error message
        message: String,
        /// Additional context for better error reporting
        context: Option<ErrorContext>,
    },

    /// Construction errors when building objects
    Construction {
        /// Position where error occurred
        position: Position,
        /// Error message
        message: String,
        /// Additional context for better error reporting
        context: Option<ErrorContext>,
    },

    /// Emission errors during output generation
    Emission {
        /// Error message
        message: String,
    },

    /// IO errors (simplified for clonability)
    Io {
        /// Error kind
        kind: std::io::ErrorKind,
        /// Error message
        message: String,
    },

    /// UTF-8 encoding errors
    Utf8 {
        /// Error message
        message: String,
    },

    /// Type conversion errors
    Type {
        /// Expected type
        expected: String,
        /// Found type
        found: String,
        /// Position where error occurred
        position: Position,
        /// Additional context for better error reporting
        context: Option<ErrorContext>,
    },

    /// Value errors for invalid values
    Value {
        /// Position where error occurred
        position: Position,
        /// Error message
        message: String,
        /// Additional context for better error reporting
        context: Option<ErrorContext>,
    },

    /// Configuration errors
    Config {
        /// Error message
        message: String,
    },

    /// Multiple related errors
    Multiple {
        /// List of related errors
        errors: Vec<Error>,
        /// Context message
        message: String,
    },

    /// Resource limit exceeded
    LimitExceeded {
        /// Error message describing which limit was exceeded
        message: String,
    },

    /// Indentation errors
    Indentation {
        /// Position where error occurred
        position: Position,
        /// Expected indentation level
        expected: usize,
        /// Found indentation level
        found: usize,
        /// Additional context
        context: Option<ErrorContext>,
    },

    /// Invalid character or sequence
    InvalidCharacter {
        /// Position where error occurred
        position: Position,
        /// The invalid character
        character: char,
        /// Context where it was found
        context_description: String,
        /// Additional context
        context: Option<ErrorContext>,
    },

    /// Unclosed delimiter (quote, bracket, etc.)
    UnclosedDelimiter {
        /// Position where delimiter started
        start_position: Position,
        /// Position where EOF or mismatch was found
        current_position: Position,
        /// Type of delimiter
        delimiter_type: String,
        /// Additional context
        context: Option<ErrorContext>,
    },
}

impl Error {
    /// Create a new parse error
    pub fn parse(position: Position, message: impl Into<String>) -> Self {
        Self::Parse {
            position,
            message: message.into(),
            context: None,
        }
    }

    /// Create a new parse error with context
    pub fn parse_with_context(
        position: Position,
        message: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::Parse {
            position,
            message: message.into(),
            context: Some(context),
        }
    }

    /// Create a new scan error
    pub fn scan(position: Position, message: impl Into<String>) -> Self {
        Self::Scan {
            position,
            message: message.into(),
            context: None,
        }
    }

    /// Create a new scan error with context
    pub fn scan_with_context(
        position: Position,
        message: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::Scan {
            position,
            message: message.into(),
            context: Some(context),
        }
    }

    /// Create a new construction error
    pub fn construction(position: Position, message: impl Into<String>) -> Self {
        Self::Construction {
            position,
            message: message.into(),
            context: None,
        }
    }

    /// Create a new construction error with context
    pub fn construction_with_context(
        position: Position,
        message: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::Construction {
            position,
            message: message.into(),
            context: Some(context),
        }
    }

    /// Create a new emission error
    pub fn emission(message: impl Into<String>) -> Self {
        Self::Emission {
            message: message.into(),
        }
    }

    /// Create a new limit exceeded error
    pub fn limit_exceeded(message: impl Into<String>) -> Self {
        Self::LimitExceeded {
            message: message.into(),
        }
    }

    /// Create a new type error
    pub fn type_error(
        position: Position,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        Self::Type {
            expected: expected.into(),
            found: found.into(),
            position,
            context: None,
        }
    }

    /// Create a new type error with context
    pub fn type_error_with_context(
        position: Position,
        expected: impl Into<String>,
        found: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::Type {
            expected: expected.into(),
            found: found.into(),
            position,
            context: Some(context),
        }
    }

    /// Create a new value error
    pub fn value_error(position: Position, message: impl Into<String>) -> Self {
        Self::Value {
            position,
            message: message.into(),
            context: None,
        }
    }

    /// Create a new value error with context
    pub fn value_error_with_context(
        position: Position,
        message: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::Value {
            position,
            message: message.into(),
            context: Some(context),
        }
    }

    /// Create a new configuration error
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Legacy alias for config_error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a multiple error with related errors
    pub fn multiple(errors: Vec<Self>, message: impl Into<String>) -> Self {
        Self::Multiple {
            errors,
            message: message.into(),
        }
    }

    /// Create an indentation error
    pub const fn indentation(position: Position, expected: usize, found: usize) -> Self {
        Self::Indentation {
            position,
            expected,
            found,
            context: None,
        }
    }

    /// Create an indentation error with context
    pub const fn indentation_with_context(
        position: Position,
        expected: usize,
        found: usize,
        context: ErrorContext,
    ) -> Self {
        Self::Indentation {
            position,
            expected,
            found,
            context: Some(context),
        }
    }

    /// Create an invalid character error
    pub fn invalid_character(
        position: Position,
        character: char,
        context_description: impl Into<String>,
    ) -> Self {
        Self::InvalidCharacter {
            position,
            character,
            context_description: context_description.into(),
            context: None,
        }
    }

    /// Create an invalid character error with context
    pub fn invalid_character_with_context(
        position: Position,
        character: char,
        context_description: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::InvalidCharacter {
            position,
            character,
            context_description: context_description.into(),
            context: Some(context),
        }
    }

    /// Create an unclosed delimiter error
    pub fn unclosed_delimiter(
        start_position: Position,
        current_position: Position,
        delimiter_type: impl Into<String>,
    ) -> Self {
        Self::UnclosedDelimiter {
            start_position,
            current_position,
            delimiter_type: delimiter_type.into(),
            context: None,
        }
    }

    /// Create an unclosed delimiter error with context
    pub fn unclosed_delimiter_with_context(
        start_position: Position,
        current_position: Position,
        delimiter_type: impl Into<String>,
        context: ErrorContext,
    ) -> Self {
        Self::UnclosedDelimiter {
            start_position,
            current_position,
            delimiter_type: delimiter_type.into(),
            context: Some(context),
        }
    }

    /// Get the position associated with this error, if any
    pub const fn position(&self) -> Option<&Position> {
        match self {
            Self::Parse { position, .. }
            | Self::Scan { position, .. }
            | Self::Construction { position, .. }
            | Self::Type { position, .. }
            | Self::Value { position, .. }
            | Self::Indentation { position, .. }
            | Self::InvalidCharacter { position, .. } => Some(position),
            Self::UnclosedDelimiter {
                current_position, ..
            } => Some(current_position),
            Self::Emission { .. }
            | Self::Io { .. }
            | Self::Utf8 { .. }
            | Self::Config { .. }
            | Self::Multiple { .. }
            | Self::LimitExceeded { .. } => None,
        }
    }

    /// Get the context associated with this error, if any
    pub const fn context(&self) -> Option<&ErrorContext> {
        match self {
            Self::Parse { context, .. }
            | Self::Scan { context, .. }
            | Self::Construction { context, .. }
            | Self::Type { context, .. }
            | Self::Value { context, .. }
            | Self::Indentation { context, .. }
            | Self::InvalidCharacter { context, .. }
            | Self::UnclosedDelimiter { context, .. } => context.as_ref(),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Self::Utf8 {
            message: err.to_string(),
        }
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(err: std::string::FromUtf8Error) -> Self {
        Self::Utf8 {
            message: err.to_string(),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    /// Format error with enhanced context display
    fn format_with_context(
        &self,
        f: &mut fmt::Formatter<'_>,
        position: &Position,
        message: &str,
        context: Option<&ErrorContext>,
    ) -> fmt::Result {
        // Write the main error message
        writeln!(
            f,
            "Error at line {}, column {}: {}",
            position.line, position.column, message
        )?;

        // Add context if available
        if let Some(ctx) = context {
            writeln!(f)?;

            // Show surrounding lines for context
            for (line_num, line_content) in &ctx.surrounding_lines {
                writeln!(f, "{:4} | {}", line_num, line_content)?;
            }

            // Show the problematic line with pointer
            writeln!(f, "{:4} | {}", position.line, ctx.line_content)?;
            write!(f, "     | ")?;
            for _ in 0..ctx.column_position.saturating_sub(1) {
                write!(f, " ")?;
            }
            writeln!(f, "^ here")?;

            // Show suggestion if available
            if let Some(suggestion) = &ctx.suggestion {
                writeln!(f)?;
                writeln!(f, "Suggestion: {}", suggestion)?;
            }
        }

        Ok(())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse {
                position,
                message,
                context,
            } => self.format_with_context(f, position, message, context.as_ref()),
            Self::Scan {
                position,
                message,
                context,
            } => self.format_with_context(
                f,
                position,
                &format!("Scan error: {}", message),
                context.as_ref(),
            ),
            Self::Construction {
                position,
                message,
                context,
            } => self.format_with_context(
                f,
                position,
                &format!("Construction error: {}", message),
                context.as_ref(),
            ),
            Self::Type {
                expected,
                found,
                position,
                context,
            } => {
                let msg = format!("Type error: expected {}, found {}", expected, found);
                self.format_with_context(f, position, &msg, context.as_ref())
            }
            Self::Value {
                position,
                message,
                context,
            } => self.format_with_context(
                f,
                position,
                &format!("Value error: {}", message),
                context.as_ref(),
            ),
            Self::Indentation {
                position,
                expected,
                found,
                context,
            } => {
                let msg = format!(
                    "Indentation error: expected {} spaces, found {}",
                    expected, found
                );
                self.format_with_context(f, position, &msg, context.as_ref())
            }
            Self::InvalidCharacter {
                position,
                character,
                context_description,
                context,
            } => {
                let msg = format!(
                    "Invalid character '{}' in {}",
                    character, context_description
                );
                self.format_with_context(f, position, &msg, context.as_ref())
            }
            Self::UnclosedDelimiter {
                start_position,
                current_position,
                delimiter_type,
                context,
            } => {
                let msg = format!(
                    "Unclosed {} starting at line {}, column {}",
                    delimiter_type, start_position.line, start_position.column
                );
                self.format_with_context(f, current_position, &msg, context.as_ref())
            }
            Self::Multiple { errors, message } => {
                writeln!(f, "Multiple errors: {}", message)?;
                for (i, error) in errors.iter().enumerate() {
                    writeln!(f, "  {}. {}", i + 1, error)?;
                }
                Ok(())
            }
            Self::Emission { message } => {
                write!(f, "Emission error: {}", message)
            }
            Self::Io { kind, message } => {
                write!(f, "IO error ({:?}): {}", kind, message)
            }
            Self::Utf8 { message } => {
                write!(f, "UTF-8 error: {}", message)
            }
            Self::Config { message } => {
                write!(f, "Configuration error: {}", message)
            }
            Self::LimitExceeded { message } => {
                write!(f, "Resource limit exceeded: {}", message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let pos = Position::new();

        let parse_err = Error::parse(pos.clone(), "unexpected token");
        assert!(matches!(parse_err, Error::Parse { .. }));
        assert_eq!(parse_err.position(), Some(&pos));

        let config_err = Error::config("invalid setting");
        assert!(matches!(config_err, Error::Config { .. }));
        assert_eq!(config_err.position(), None);
    }

    #[test]
    fn test_error_display() {
        let mut pos = Position::new();
        pos.line = 5;
        pos.column = 12;
        let err = Error::parse(pos, "unexpected character");
        let display = format!("{}", err);
        assert!(display.contains("line 5"));
        assert!(display.contains("column 12"));
        assert!(display.contains("unexpected character"));
    }
}
