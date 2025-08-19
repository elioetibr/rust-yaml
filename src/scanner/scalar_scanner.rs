//! Scalar scanning functionality for YAML scanner

use super::{QuoteStyle, Token, TokenType};
use crate::{Error, Position, Result};

/// Trait for scanning scalar values
pub trait ScalarScanner {
    /// Scan a plain scalar (unquoted string)
    fn scan_plain_scalar(&mut self) -> Result<Token>;

    /// Scan a quoted string (single or double quotes)
    fn scan_quoted_string(&mut self, quote_char: char) -> Result<Token>;

    /// Scan a number (integer or float)
    fn scan_number(&mut self) -> Result<Token>;

    /// Scan a literal block scalar (|)
    fn scan_literal_block_scalar(&mut self) -> Result<Token>;

    /// Scan a folded block scalar (>)
    fn scan_folded_block_scalar(&mut self) -> Result<Token>;

    /// Scan block scalar header for chomping and indentation
    fn scan_block_scalar_header(&mut self) -> Result<(bool, Option<usize>)>;

    /// Helper: get current position
    fn current_position(&self) -> Position;

    /// Helper: peek at current character
    fn current_char(&self) -> Option<char>;

    /// Helper: advance to next character
    fn advance_char(&mut self) -> Option<char>;

    /// Helper: peek at next character
    fn peek_char(&self, offset: usize) -> Option<char>;

    /// Helper: check if at line start
    fn at_line_start(&self) -> bool;
}

/// Helper functions for scalar processing
pub(super) fn is_plain_scalar_char(ch: char) -> bool {
    !matches!(
        ch,
        ':' | ','
            | '['
            | ']'
            | '{'
            | '}'
            | '#'
            | '&'
            | '*'
            | '!'
            | '|'
            | '>'
            | '\''
            | '"'
            | '%'
            | '@'
            | '`'
    )
}

pub(super) fn process_escape_sequence(ch: char) -> Result<String> {
    match ch {
        'n' => Ok("\n".to_string()),
        'r' => Ok("\r".to_string()),
        't' => Ok("\t".to_string()),
        '\\' => Ok("\\".to_string()),
        '"' => Ok("\"".to_string()),
        '\'' => Ok("'".to_string()),
        '0' => Ok("\0".to_string()),
        'a' => Ok("\x07".to_string()), // Bell
        'b' => Ok("\x08".to_string()), // Backspace
        'f' => Ok("\x0C".to_string()), // Form feed
        'v' => Ok("\x0B".to_string()), // Vertical tab
        'e' => Ok("\x1B".to_string()), // Escape
        ' ' => Ok(" ".to_string()),
        'N' => Ok("\u{85}".to_string()),   // Next line (NEL)
        '_' => Ok("\u{A0}".to_string()),   // Non-breaking space
        'L' => Ok("\u{2028}".to_string()), // Line separator
        'P' => Ok("\u{2029}".to_string()), // Paragraph separator
        _ => Err(Error::scan(
            Position::new(),
            format!("Invalid escape sequence: \\{}", ch),
        )),
    }
}

/// Implementation of ScalarScanner for BasicScanner
impl ScalarScanner for super::BasicScanner {
    fn scan_plain_scalar(&mut self) -> Result<Token> {
        let start_pos = self.position;
        let mut value = String::new();

        while let Some(ch) = self.current_char {
            // Stop at structural characters in block context
            if self.flow_level == 0 {
                match ch {
                    '\n' | '\r' => break,
                    ':' if self.peek_char(1).map_or(true, |c| c.is_whitespace()) => break,
                    '#' if value.is_empty()
                        || self.peek_char(-1).map_or(false, |c| c.is_whitespace()) =>
                    {
                        break;
                    }
                    _ => {}
                }
            } else {
                // In flow context, stop at flow indicators
                match ch {
                    ',' | '[' | ']' | '{' | '}' => break,
                    ':' if self
                        .peek_char(1)
                        .map_or(true, |c| c.is_whitespace() || "]}".contains(c)) =>
                    {
                        break;
                    }
                    '#' if value.is_empty()
                        || self.peek_char(-1).map_or(false, |c| c.is_whitespace()) =>
                    {
                        break;
                    }
                    _ => {}
                }
            }

            value.push(ch);
            self.advance();
        }

        // Check string length limit
        self.resource_tracker
            .check_string_length(&self.limits, value.len())?;

        // Trim trailing whitespace from plain scalars
        let value = value.trim_end().to_string();
        let normalized_value = Self::normalize_scalar(value);

        Ok(Token::new(
            TokenType::Scalar(normalized_value, QuoteStyle::Plain),
            start_pos,
            self.position,
        ))
    }

    fn scan_quoted_string(&mut self, quote_char: char) -> Result<Token> {
        let start_pos = self.position;
        let mut value = String::new();

        // Skip opening quote
        self.advance();

        while let Some(ch) = self.current_char {
            if ch == quote_char {
                // End quote found
                self.advance();
                break;
            } else if ch == '\\' && quote_char == '"' {
                // Handle escape sequences in double quotes
                self.advance();
                if let Some(escaped_char) = self.current_char {
                    match escaped_char {
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        '\'' => value.push('\''),
                        '0' => value.push('\0'),
                        'a' => value.push('\x07'), // Bell
                        'b' => value.push('\x08'), // Backspace
                        'f' => value.push('\x0C'), // Form feed
                        'v' => value.push('\x0B'), // Vertical tab
                        'e' => value.push('\x1B'), // Escape
                        ' ' => value.push(' '),
                        'N' => value.push('\u{85}'),   // Next line (NEL)
                        '_' => value.push('\u{A0}'),   // Non-breaking space
                        'L' => value.push('\u{2028}'), // Line separator
                        'P' => value.push('\u{2029}'), // Paragraph separator
                        _ => {
                            // Invalid escape sequence
                            return Err(Error::scan(
                                self.position,
                                format!("Invalid escape sequence: \\{}", escaped_char),
                            ));
                        }
                    }
                    self.advance();
                } else {
                    return Err(Error::scan(
                        self.position,
                        "Unterminated escape sequence".to_string(),
                    ));
                }
            } else {
                value.push(ch);
                self.advance();
            }
        }

        // Check string length limit
        self.resource_tracker
            .check_string_length(&self.limits, value.len())?;

        let quote_style = match quote_char {
            '\'' => QuoteStyle::Single,
            '"' => QuoteStyle::Double,
            _ => QuoteStyle::Plain,
        };

        Ok(Token::new(
            TokenType::Scalar(value, quote_style),
            start_pos,
            self.position,
        ))
    }

    fn scan_number(&mut self) -> Result<Token> {
        let start_pos = self.position;
        let mut value = String::new();

        // Handle negative numbers
        if self.current_char == Some('-') {
            value.push('-');
            self.advance();
        }

        // Scan digits
        while let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                value.push(ch);
                self.advance();
            } else if ch == '.' {
                value.push(ch);
                self.advance();
                // Scan fractional part
                while let Some(ch) = self.current_char {
                    if ch.is_ascii_digit() {
                        value.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                break;
            } else {
                break;
            }
        }

        Ok(Token::new(
            TokenType::Scalar(value, QuoteStyle::Plain),
            start_pos,
            self.position,
        ))
    }

    fn scan_literal_block_scalar(&mut self) -> Result<Token> {
        let start_pos = self.position;

        // Skip the '|' character
        self.advance();

        // Scan block scalar header for chomping and indentation
        let (keep_chomping, explicit_indent) = self.scan_block_scalar_header()?;

        // Find the base indentation level
        let mut base_indent = None;
        let mut lines = Vec::new();
        let mut current_line = String::new();

        // Skip to end of header line
        while let Some(ch) = self.current_char {
            if ch == '\n' || ch == '\r' {
                self.advance();
                break;
            }
            self.advance();
        }

        // Collect lines
        while let Some(ch) = self.current_char {
            if ch == '\n' || ch == '\r' {
                lines.push(current_line.clone());
                current_line.clear();
                self.advance();

                // Check if next line has content to determine if we should continue
                let mut temp_indent = 0usize;
                let mut has_content = false;

                while let Some(next_ch) = self.peek_char(temp_indent as isize) {
                    if next_ch == ' ' || next_ch == '\t' {
                        temp_indent += 1;
                    } else if next_ch == '\n' || next_ch == '\r' {
                        // Empty line, continue collecting
                        break;
                    } else {
                        has_content = true;
                        break;
                    }
                }

                if !has_content {
                    // No more content lines
                    break;
                }

                // Set base indentation from first content line
                if base_indent.is_none() && has_content {
                    base_indent = Some(explicit_indent.unwrap_or(temp_indent));
                }
            } else {
                current_line.push(ch);
                self.advance();
            }
        }

        // Add final line if not empty
        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Join lines with newlines (literal style preserves line breaks)
        let mut value = lines.join("\n");

        // Apply chomping rules
        if !keep_chomping {
            value = value.trim_end_matches('\n').to_string();
        }

        // Check string length limit
        self.resource_tracker
            .check_string_length(&self.limits, value.len())?;

        Ok(Token::new(
            TokenType::BlockScalarLiteral(value),
            start_pos,
            self.position,
        ))
    }

    fn scan_folded_block_scalar(&mut self) -> Result<Token> {
        let start_pos = self.position;

        // Skip the '>' character
        self.advance();

        // Scan block scalar header for chomping and indentation
        let (keep_chomping, explicit_indent) = self.scan_block_scalar_header()?;

        // Similar to literal but fold newlines
        let mut base_indent = None;
        let mut lines = Vec::new();
        let mut current_line = String::new();

        // Skip to end of header line
        while let Some(ch) = self.current_char {
            if ch == '\n' || ch == '\r' {
                self.advance();
                break;
            }
            self.advance();
        }

        // Collect lines
        while let Some(ch) = self.current_char {
            if ch == '\n' || ch == '\r' {
                lines.push(current_line.clone());
                current_line.clear();
                self.advance();

                // Check if next line has content
                let mut temp_indent = 0usize;
                let mut has_content = false;

                while let Some(next_ch) = self.peek_char(temp_indent as isize) {
                    if next_ch == ' ' || next_ch == '\t' {
                        temp_indent += 1;
                    } else if next_ch == '\n' || next_ch == '\r' {
                        break;
                    } else {
                        has_content = true;
                        break;
                    }
                }

                if !has_content {
                    break;
                }

                if base_indent.is_none() && has_content {
                    base_indent = Some(explicit_indent.unwrap_or(temp_indent));
                }
            } else {
                current_line.push(ch);
                self.advance();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        // Fold lines: join non-empty lines with spaces, preserve empty lines
        let mut value = String::new();
        let mut prev_was_empty = false;

        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                if !prev_was_empty && i > 0 {
                    value.push('\n');
                }
                prev_was_empty = true;
            } else {
                if i > 0 && !prev_was_empty {
                    value.push(' ');
                } else if prev_was_empty && i > 0 {
                    value.push('\n');
                }
                value.push_str(line.trim());
                prev_was_empty = false;
            }
        }

        // Apply chomping rules
        if !keep_chomping {
            value = value.trim_end_matches('\n').to_string();
        }

        // Check string length limit
        self.resource_tracker
            .check_string_length(&self.limits, value.len())?;

        Ok(Token::new(
            TokenType::BlockScalarFolded(value),
            start_pos,
            self.position,
        ))
    }

    fn scan_block_scalar_header(&mut self) -> Result<(bool, Option<usize>)> {
        let mut keep_chomping = true;
        let mut explicit_indent = None;

        // Skip whitespace after '|' or '>'
        while let Some(ch) = self.current_char {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }

        // Check for explicit indentation indicator (digit)
        if let Some(ch) = self.current_char {
            if ch.is_ascii_digit() {
                explicit_indent = Some(ch.to_digit(10).unwrap() as usize);
                self.advance();
            }
        }

        // Check for chomping indicator
        if let Some(ch) = self.current_char {
            match ch {
                '-' => {
                    keep_chomping = false; // Strip final newlines
                    self.advance();
                }
                '+' => {
                    keep_chomping = true; // Keep final newlines
                    self.advance();
                }
                _ => {}
            }
        }

        Ok((keep_chomping, explicit_indent))
    }

    // Helper trait methods
    fn current_position(&self) -> Position {
        self.position
    }

    fn current_char(&self) -> Option<char> {
        self.current_char
    }

    fn advance_char(&mut self) -> Option<char> {
        self.advance()
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        self.peek_char(offset as isize)
    }

    fn at_line_start(&self) -> bool {
        self.position.column == 1
    }
}
