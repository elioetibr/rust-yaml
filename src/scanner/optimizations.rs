//! Scanner optimizations for better performance

use crate::{Error, Result, profiling::YamlProfiler, scanner::QuoteStyle};

/// Optimized string scanning utilities
pub struct StringScanner {
    buffer: String,
    profiler: Option<YamlProfiler>,
}

impl StringScanner {
    pub fn new() -> Self {
        Self {
            buffer: String::with_capacity(128),
            profiler: std::env::var("RUST_YAML_PROFILE")
                .ok()
                .map(|_| YamlProfiler::new()),
        }
    }

    /// Fast scan for quoted strings with minimal allocations
    pub fn scan_quoted_string(
        &mut self,
        chars: &[char],
        start_pos: usize,
        quote_char: char,
    ) -> Result<(String, QuoteStyle, usize)> {
        let use_profiler = self.profiler.is_some();
        if use_profiler {
            let result = self.scan_quoted_string_impl(chars, start_pos, quote_char);
            if let Some(ref mut profiler) = self.profiler {
                profiler.record_memory(
                    "scan_quoted_string",
                    chars.len() * std::mem::size_of::<char>(),
                );
            }
            result
        } else {
            self.scan_quoted_string_impl(chars, start_pos, quote_char)
        }
    }

    fn scan_quoted_string_impl(
        &mut self,
        chars: &[char],
        start_pos: usize,
        quote_char: char,
    ) -> Result<(String, QuoteStyle, usize)> {
        self.buffer.clear();
        let mut pos = start_pos + 1; // Skip opening quote
        let quote_style = if quote_char == '\'' {
            QuoteStyle::Single
        } else {
            QuoteStyle::Double
        };

        while pos < chars.len() {
            match chars[pos] {
                c if c == quote_char => {
                    // End of string
                    return Ok((self.buffer.clone(), quote_style, pos + 1));
                }
                '\\' if quote_char == '"' => {
                    // Handle escape sequences in double-quoted strings
                    pos += 1;
                    if pos >= chars.len() {
                        return Err(Error::parse(
                            crate::Position::new(),
                            "Unexpected end of input in escape sequence",
                        ));
                    }

                    match chars[pos] {
                        'n' => self.buffer.push('\n'),
                        't' => self.buffer.push('\t'),
                        'r' => self.buffer.push('\r'),
                        '\\' => self.buffer.push('\\'),
                        '"' => self.buffer.push('"'),
                        '\'' => self.buffer.push('\''),
                        '0' => self.buffer.push('\0'),
                        c => {
                            // For unknown escapes, preserve both characters
                            self.buffer.push('\\');
                            self.buffer.push(c);
                        }
                    }
                    pos += 1;
                }
                '\'' if quote_char == '\'' => {
                    // Handle single quote escaping in single-quoted strings
                    if pos + 1 < chars.len() && chars[pos + 1] == '\'' {
                        self.buffer.push('\'');
                        pos += 2; // Skip both quotes
                    } else {
                        // End of string
                        return Ok((self.buffer.clone(), quote_style, pos + 1));
                    }
                }
                c => {
                    self.buffer.push(c);
                    pos += 1;
                }
            }
        }

        Err(Error::parse(
            crate::Position::new(),
            &format!(
                "Unterminated {} quoted string",
                if quote_char == '"' {
                    "double"
                } else {
                    "single"
                }
            ),
        ))
    }

    /// Fast scan for unquoted scalars
    pub fn scan_plain_scalar(
        &mut self,
        chars: &[char],
        start_pos: usize,
    ) -> Result<(String, usize)> {
        let use_profiler = self.profiler.is_some();
        if use_profiler {
            let result = self.scan_plain_scalar_impl(chars, start_pos);
            if let Some(ref mut profiler) = self.profiler {
                profiler.record_memory(
                    "scan_plain_scalar",
                    chars.len() * std::mem::size_of::<char>(),
                );
            }
            result
        } else {
            self.scan_plain_scalar_impl(chars, start_pos)
        }
    }

    fn scan_plain_scalar_impl(
        &mut self,
        chars: &[char],
        start_pos: usize,
    ) -> Result<(String, usize)> {
        self.buffer.clear();
        let mut pos = start_pos;

        while pos < chars.len() {
            match chars[pos] {
                // Characters that end a plain scalar
                ':' | '{' | '}' | '[' | ']' | ',' | '#' | '&' | '*' | '!' | '|' | '>' | '\''
                | '"' | '%' | '@' | '`' => {
                    break;
                }
                // Whitespace handling
                ' ' | '\t' => {
                    // Check if this is trailing whitespace
                    let mut next_pos = pos + 1;
                    while next_pos < chars.len()
                        && (chars[next_pos] == ' ' || chars[next_pos] == '\t')
                    {
                        next_pos += 1;
                    }

                    // If we hit end of line or special characters, stop
                    if next_pos >= chars.len()
                        || matches!(
                            chars[next_pos],
                            '\n' | '\r' | ':' | '{' | '}' | '[' | ']' | ',' | '#'
                        )
                    {
                        break;
                    }

                    // Add the space and continue
                    self.buffer.push(' ');
                    pos = next_pos;
                }
                '\n' | '\r' => {
                    break;
                }
                c => {
                    self.buffer.push(c);
                    pos += 1;
                }
            }
        }

        // Trim trailing whitespace
        let result = self.buffer.trim_end().to_string();
        Ok((result, pos))
    }

    /// Fast whitespace skipping
    pub fn skip_whitespace(chars: &[char], mut pos: usize) -> usize {
        while pos < chars.len() && matches!(chars[pos], ' ' | '\t') {
            pos += 1;
        }
        pos
    }

    /// Fast line ending detection
    pub fn is_line_ending(chars: &[char], pos: usize) -> bool {
        pos < chars.len() && matches!(chars[pos], '\n' | '\r')
    }

    /// Skip to end of line
    pub fn skip_to_line_end(chars: &[char], mut pos: usize) -> usize {
        while pos < chars.len() && !matches!(chars[pos], '\n' | '\r') {
            pos += 1;
        }
        pos
    }
}

impl Default for StringScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory-efficient token buffer
pub struct TokenBuffer {
    tokens: Vec<crate::scanner::Token>,
    capacity: usize,
}

impl TokenBuffer {
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            capacity: 256, // Start with reasonable capacity
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            tokens: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, token: crate::scanner::Token) {
        self.tokens.push(token);
    }

    pub fn get(&self, index: usize) -> Option<&crate::scanner::Token> {
        self.tokens.get(index)
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn clear(&mut self) {
        self.tokens.clear();
    }

    /// Shrink the buffer if it's using too much memory
    pub fn shrink_if_needed(&mut self) {
        if self.tokens.capacity() > self.capacity * 2 && self.tokens.is_empty() {
            self.tokens.shrink_to(self.capacity);
        }
    }
}

impl Default for TokenBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_scanner_quoted_strings() {
        let mut scanner = StringScanner::new();
        let chars: Vec<char> = "'hello world'".chars().collect();

        let result = scanner.scan_quoted_string(&chars, 0, '"').unwrap();
        assert_eq!(result.0, "hello world");
        assert_eq!(result.1, QuoteStyle::Double);
        assert_eq!(result.2, chars.len());
    }

    #[test]
    fn test_string_scanner_escape_sequences() {
        let mut scanner = StringScanner::new();
        let chars: Vec<char> = "'hello\nworld\t!'".chars().collect();

        let result = scanner.scan_quoted_string(&chars, 0, '"').unwrap();
        assert_eq!(result.0, "hello\nworld\t!");
    }

    #[test]
    fn test_string_scanner_plain_scalar() {
        let mut scanner = StringScanner::new();
        let chars: Vec<char> = "hello world: value".chars().collect();

        let result = scanner.scan_plain_scalar(&chars, 0).unwrap();
        assert_eq!(result.0, "hello world");
        assert!(result.1 > 0);
    }

    #[test]
    fn test_whitespace_skipping() {
        let chars: Vec<char> = "   \t  hello".chars().collect();
        let pos = StringScanner::skip_whitespace(&chars, 0);
        assert_eq!(chars[pos], 'h');
    }

    #[test]
    fn test_token_buffer() {
        let mut buffer = TokenBuffer::new();
        assert!(buffer.is_empty());

        let token = crate::scanner::Token::new(
            crate::scanner::TokenType::StreamStart,
            crate::Position::new(),
            crate::Position::new(),
        );

        buffer.push(token);
        assert_eq!(buffer.len(), 1);
        assert!(!buffer.is_empty());
    }
}
