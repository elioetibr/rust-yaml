//! YAML scanner for tokenization

use crate::{Error, Limits, Position, ResourceTracker, Result, error::ErrorContext};

pub mod indentation;
pub mod scalar_scanner;
pub mod state;
pub mod token_processor;
pub mod tokens;
// pub mod optimizations; // Temporarily disabled
pub use scalar_scanner::ScalarScanner;
pub use tokens::*;
// pub use optimizations::*;

/// Trait for YAML scanners that convert character streams to tokens
pub trait Scanner {
    /// Check if there are more tokens available
    fn check_token(&self) -> bool;

    /// Peek at the next token without consuming it
    fn peek_token(&self) -> Result<Option<&Token>>;

    /// Get the next token, consuming it
    fn get_token(&mut self) -> Result<Option<Token>>;

    /// Reset the scanner state
    fn reset(&mut self);

    /// Get the current position in the input
    fn position(&self) -> Position;

    /// Get the input text for error reporting
    fn input(&self) -> &str;
}

/// A basic scanner implementation for YAML tokenization
#[derive(Debug)]
#[allow(dead_code)]
pub struct BasicScanner {
    input: String,
    position: Position,
    current_char: Option<char>,
    tokens: Vec<Token>,
    token_index: usize,
    done: bool,
    indent_stack: Vec<usize>,
    current_indent: usize,
    allow_simple_key: bool,
    simple_key_allowed: bool,
    flow_level: usize,
    preserve_comments: bool,
    // Indentation style detection
    detected_indent_style: Option<crate::value::IndentStyle>,
    indent_samples: Vec<(usize, bool)>, // (size, is_tabs)
    previous_indent_level: usize,       // Track the previous indentation for style detection
    // Performance optimizations
    buffer: String,                   // Reusable string buffer for token values
    char_cache: Vec<char>,            // Cached characters for faster access
    char_indices: Vec<(usize, char)>, // Cached character indices for O(1) lookups
    current_char_index: usize,        // Current index in char_cache
    profiler: Option<crate::profiling::YamlProfiler>, // Optional profiling
    // Error tracking
    scanning_error: Option<Error>, // Store scanning errors for later retrieval
    // Resource tracking
    limits: Limits,
    resource_tracker: ResourceTracker,
    // Track inline nested sequences that need closing
    inline_sequence_depth: usize,
}

impl BasicScanner {
    /// Create a new scanner from input string
    pub fn new(input: String) -> Self {
        Self::with_limits(input, Limits::default())
    }

    /// Create a new scanner with custom resource limits
    pub fn with_limits(input: String, limits: Limits) -> Self {
        let char_cache: Vec<char> = input.chars().collect();
        let char_indices: Vec<(usize, char)> = input.char_indices().collect();
        let current_char = char_cache.first().copied();

        // Track document size for resource limits
        let mut resource_tracker = ResourceTracker::new();
        if let Err(e) = resource_tracker.add_bytes(&limits, input.len()) {
            // If the input is too large, create scanner with error state
            return Self {
                current_char: None,
                input,
                position: Position::start(),
                tokens: Vec::new(),
                token_index: 0,
                done: true,
                indent_stack: vec![0],
                current_indent: 0,
                allow_simple_key: false,
                simple_key_allowed: false,
                flow_level: 0,
                preserve_comments: false,
                detected_indent_style: None,
                indent_samples: Vec::new(),
                previous_indent_level: 0,
                buffer: String::new(),
                char_cache: Vec::new(),
                char_indices: Vec::new(),
                current_char_index: 0,
                profiler: None,
                scanning_error: Some(e),
                limits,
                resource_tracker,
                inline_sequence_depth: 0,
            };
        }

        Self {
            current_char,
            input,
            position: Position::start(),
            tokens: Vec::new(),
            token_index: 0,
            done: false,
            indent_stack: vec![0], // Always start with base indentation
            current_indent: 0,
            allow_simple_key: true,
            simple_key_allowed: true,
            flow_level: 0,
            preserve_comments: false,
            detected_indent_style: None,
            indent_samples: Vec::new(),
            previous_indent_level: 0,
            buffer: String::with_capacity(64), // Pre-allocate buffer
            char_cache,
            char_indices,
            current_char_index: 0,
            profiler: std::env::var("RUST_YAML_PROFILE")
                .ok()
                .map(|_| crate::profiling::YamlProfiler::new()),
            scanning_error: None,
            limits,
            resource_tracker,
            inline_sequence_depth: 0,
        }
    }

    /// Create a new scanner with eager token scanning (for compatibility)
    pub fn new_eager(input: String) -> Self {
        Self::new_eager_with_limits(input, Limits::default())
    }

    /// Create a new scanner with eager token scanning and custom limits
    pub fn new_eager_with_limits(input: String, limits: Limits) -> Self {
        let mut scanner = Self::with_limits(input, limits);
        // Store any scanning errors for later retrieval
        if let Err(error) = scanner.scan_all_tokens() {
            scanner.scanning_error = Some(error);
        }
        scanner
    }

    /// Create a new scanner with comment preservation enabled
    pub fn new_with_comments(input: String) -> Self {
        let mut scanner = Self::new(input);
        scanner.preserve_comments = true;
        scanner
    }

    /// Create a new scanner with comments and custom limits
    pub fn new_with_comments_and_limits(input: String, limits: Limits) -> Self {
        let mut scanner = Self::with_limits(input, limits);
        scanner.preserve_comments = true;
        scanner
    }

    /// Create a new scanner with eager scanning and comment preservation
    pub fn new_eager_with_comments(input: String) -> Self {
        let mut scanner = Self::new_with_comments(input);
        scanner.scan_all_tokens().unwrap_or(());
        scanner
    }

    /// Get the detected indentation style from the document
    pub const fn detected_indent_style(&self) -> Option<&crate::value::IndentStyle> {
        self.detected_indent_style.as_ref()
    }

    /// Check if there was a scanning error
    pub const fn has_scanning_error(&self) -> bool {
        self.scanning_error.is_some()
    }

    /// Get the scanning error if any
    #[allow(clippy::missing_const_for_fn)]
    pub fn take_scanning_error(&mut self) -> Option<Error> {
        self.scanning_error.take()
    }

    /// Advance to the next character
    fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.current_char {
            self.position = self.position.advance(ch);
            self.current_char_index += 1;

            if self.current_char_index < self.char_cache.len() {
                self.current_char = Some(self.char_cache[self.current_char_index]);
            } else {
                self.current_char = None;
            }
        }

        self.current_char
    }

    /// Skip whitespace characters (excluding newlines)
    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.current_char {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Handle indentation and produce block tokens if necessary
    fn handle_indentation(&mut self) -> Result<()> {
        // Only handle indentation in block context (flow_level == 0)
        if self.flow_level > 0 {
            return Ok(());
        }

        let line_start_pos = self.position;
        let mut indent = 0;
        let mut has_tabs = false;
        let mut has_spaces = false;
        let _indent_start_pos = self.position;

        // Count indentation and detect style
        while let Some(ch) = self.current_char {
            if ch == ' ' {
                indent += 1;
                has_spaces = true;
                self.advance();
            } else if ch == '\t' {
                indent += 8; // Tab counts as 8 spaces for indentation calculation
                has_tabs = true;
                self.advance();
            } else {
                break;
            }
        }

        // Analyze indentation pattern for style detection
        // Only analyze if there's actual content after the indentation (not just whitespace)
        if indent > 0
            && self.current_char.is_some()
            && !matches!(self.current_char, Some('\n' | '\r'))
        {
            self.analyze_indentation_pattern(indent, has_tabs, has_spaces)?;
        }

        // Perform strict indentation validation if we have established a style
        if let Some(crate::value::IndentStyle::Spaces(width)) = self.detected_indent_style {
            if indent > 0 && indent % width != 0 {
                // Check if this is a valid nested level or inconsistent indentation
                let is_valid_nesting = self.is_valid_indentation_level(indent);
                if !is_valid_nesting {
                    let lower_level = (indent / width) * width;
                    let higher_level = lower_level + width;
                    let suggestion = format!(
                        "Inconsistent indentation detected. Expected multiples of {} spaces. Use {} or {} spaces instead of {}",
                        width, lower_level, higher_level, indent
                    );
                    let context =
                        crate::error::ErrorContext::from_input(&self.input, &self.position, 4)
                            .with_suggestion(suggestion);
                    return Err(Error::indentation_with_context(
                        self.position,
                        lower_level,
                        indent,
                        context,
                    ));
                }
            }
        }

        // Update previous indentation level for future comparisons
        if indent > 0 {
            self.previous_indent_level = indent;
        }

        // Update current indentation level
        self.current_indent = indent;

        // Check if we need to emit block end tokens for decreased indentation
        while let Some(&last_indent) = self.indent_stack.last() {
            if indent < last_indent && last_indent > 0 {
                self.indent_stack.pop();
                self.tokens
                    .push(Token::simple(TokenType::BlockEnd, line_start_pos));
            } else {
                break;
            }
        }

        Ok(())
    }

    /// Analyze indentation pattern to detect the document's indentation style
    fn analyze_indentation_pattern(
        &mut self,
        current_indent: usize,
        has_tabs: bool,
        has_spaces: bool,
    ) -> Result<()> {
        // Prevent mixed indentation (tabs + spaces on same line)
        if has_tabs && has_spaces {
            let context = crate::error::ErrorContext::from_input(&self.input, &self.position, 4)
                .with_suggestion("Use either tabs OR spaces for indentation, not both".to_string());
            return Err(Error::invalid_character_with_context(
                self.position,
                '\t',
                "mixed indentation",
                context,
            ));
        }

        // If we detected tabs, check for mixed indentation across lines
        if has_tabs {
            match self.detected_indent_style {
                None => {
                    // First time detecting indentation style - set to tabs
                    self.detected_indent_style = Some(crate::value::IndentStyle::Tabs);
                }
                Some(crate::value::IndentStyle::Spaces(_)) => {
                    // Previously detected spaces, now seeing tabs - mixed indentation error
                    let context =
                        crate::error::ErrorContext::from_input(&self.input, &self.position, 4)
                            .with_suggestion(
                                "Use consistent indentation style throughout the document"
                                    .to_string(),
                            );
                    return Err(Error::invalid_character_with_context(
                        self.position,
                        '\t',
                        "mixed indentation",
                        context,
                    ));
                }
                Some(crate::value::IndentStyle::Tabs) => {
                    // Already using tabs - this is consistent
                }
            }
            return Ok(());
        }

        // For spaces, check for mixed indentation across lines first
        if has_spaces {
            // Check if we previously detected tabs
            if matches!(
                self.detected_indent_style,
                Some(crate::value::IndentStyle::Tabs)
            ) {
                let context =
                    crate::error::ErrorContext::from_input(&self.input, &self.position, 4)
                        .with_suggestion(
                            "Use consistent indentation style throughout the document".to_string(),
                        );
                return Err(Error::invalid_character_with_context(
                    self.position,
                    ' ',
                    "mixed indentation",
                    context,
                ));
            }

            // Calculate the indentation level difference
            if current_indent > self.previous_indent_level {
                let indent_diff = current_indent - self.previous_indent_level;

                // Store this sample for analysis (but only meaningful differences)
                if indent_diff > 0 && indent_diff <= 8 {
                    // Reasonable indentation range
                    self.indent_samples.push((indent_diff, false));

                    // Try to determine consistent indentation width
                    if self.detected_indent_style.is_none() {
                        self.detect_space_indentation_width();
                    }
                }
            }

            // Validate indentation consistency if we already have a detected style
            self.validate_indentation_consistency(current_indent)?;
        }

        Ok(())
    }

    /// Detect the consistent space indentation width from samples
    fn detect_space_indentation_width(&mut self) {
        if self.indent_samples.is_empty() {
            return; // Need at least 1 sample
        }

        // Find the most common indentation width
        let mut width_counts = std::collections::HashMap::new();

        for &(width, is_tabs) in &self.indent_samples {
            if !is_tabs && width > 0 {
                *width_counts.entry(width).or_insert(0) += 1;
            }
        }

        // Find the most frequent width - be more aggressive and detect early
        if let Some((&most_common_width, &_count)) =
            width_counts.iter().max_by_key(|&(_, count)| count)
        {
            // Set on first consistent sample to enable stricter validation
            self.detected_indent_style = Some(crate::value::IndentStyle::Spaces(most_common_width));
        }
    }

    /// Check if the given indentation level is valid based on current context
    #[allow(clippy::missing_const_for_fn)] // Cannot be const due to self.detected_indent_style access
    fn is_valid_indentation_level(&self, indent: usize) -> bool {
        // For now, allow any indentation that could represent valid nesting
        // In the future, this could be made more strict by checking against
        // the current indent_stack to ensure proper nesting
        if let Some(crate::value::IndentStyle::Spaces(width)) = self.detected_indent_style {
            // Must be a multiple of the detected width
            indent % width == 0
        } else {
            // If no style detected yet, allow any indentation
            true
        }
    }

    /// Validate that current indentation is consistent with detected style
    fn validate_indentation_consistency(&self, current_indent: usize) -> Result<()> {
        if let Some(crate::value::IndentStyle::Spaces(width)) = self.detected_indent_style {
            // Check if current indentation is a multiple of the detected width
            if current_indent > 0 && current_indent % width != 0 {
                let lower_level = (current_indent / width) * width;
                let higher_level = lower_level + width;
                let suggestion = format!(
                    "Expected indentation to be a multiple of {} spaces. Use {} or {} spaces instead of {}",
                    width, lower_level, higher_level, current_indent
                );
                let context =
                    crate::error::ErrorContext::from_input(&self.input, &self.position, 4)
                        .with_suggestion(suggestion);
                return Err(Error::indentation_with_context(
                    self.position,
                    (current_indent / width) * width, // expected (nearest valid level)
                    current_indent,                   // found
                    context,
                ));
            }
        }
        Ok(())
    }

    /// Check if current position starts a plain scalar
    fn is_plain_scalar_start(&self) -> bool {
        self.current_char.map_or(false, |ch| match ch {
            '-' | '?' | ':' | ',' | '[' | ']' | '{' | '}' | '#' | '&' | '*' | '!' | '|' | '>'
            | '\'' | '"' | '%' | '@' | '`' => false,
            _ => !ch.is_whitespace(),
        })
    }

    /// Check if the value is a YAML boolean
    fn is_yaml_bool(value: &str) -> bool {
        matches!(
            value,
            "true"
                | "false"
                | "True"
                | "False"
                | "TRUE"
                | "FALSE"
                | "yes"
                | "no"
                | "Yes"
                | "No"
                | "YES"
                | "NO"
                | "on"
                | "off"
                | "On"
                | "Off"
                | "ON"
                | "OFF"
        )
    }

    /// Check if the value is a YAML null
    fn is_yaml_null(value: &str) -> bool {
        matches!(value, "null" | "Null" | "NULL" | "~" | "")
    }

    /// Normalize a scalar value based on YAML rules
    fn normalize_scalar(value: String) -> String {
        if Self::is_yaml_bool(&value) {
            // Normalize booleans to lowercase
            match value.to_lowercase().as_str() {
                "true" | "yes" | "on" => "true".to_string(),
                "false" | "no" | "off" => "false".to_string(),
                _ => value,
            }
        } else if Self::is_yaml_null(&value) {
            // Normalize nulls to empty string (will be handled by parser)
            "null".to_string()
        } else {
            value
        }
    }

    /// Scan a number token
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
            TokenType::Scalar(value, tokens::QuoteStyle::Plain),
            start_pos,
            self.position,
        ))
    }

    /// Scan a plain scalar (unquoted string)
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
            TokenType::Scalar(normalized_value, tokens::QuoteStyle::Plain),
            start_pos,
            self.position,
        ))
    }

    /// Scan a quoted string
    fn scan_quoted_string(&mut self, quote_char: char) -> Result<Token> {
        let start_pos = self.position;
        let mut value = String::new();

        // Determine quote style based on quote character
        let quote_style = match quote_char {
            '\'' => tokens::QuoteStyle::Single,
            '"' => tokens::QuoteStyle::Double,
            _ => tokens::QuoteStyle::Plain,
        };

        self.advance(); // Skip opening quote

        while let Some(ch) = self.current_char {
            if ch == quote_char {
                self.advance(); // Skip closing quote
                break;
            } else if ch == '\\' {
                self.advance();
                if let Some(escaped) = self.current_char {
                    match escaped {
                        // Standard C-style escapes
                        'n' => value.push('\n'),  // newline
                        't' => value.push('\t'),  // tab
                        'r' => value.push('\r'),  // carriage return
                        '\\' => value.push('\\'), // literal backslash
                        '\'' => value.push('\''), // single quote
                        '"' => value.push('"'),   // double quote

                        // Additional YAML escapes
                        '0' => value.push('\0'),   // null character
                        'a' => value.push('\x07'), // bell character
                        'b' => value.push('\x08'), // backspace
                        'f' => value.push('\x0C'), // form feed
                        'v' => value.push('\x0B'), // vertical tab
                        'e' => value.push('\x1B'), // escape character
                        ' ' => value.push(' '),    // literal space
                        '/' => value.push('/'),    // literal forward slash

                        // For unknown escapes, preserve them literally (YAML spec behavior)
                        _ => {
                            value.push('\\');
                            value.push(escaped);
                        }
                    }
                    self.advance();
                }
            } else {
                value.push(ch);
                self.advance();

                // Check string length periodically to fail fast
                if value.len() > self.limits.max_string_length {
                    return Err(Error::limit_exceeded(format!(
                        "String length {} exceeds maximum {}",
                        value.len(),
                        self.limits.max_string_length
                    )));
                }
            }
        }

        // Check string length limit
        self.resource_tracker
            .check_string_length(&self.limits, value.len())?;

        Ok(Token::new(
            TokenType::Scalar(value, quote_style),
            start_pos,
            self.position,
        ))
    }

    /// Scan document start marker (---)
    fn scan_document_start(&mut self) -> Result<Option<Token>> {
        if self.current_char == Some('-')
            && self.peek_char(1) == Some('-')
            && self.peek_char(2) == Some('-')
            && self.peek_char(3).map_or(true, |c| c.is_whitespace())
        {
            let start_pos = self.position;
            self.advance(); // -
            self.advance(); // -
            self.advance(); // -

            Ok(Some(Token::new(
                TokenType::DocumentStart,
                start_pos,
                self.position,
            )))
        } else {
            Ok(None)
        }
    }

    /// Scan YAML version directive (%YAML)
    fn scan_yaml_directive(&mut self) -> Result<Option<Token>> {
        if self.current_char != Some('%') {
            return Ok(None);
        }

        let start_pos = self.position;
        let saved_position = self.position;
        self.advance(); // Skip '%'

        // Check for "YAML"
        if self.current_char == Some('Y')
            && self.peek_char(1) == Some('A')
            && self.peek_char(2) == Some('M')
            && self.peek_char(3) == Some('L')
            && self.peek_char(4).map_or(false, |c| c.is_whitespace())
        {
            self.advance(); // Y
            self.advance(); // A
            self.advance(); // M
            self.advance(); // L

            // Skip whitespace
            self.skip_whitespace();

            // Parse version number (e.g., "1.2")
            let major = if let Some(ch) = self.current_char {
                if ch.is_ascii_digit() {
                    let digit = ch.to_digit(10).unwrap() as u8;
                    self.advance();
                    digit
                } else {
                    return Err(Error::scan(
                        self.position,
                        "Expected major version number after %YAML".to_string(),
                    ));
                }
            } else {
                return Err(Error::scan(
                    self.position,
                    "Expected version after %YAML directive".to_string(),
                ));
            };

            // Expect '.'
            if self.current_char != Some('.') {
                return Err(Error::scan(
                    self.position,
                    "Expected '.' in YAML version".to_string(),
                ));
            }
            self.advance();

            // Parse minor version
            let minor = if let Some(ch) = self.current_char {
                if ch.is_ascii_digit() {
                    let digit = ch.to_digit(10).unwrap() as u8;
                    self.advance();
                    digit
                } else {
                    return Err(Error::scan(
                        self.position,
                        "Expected minor version number after '.'".to_string(),
                    ));
                }
            } else {
                return Err(Error::scan(
                    self.position,
                    "Expected minor version number".to_string(),
                ));
            };

            Ok(Some(Token::new(
                TokenType::YamlDirective(major, minor),
                start_pos,
                self.position,
            )))
        } else {
            // Not a YAML directive, reset position
            self.position = saved_position;
            // Properly reset current_char based on saved position
            self.current_char = self
                .char_indices
                .iter()
                .find(|(i, _)| *i == saved_position.index)
                .map(|(_, ch)| *ch);
            // Reset the current_char_index
            self.current_char_index = self
                .char_indices
                .iter()
                .position(|(i, _)| *i == saved_position.index)
                .unwrap_or(0);
            Ok(None)
        }
    }

    /// Scan TAG directive (%TAG)
    fn scan_tag_directive(&mut self) -> Result<Option<Token>> {
        if self.current_char != Some('%') {
            return Ok(None);
        }

        let start_pos = self.position;
        let saved_position = self.position;
        self.advance(); // Skip '%'

        // Check for "TAG"
        if self.current_char == Some('T')
            && self.peek_char(1) == Some('A')
            && self.peek_char(2) == Some('G')
            && self.peek_char(3).map_or(false, |c| c.is_whitespace())
        {
            self.advance(); // T
            self.advance(); // A
            self.advance(); // G

            // Skip whitespace
            self.skip_whitespace();

            // Parse handle (e.g., "!" or "!!")
            let handle = self.scan_tag_handle()?;

            // Skip whitespace
            self.skip_whitespace();

            // Parse prefix (URI)
            let prefix = self.scan_tag_prefix()?;

            Ok(Some(Token::new(
                TokenType::TagDirective(handle, prefix),
                start_pos,
                self.position,
            )))
        } else {
            // Reset position if not a TAG directive
            self.position = saved_position;
            // Properly reset current_char based on saved position
            self.current_char = self
                .char_indices
                .iter()
                .find(|(i, _)| *i == saved_position.index)
                .map(|(_, ch)| *ch);
            // Reset the current_char_index
            self.current_char_index = self
                .char_indices
                .iter()
                .position(|(i, _)| *i == saved_position.index)
                .unwrap_or(0);
            Ok(None)
        }
    }

    /// Scan a tag handle for TAG directive
    fn scan_tag_handle(&mut self) -> Result<String> {
        let mut handle = String::new();

        if self.current_char != Some('!') {
            return Err(Error::scan(
                self.position,
                "Expected '!' at start of tag handle".to_string(),
            ));
        }

        handle.push('!');
        self.advance();

        // Handle can be "!" or "!!" or "!name!"
        if self.current_char == Some('!') {
            // Secondary handle "!!"
            handle.push('!');
            self.advance();
        } else if self.current_char.map_or(false, |c| c.is_alphanumeric()) {
            // Named handle like "!name!"
            while let Some(ch) = self.current_char {
                if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                    handle.push(ch);
                    self.advance();
                } else if ch == '!' {
                    handle.push(ch);
                    self.advance();
                    break;
                } else {
                    break;
                }
            }
        }
        // else just "!" primary handle

        Ok(handle)
    }

    /// Scan a tag prefix (URI) for TAG directive
    fn scan_tag_prefix(&mut self) -> Result<String> {
        let mut prefix = String::new();

        // Read until end of line or comment
        while let Some(ch) = self.current_char {
            if ch == '\n' || ch == '\r' || ch == '#' {
                break;
            }
            if ch.is_whitespace() && prefix.is_empty() {
                self.advance();
                continue;
            }
            if ch.is_whitespace() && !prefix.is_empty() {
                // Trailing whitespace, we're done
                break;
            }
            prefix.push(ch);
            self.advance();
        }

        if prefix.is_empty() {
            return Err(Error::scan(
                self.position,
                "Expected tag prefix after tag handle".to_string(),
            ));
        }

        Ok(prefix.trim().to_string())
    }

    /// Check if current position might be a directive
    fn is_directive(&self) -> bool {
        self.current_char == Some('%') && self.position.column == 1
    }

    /// Scan document end marker (...)
    fn scan_document_end(&mut self) -> Result<Option<Token>> {
        if self.current_char == Some('.')
            && self.peek_char(1) == Some('.')
            && self.peek_char(2) == Some('.')
            && self.peek_char(3).map_or(true, |c| c.is_whitespace())
        {
            let start_pos = self.position;
            self.advance(); // .
            self.advance(); // .
            self.advance(); // .

            Ok(Some(Token::new(
                TokenType::DocumentEnd,
                start_pos,
                self.position,
            )))
        } else {
            Ok(None)
        }
    }

    /// Scan a comment token
    fn scan_comment(&mut self) -> Result<Token> {
        let start_pos = self.position;
        let mut comment_text = String::new();

        // Skip the '#' character
        if self.current_char == Some('#') {
            self.advance();
        }

        // Collect the comment text
        while let Some(ch) = self.current_char {
            if ch == '\n' || ch == '\r' {
                break;
            }
            comment_text.push(ch);
            self.advance();
        }

        // Trim leading whitespace from comment text
        let comment_text = comment_text.trim_start().to_string();

        Ok(Token::new(
            TokenType::Comment(comment_text),
            start_pos,
            self.position,
        ))
    }

    /// Process a line and generate appropriate tokens
    #[allow(clippy::cognitive_complexity)]
    fn process_line(&mut self) -> Result<()> {
        // Check for directives at start of line
        if self.position.column == 1 && self.current_char == Some('%') {
            // Try to scan YAML directive
            if let Some(token) = self.scan_yaml_directive()? {
                self.tokens.push(token);
                return Ok(());
            }

            // Try to scan TAG directive
            if let Some(token) = self.scan_tag_directive()? {
                self.tokens.push(token);
                return Ok(());
            }

            // If not a recognized directive, treat as error
            if self.current_char == Some('%') {
                return Err(Error::scan(self.position, "Unknown directive".to_string()));
            }
        }

        // Check for document markers at start of line
        if self.position.column == 1 {
            // Check for document start marker
            if let Some(token) = self.scan_document_start()? {
                self.tokens.push(token);
                return Ok(());
            }

            // Check for document end marker
            if let Some(token) = self.scan_document_end()? {
                self.tokens.push(token);
                return Ok(());
            }
        }

        // Handle indentation at start of line
        if self.position.column == 1 {
            self.handle_indentation()?;
        }

        // Skip empty lines and comments
        self.skip_whitespace();

        match self.current_char {
            None => return Ok(()),
            Some('#') => {
                if self.preserve_comments {
                    // Create a comment token
                    let comment_token = self.scan_comment()?;
                    self.tokens.push(comment_token);
                } else {
                    // Skip comment lines
                    while let Some(ch) = self.current_char {
                        if ch == '\n' || ch == '\r' {
                            break;
                        }
                        self.advance();
                    }
                }
                return Ok(());
            }
            Some('\n' | '\r') => {
                self.advance();
                return Ok(());
            }
            _ => {}
        }

        // Process tokens on this line
        while let Some(ch) = self.current_char {
            match ch {
                '\n' | '\r' => break,
                ' ' | '\t' => {
                    self.skip_whitespace();
                }
                '#' => {
                    if self.preserve_comments {
                        // Create a comment token
                        let comment_token = self.scan_comment()?;
                        self.tokens.push(comment_token);
                    } else {
                        // Skip rest of line (comment)
                        while let Some(ch) = self.current_char {
                            if ch == '\n' || ch == '\r' {
                                break;
                            }
                            self.advance();
                        }
                    }
                    break;
                }

                // Flow indicators
                '[' => {
                    let pos = self.position;
                    self.advance();
                    self.flow_level += 1;
                    // Check depth limit
                    self.resource_tracker
                        .check_depth(&self.limits, self.flow_level + self.indent_stack.len())?;
                    self.tokens
                        .push(Token::new(TokenType::FlowSequenceStart, pos, self.position));
                }
                ']' => {
                    let pos = self.position;
                    self.advance();
                    if self.flow_level > 0 {
                        self.flow_level -= 1;
                    }
                    self.tokens
                        .push(Token::new(TokenType::FlowSequenceEnd, pos, self.position));
                }
                '{' => {
                    let pos = self.position;
                    self.advance();
                    self.flow_level += 1;
                    // Check depth limit
                    self.resource_tracker
                        .check_depth(&self.limits, self.flow_level + self.indent_stack.len())?;
                    self.tokens
                        .push(Token::new(TokenType::FlowMappingStart, pos, self.position));
                }
                '}' => {
                    let pos = self.position;
                    self.advance();
                    if self.flow_level > 0 {
                        self.flow_level -= 1;
                    }
                    self.tokens
                        .push(Token::new(TokenType::FlowMappingEnd, pos, self.position));
                }
                ',' => {
                    let pos = self.position;
                    self.advance();
                    self.tokens
                        .push(Token::new(TokenType::FlowEntry, pos, self.position));
                }

                // Key-value separator
                ':' => {
                    let pos = self.position;
                    self.advance();
                    self.tokens
                        .push(Token::new(TokenType::Value, pos, self.position));
                }

                // Explicit key marker
                '?' if self.flow_level == 0
                    && (self.peek_char(1).map_or(true, |c| c.is_whitespace())
                        || self.peek_char(1).is_none()) =>
                {
                    let pos = self.position;
                    self.advance();
                    self.tokens
                        .push(Token::new(TokenType::Key, pos, self.position));
                }
                '?' if self.flow_level > 0
                    && (self
                        .peek_char(1)
                        .map_or(true, |c| c.is_whitespace() || ",:]}".contains(c))
                        || self.peek_char(1).is_none()) =>
                {
                    let pos = self.position;
                    self.advance();
                    self.tokens
                        .push(Token::new(TokenType::Key, pos, self.position));
                }

                // Block entry
                '-' if self.flow_level == 0
                    && (self.peek_char(1).map_or(true, |c| c.is_whitespace())
                        || self.peek_char(1).is_none()) =>
                {
                    let pos = self.position;
                    self.advance();

                    // Check if we need to start a new block sequence
                    let last_indent = *self.indent_stack.last().unwrap();

                    if self.current_indent > last_indent {
                        // Deeper indentation - start new nested sequence
                        self.indent_stack.push(self.current_indent);
                        // Check depth limit
                        self.resource_tracker
                            .check_depth(&self.limits, self.flow_level + self.indent_stack.len())?;
                        self.tokens
                            .push(Token::simple(TokenType::BlockSequenceStart, pos));
                    } else if self.current_indent >= last_indent {
                        // Same or root level - check if we need to start a sequence
                        // We need BlockSequenceStart if we haven't started a sequence yet at this document level
                        let has_active_sequence = self
                            .tokens
                            .iter()
                            .rev()
                            .take_while(|t| {
                                !matches!(
                                    t.token_type,
                                    TokenType::StreamStart
                                        | TokenType::DocumentStart
                                        | TokenType::DocumentEnd
                                )
                            })
                            .any(|t| matches!(t.token_type, TokenType::BlockSequenceStart));

                        if !has_active_sequence {
                            // Check depth limit
                            self.resource_tracker.check_depth(
                                &self.limits,
                                self.flow_level + self.indent_stack.len(),
                            )?;
                            self.tokens
                                .push(Token::simple(TokenType::BlockSequenceStart, pos));
                        }
                    }

                    self.tokens
                        .push(Token::new(TokenType::BlockEntry, pos, self.position));

                    // After emitting BlockEntry, check if the next token is another dash (nested sequence)
                    self.skip_whitespace();
                    if self.current_char == Some('-')
                        && self.peek_char(1).map_or(true, |c| c.is_whitespace())
                    {
                        // We have a nested sequence on the same line!
                        // Track this as an inline sequence
                        self.inline_sequence_depth += 1;
                        // Also push to indent_stack to track proper nesting
                        self.indent_stack.push(self.position.column);
                        // Check depth limit
                        self.resource_tracker
                            .check_depth(&self.limits, self.flow_level + self.indent_stack.len())?;
                        self.tokens
                            .push(Token::simple(TokenType::BlockSequenceStart, self.position));
                        // Continue processing - the next iteration will handle the nested dash
                    }
                }

                // Quoted strings
                '"' => {
                    let token = self.scan_quoted_string('"')?;
                    self.tokens.push(token);
                }
                '\'' => {
                    let token = self.scan_quoted_string('\'')?;
                    self.tokens.push(token);
                }

                // Document markers (only if not a block entry)
                '-' if self.position.column == self.current_indent + 1
                    && !self.peek_char(1).map_or(true, |c| c.is_whitespace()) =>
                {
                    if let Some(token) = self.scan_document_start()? {
                        self.tokens.push(token);
                    } else if self.is_plain_scalar_start() {
                        let token = self.scan_plain_scalar()?;
                        self.tokens.push(token);
                    }
                }
                '.' if self.position.column == self.current_indent + 1 => {
                    if let Some(token) = self.scan_document_end()? {
                        self.tokens.push(token);
                    } else if self.is_plain_scalar_start() {
                        let token = self.scan_plain_scalar()?;
                        self.tokens.push(token);
                    }
                }

                // Numbers or plain scalars starting with -
                _ if ch.is_ascii_digit()
                    || (ch == '-' && self.peek_char(1).map_or(false, |c| c.is_ascii_digit())) =>
                {
                    let token = self.scan_number()?;
                    self.tokens.push(token);
                }

                // Anchors and aliases
                '&' => {
                    let token = self.scan_anchor()?;
                    self.tokens.push(token);
                }
                '*' => {
                    let token = self.scan_alias()?;
                    self.tokens.push(token);
                }

                // Block scalars
                '|' => {
                    let token = self.scan_literal_block_scalar()?;
                    self.tokens.push(token);
                }
                '>' => {
                    let token = self.scan_folded_block_scalar()?;
                    self.tokens.push(token);
                }

                // Tags
                '!' => {
                    let token = self.scan_tag()?;
                    self.tokens.push(token);
                }

                // Plain scalars
                _ if self.is_plain_scalar_start() => {
                    // Look ahead to see if this is a mapping key
                    if self.flow_level == 0 {
                        let should_start_mapping = self.check_for_mapping_ahead();
                        if should_start_mapping {
                            let last_indent = *self.indent_stack.last().unwrap();

                            // Check if we should start a new mapping
                            // Start a mapping if:
                            // 1. No mapping is active at this indentation level, OR
                            // 2. We're at a deeper indentation level (nested mapping)
                            let should_start_new_mapping = if self.current_indent > last_indent {
                                // Deeper indentation - start nested mapping
                                true
                            } else if self.current_indent == last_indent {
                                // Same indentation - check if there's an active mapping at this level
                                // We need to carefully track mapping contexts across BlockEnd tokens
                                let has_active_mapping_at_this_level =
                                    self.check_active_mapping_at_level(self.current_indent);
                                !has_active_mapping_at_this_level
                            } else {
                                // Shallower indentation - should have been handled by handle_indentation
                                false
                            };

                            if should_start_new_mapping {
                                // Start mapping before processing the key
                                self.indent_stack.push(self.current_indent);
                                // Check depth limit
                                self.resource_tracker.check_depth(
                                    &self.limits,
                                    self.flow_level + self.indent_stack.len(),
                                )?;
                                self.tokens.push(Token::simple(
                                    TokenType::BlockMappingStart,
                                    self.position,
                                ));
                            }
                        }
                    }

                    let token = self.scan_plain_scalar()?;
                    self.tokens.push(token);
                }

                _ => {
                    let context = ErrorContext::from_input(&self.input, &self.position, 2)
                        .with_suggestion("Check for valid YAML syntax characters".to_string());
                    return Err(Error::invalid_character_with_context(
                        self.position,
                        ch,
                        "YAML document",
                        context,
                    ));
                }
            }
        }

        // After processing the line, close any inline sequences
        while self.inline_sequence_depth > 0 {
            self.inline_sequence_depth -= 1;
            // Also pop from indent_stack
            if self.indent_stack.len() > 1 {
                self.indent_stack.pop();
            }
            self.tokens
                .push(Token::simple(TokenType::BlockEnd, self.position));
        }

        Ok(())
    }

    /// Scan the next token lazily
    fn scan_next_token(&mut self) -> Result<()> {
        if self.done {
            return Ok(());
        }

        // Add stream start token if this is the beginning
        if self.tokens.is_empty() {
            self.tokens
                .push(Token::simple(TokenType::StreamStart, self.position));
            return Ok(());
        }

        // Check if we're at the end of input
        if self.current_char.is_none() {
            if !self
                .tokens
                .iter()
                .any(|t| matches!(t.token_type, TokenType::StreamEnd))
            {
                self.tokens
                    .push(Token::simple(TokenType::StreamEnd, self.position));
            }
            self.done = true;
            return Ok(());
        }

        // For now, fall back to scanning all tokens at once for the lazy scanner
        // This is a simplified implementation - a full streaming parser would
        // need more sophisticated state management
        let tokens_before = self.tokens.len();
        self.scan_all_tokens()?;

        // Mark as done after scanning all tokens
        if self.tokens.len() == tokens_before {
            self.done = true;
        }

        Ok(())
    }

    /// Pre-scan all tokens (simplified approach for basic implementation)
    fn scan_all_tokens(&mut self) -> Result<()> {
        // Only add StreamStart if we don't have it yet
        if !self
            .tokens
            .iter()
            .any(|t| matches!(t.token_type, TokenType::StreamStart))
        {
            self.tokens
                .push(Token::simple(TokenType::StreamStart, self.position));
        }

        while self.current_char.is_some() {
            self.process_line()?;

            // Advance past newlines
            while let Some(ch) = self.current_char {
                if ch == '\n' || ch == '\r' {
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Close any remaining blocks
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.tokens
                .push(Token::simple(TokenType::BlockEnd, self.position));
        }

        self.tokens
            .push(Token::simple(TokenType::StreamEnd, self.position));
        self.done = true;
        Ok(())
    }

    /// Peek at a character at the given offset (can be negative)
    fn peek_char(&self, offset: isize) -> Option<char> {
        if offset >= 0 {
            let target_index = self.current_char_index + offset as usize;
            if target_index < self.char_cache.len() {
                Some(self.char_cache[target_index])
            } else {
                None
            }
        } else {
            let offset_magnitude = (-offset) as usize;
            if self.current_char_index >= offset_magnitude {
                Some(self.char_cache[self.current_char_index - offset_magnitude])
            } else {
                None
            }
        }
    }

    /// Scan an anchor token (&name)
    fn scan_anchor(&mut self) -> Result<Token> {
        let start_pos = self.position;
        self.advance(); // Skip '&'

        let name = self.scan_identifier()?;
        if name.is_empty() {
            let context = ErrorContext::from_input(&self.input, &self.position, 2).with_suggestion(
                "Provide a valid anchor name after &, e.g., &anchor_name".to_string(),
            );
            return Err(Error::scan_with_context(
                self.position,
                "Anchor name cannot be empty",
                context,
            ));
        }

        // Track anchor for resource limits
        self.resource_tracker.add_anchor(&self.limits)?;

        Ok(Token::new(
            TokenType::Anchor(name),
            start_pos,
            self.position,
        ))
    }

    /// Scan an alias token (*name)
    fn scan_alias(&mut self) -> Result<Token> {
        let start_pos = self.position;
        self.advance(); // Skip '*'

        let name = self.scan_identifier()?;
        if name.is_empty() {
            let context = ErrorContext::from_input(&self.input, &self.position, 2).with_suggestion(
                "Provide a valid alias name after *, e.g., *alias_name".to_string(),
            );
            return Err(Error::scan_with_context(
                self.position,
                "Alias name cannot be empty",
                context,
            ));
        }

        Ok(Token::new(TokenType::Alias(name), start_pos, self.position))
    }

    /// Scan an identifier (used for anchor and alias names)
    fn scan_identifier(&mut self) -> Result<String> {
        let mut identifier = String::new();

        while let Some(ch) = self.current_char {
            if ch.is_alphanumeric() || ch == '_' || ch == '-' {
                identifier.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(identifier)
    }

    /// Scan a tag token (!tag or !!tag or !<verbatim>)
    fn scan_tag(&mut self) -> Result<Token> {
        let start_pos = self.position;
        self.advance(); // Skip first '!'

        let mut tag = String::from("!");

        // Check for verbatim tag format: !<tag>
        if self.current_char == Some('<') {
            tag.push('<');
            self.advance(); // Skip '<'

            // Scan until closing '>'
            while let Some(ch) = self.current_char {
                if ch == '>' {
                    tag.push(ch);
                    self.advance();
                    break;
                } else if ch.is_control() || ch.is_whitespace() {
                    return Err(Error::scan(
                        self.position,
                        "Invalid character in verbatim tag".to_string(),
                    ));
                }
                tag.push(ch);
                self.advance();
            }
        } else {
            // Check for secondary tag handle: !!
            if self.current_char == Some('!') {
                tag.push('!');
                self.advance(); // Skip second '!'
            }

            // Scan tag name/suffix
            while let Some(ch) = self.current_char {
                if ch.is_alphanumeric() || "-./_:".contains(ch) {
                    tag.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        Ok(Token::new(TokenType::Tag(tag), start_pos, self.position))
    }

    /// Scan a literal block scalar (|)
    fn scan_literal_block_scalar(&mut self) -> Result<Token> {
        let start_pos = self.position;
        self.advance(); // Skip '|'

        // Parse block scalar header (indicators like +, -, explicit indent)
        let (keep_trailing, explicit_indent) = self.scan_block_scalar_header()?;

        // Skip to next line
        self.skip_to_next_line()?;

        // Determine indentation
        let base_indent = self.current_indent;
        let content_indent = if let Some(explicit) = explicit_indent {
            base_indent + explicit
        } else {
            // Find the first non-empty content line to determine indentation
            self.find_block_scalar_indent(base_indent)?
        };

        // Collect the literal block content
        let content = self.collect_literal_block_content(content_indent, keep_trailing)?;

        Ok(Token::new(
            TokenType::BlockScalarLiteral(content),
            start_pos,
            self.position,
        ))
    }

    /// Scan a folded block scalar (>)
    fn scan_folded_block_scalar(&mut self) -> Result<Token> {
        let start_pos = self.position;
        self.advance(); // Skip '>'

        // Parse block scalar header (indicators like +, -, explicit indent)
        let (keep_trailing, explicit_indent) = self.scan_block_scalar_header()?;

        // Skip to next line
        self.skip_to_next_line()?;

        // Determine indentation
        let base_indent = self.current_indent;
        let content_indent = if let Some(explicit) = explicit_indent {
            base_indent + explicit
        } else {
            // Find the first non-empty content line to determine indentation
            self.find_block_scalar_indent(base_indent)?
        };

        // Collect the folded block content
        let content = self.collect_folded_block_content(content_indent, keep_trailing)?;

        Ok(Token::new(
            TokenType::BlockScalarFolded(content),
            start_pos,
            self.position,
        ))
    }

    /// Parse block scalar header indicators (+, -, and explicit indent)
    fn scan_block_scalar_header(&mut self) -> Result<(bool, Option<usize>)> {
        let mut keep_trailing = false;
        let mut explicit_indent: Option<usize> = None;

        // Parse indicators in any order
        while let Some(ch) = self.current_char {
            match ch {
                '+' => {
                    keep_trailing = true;
                    self.advance();
                }
                '-' => {
                    keep_trailing = false; // Strip trailing newlines
                    self.advance();
                }
                '0'..='9' => {
                    let digit = ch.to_digit(10).unwrap() as usize;
                    if explicit_indent.is_some() {
                        let context = ErrorContext::from_input(&self.input, &self.position, 2)
                            .with_suggestion(
                                "Use only one indent indicator digit in block scalar".to_string(),
                            );
                        return Err(Error::scan_with_context(
                            self.position,
                            "Multiple indent indicators in block scalar",
                            context,
                        ));
                    }
                    explicit_indent = Some(digit);
                    self.advance();
                }
                ' ' | '\t' => {
                    self.advance(); // Skip whitespace
                }
                '#' => {
                    // Skip comment to end of line
                    while let Some(ch) = self.current_char {
                        self.advance();
                        if ch == '\n' || ch == '\r' {
                            break;
                        }
                    }
                    break;
                }
                '\n' | '\r' => break,
                _ => {
                    let context = ErrorContext::from_input(&self.input, &self.position, 2)
                        .with_suggestion("Use valid block scalar indicators: | (literal), > (folded), + (keep), - (strip), or digit (indent)".to_string());
                    return Err(Error::invalid_character_with_context(
                        self.position,
                        ch,
                        "block scalar header",
                        context,
                    ));
                }
            }
        }

        Ok((keep_trailing, explicit_indent))
    }

    /// Skip whitespace and comments to the next content line
    fn skip_to_next_line(&mut self) -> Result<()> {
        while let Some(ch) = self.current_char {
            match ch {
                '\n' | '\r' => {
                    self.advance();
                    break;
                }
                ' ' | '\t' => {
                    self.advance();
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Find the content indentation for a block scalar
    fn find_block_scalar_indent(&mut self, base_indent: usize) -> Result<usize> {
        let saved_position = self.position;
        let saved_char = self.current_char;
        let saved_char_index = self.current_char_index;

        let mut content_indent = base_indent + 1; // Default minimum indent

        // Look ahead to find the first non-empty line
        while let Some(ch) = self.current_char {
            self.advance();
            if ch == '\n' || ch == '\r' {
                let line_indent = self.count_line_indent();

                // If this line has content (not just whitespace)
                if let Some(line_ch) = self.current_char {
                    if line_ch != '\n' && line_ch != '\r' {
                        if line_indent > base_indent {
                            content_indent = line_indent;
                            break;
                        }
                        // Content must be indented more than the block scalar indicator
                        content_indent = base_indent + 1;
                        break;
                    }
                }
            }
        }

        // Restore position
        self.position = saved_position;
        self.current_char = saved_char;
        self.current_char_index = saved_char_index;

        Ok(content_indent)
    }

    /// Count indentation at start of current line
    fn count_line_indent(&mut self) -> usize {
        let mut indent = 0;
        let saved_position = self.position;
        let saved_char = self.current_char;
        let saved_char_index = self.current_char_index;

        while let Some(ch) = self.current_char {
            if ch == ' ' {
                indent += 1;
                self.advance();
            } else if ch == '\t' {
                indent += 8; // Tab counts as 8 spaces
                self.advance();
            } else {
                break;
            }
        }

        // Restore position
        self.position = saved_position;
        self.current_char = saved_char;
        self.current_char_index = saved_char_index;

        indent
    }

    /// Collect content for a literal block scalar
    fn collect_literal_block_content(
        &mut self,
        content_indent: usize,
        _keep_trailing: bool,
    ) -> Result<String> {
        let mut content = String::new();

        while let Some(_) = self.current_char {
            let line_indent = self.count_line_indent();

            // Skip indentation
            for _ in 0..content_indent.min(line_indent) {
                if let Some(' ' | '\t') = self.current_char {
                    self.advance();
                }
            }

            // Collect line content
            let mut line = String::new();
            while let Some(ch) = self.current_char {
                if ch == '\n' || ch == '\r' {
                    self.advance();
                    break;
                }
                line.push(ch);
                self.advance();
            }

            // Check if we've reached the end of the block scalar
            if line_indent < content_indent && !line.trim().is_empty() {
                // This line is part of the next construct
                break;
            }

            // Add line to content (preserving literal newlines)
            content.push_str(&line);
            if self.current_char.is_some() {
                content.push('\n');
            }

            // Check for end of input or document boundaries
            if self.current_char.is_none() {
                break;
            }
        }

        Ok(content)
    }

    /// Collect content for a folded block scalar
    fn collect_folded_block_content(
        &mut self,
        content_indent: usize,
        _keep_trailing: bool,
    ) -> Result<String> {
        let mut content = String::new();
        let mut prev_was_empty = false;
        let mut first_line = true;

        while let Some(_) = self.current_char {
            let line_indent = self.count_line_indent();

            // Skip indentation
            for _ in 0..content_indent.min(line_indent) {
                if let Some(' ' | '\t') = self.current_char {
                    self.advance();
                }
            }

            // Collect line content
            let mut line = String::new();
            while let Some(ch) = self.current_char {
                if ch == '\n' || ch == '\r' {
                    self.advance();
                    break;
                }
                line.push(ch);
                self.advance();
            }

            // Check if we've reached the end of the block scalar
            if line_indent < content_indent && !line.trim().is_empty() {
                break;
            }

            let line_is_empty = line.trim().is_empty();

            if line_is_empty {
                // Empty lines are preserved as-is
                if !first_line && !prev_was_empty {
                    content.push('\n');
                }
                prev_was_empty = true;
            } else {
                // Non-empty lines are folded (joined with spaces)
                if !first_line && !prev_was_empty {
                    content.push(' '); // Fold previous line break into space
                }
                content.push_str(line.trim());
                prev_was_empty = false;
            }

            first_line = false;

            if self.current_char.is_none() {
                break;
            }
        }

        Ok(content)
    }

    /// Check if the current position is the start of a mapping key by looking ahead for ':'
    fn check_for_mapping_ahead(&self) -> bool {
        // Look ahead through the current line for a ':' character
        for i in self.current_char_index..self.char_cache.len() {
            let ch = self.char_cache[i];
            match ch {
                ':' => {
                    // Found colon, check if it's followed by whitespace or end of line
                    let next_char = self.char_cache.get(i + 1).copied();
                    return next_char.map_or(true, |c| c.is_whitespace());
                }
                '\n' | '\r' => break, // End of line, no colon found
                _ => {}
            }
        }
        false
    }

    /// Check if there's an active mapping at the specified indentation level
    /// This method properly handles BlockEnd tokens by tracking mapping start/end pairs
    fn check_active_mapping_at_level(&self, _target_indent: usize) -> bool {
        let mut mapping_depth = 0;
        let _current_mapping_indent: Option<usize> = None;

        // Walk backwards through tokens to find mapping context
        for token in self.tokens.iter().rev() {
            match &token.token_type {
                TokenType::BlockMappingStart => {
                    if mapping_depth == 0 {
                        // This is the most recent unmatched mapping start
                        // Check if it's at our target indentation level
                        // We approximate the indentation based on the indent stack when this token was created
                        return true; // Simplified: assume we found an active mapping
                    }
                    mapping_depth -= 1;
                }
                TokenType::BlockEnd => {
                    mapping_depth += 1;
                }
                TokenType::StreamStart | TokenType::DocumentStart | TokenType::DocumentEnd => {
                    // Stop at document boundaries
                    break;
                }
                _ => {}
            }
        }

        false
    }
}

impl Scanner for BasicScanner {
    fn check_token(&self) -> bool {
        // For lazy scanning: check if we have cached tokens or can generate more
        self.token_index < self.tokens.len() || !self.done
    }

    fn peek_token(&self) -> Result<Option<&Token>> {
        // This is a bit tricky with lazy scanning since peek shouldn't mutate
        // For now, return cached token if available
        Ok(self.tokens.get(self.token_index))
    }

    fn get_token(&mut self) -> Result<Option<Token>> {
        // If we need more tokens and haven't finished, scan next token
        if self.token_index >= self.tokens.len() && !self.done {
            self.scan_next_token()?;
        }

        if self.token_index < self.tokens.len() {
            let token = self.tokens[self.token_index].clone();
            self.token_index += 1;
            Ok(Some(token))
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) {
        self.token_index = 0;
        self.position = Position::start();
        self.tokens.clear();
        self.done = false;
        self.current_char = self.input.chars().next();
        self.indent_stack = vec![0];
        self.current_indent = 0;
        self.flow_level = 0;
        self.detected_indent_style = None;
        self.indent_samples.clear();
        self.previous_indent_level = 0;
        self.current_char_index = 0;
        self.current_char = self.char_cache.first().copied();
    }

    fn position(&self) -> Position {
        self.position
    }

    fn input(&self) -> &str {
        &self.input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokenization() {
        let mut scanner = BasicScanner::new("42".to_string());

        assert!(scanner.check_token());

        // StreamStart
        let token = scanner.get_token().unwrap().unwrap();
        assert!(matches!(token.token_type, TokenType::StreamStart));

        // Number
        let token = scanner.get_token().unwrap().unwrap();
        if let TokenType::Scalar(value, _) = token.token_type {
            assert_eq!(value, "42");
        } else {
            panic!("Expected scalar token");
        }

        // StreamEnd
        let token = scanner.get_token().unwrap().unwrap();
        assert!(matches!(token.token_type, TokenType::StreamEnd));
    }

    #[test]
    fn test_flow_sequence() {
        let mut scanner = BasicScanner::new("[1, 2, 3]".to_string());

        // StreamStart
        scanner.get_token().unwrap();

        // [
        let token = scanner.get_token().unwrap().unwrap();
        assert!(matches!(token.token_type, TokenType::FlowSequenceStart));

        // 1
        let token = scanner.get_token().unwrap().unwrap();
        if let TokenType::Scalar(value, _) = token.token_type {
            assert_eq!(value, "1");
        }

        // ,
        let token = scanner.get_token().unwrap().unwrap();
        assert!(matches!(token.token_type, TokenType::FlowEntry));
    }

    #[test]
    fn test_quoted_strings() {
        let mut scanner = BasicScanner::new(r#""hello world""#.to_string());

        // StreamStart
        scanner.get_token().unwrap();

        // Quoted string
        let token = scanner.get_token().unwrap().unwrap();
        if let TokenType::Scalar(value, _) = token.token_type {
            assert_eq!(value, "hello world");
        } else {
            panic!("Expected scalar token");
        }
    }

    #[test]
    fn test_comment_handling() {
        let input = r"
# Full line comment
key: value  # End of line comment
# Another comment
data: test
";
        let mut scanner = BasicScanner::new(input.to_string());

        let mut tokens = Vec::new();
        while let Ok(Some(token)) = scanner.get_token() {
            tokens.push(token);
        }

        // Should only contain YAML structure tokens, no comment tokens
        let scalar_values: Vec<String> = tokens
            .iter()
            .filter_map(|t| match &t.token_type {
                TokenType::Scalar(s, _) => Some(s.clone()),
                _ => None,
            })
            .collect();

        assert_eq!(scalar_values, vec!["key", "value", "data", "test"]);

        // Should not contain any comment tokens
        assert!(
            !tokens
                .iter()
                .any(|t| matches!(t.token_type, TokenType::Comment(_)))
        );
    }

    #[test]
    fn test_hash_in_strings() {
        let input = r#"
string1: "This has a # character"
string2: 'Also has # character'
normal: value # This is a comment
"#;
        let mut scanner = BasicScanner::new(input.to_string());

        let mut scalar_values = Vec::new();
        while let Ok(Some(token)) = scanner.get_token() {
            if let TokenType::Scalar(value, _) = token.token_type {
                scalar_values.push(value);
            }
        }

        assert!(scalar_values.contains(&"This has a # character".to_string()));
        assert!(scalar_values.contains(&"Also has # character".to_string()));
        assert!(scalar_values.contains(&"value".to_string()));
        assert!(
            !scalar_values
                .iter()
                .any(|s| s.contains("This is a comment"))
        );
    }

    #[test]
    fn test_escape_sequences() {
        // Test standard C-style escapes
        let test_cases = vec![
            (r#""Line 1\nLine 2""#, "Line 1\nLine 2"),
            (r#""Col1\tCol2""#, "Col1\tCol2"),
            (r#""First\rSecond""#, "First\rSecond"),
            (r#""Path\\to\\file""#, "Path\\to\\file"),
            (r#""He said \"Hello\"""#, "He said \"Hello\""),
            (r"'Don\'t do that'", "Don't do that"),
        ];

        for (input, expected) in test_cases {
            let mut scanner = BasicScanner::new(input.to_string());
            scanner.get_token().unwrap(); // Skip StreamStart

            if let Ok(Some(token)) = scanner.get_token() {
                if let TokenType::Scalar(value, _) = token.token_type {
                    assert_eq!(value, expected, "Failed for input: {}", input);
                } else {
                    panic!("Expected scalar token for input: {}", input);
                }
            } else {
                panic!("Failed to get token for input: {}", input);
            }
        }
    }

    #[test]
    fn test_extended_yaml_escapes() {
        // Test additional YAML escape sequences
        let test_cases = vec![
            (r#""\0""#, "\0"),   // null character
            (r#""\a""#, "\x07"), // bell
            (r#""\b""#, "\x08"), // backspace
            (r#""\f""#, "\x0C"), // form feed
            (r#""\v""#, "\x0B"), // vertical tab
            (r#""\e""#, "\x1B"), // escape
            (r#""\ ""#, " "),    // literal space
            (r#""\/"#, "/"),     // literal forward slash
        ];

        for (input, expected) in test_cases {
            let mut scanner = BasicScanner::new(input.to_string());
            scanner.get_token().unwrap(); // Skip StreamStart

            if let Ok(Some(token)) = scanner.get_token() {
                if let TokenType::Scalar(value, _) = token.token_type {
                    assert_eq!(value, expected, "Failed for input: {}", input);
                } else {
                    panic!("Expected scalar token for input: {}", input);
                }
            } else {
                panic!("Failed to get token for input: {}", input);
            }
        }
    }

    #[test]
    fn test_unknown_escape_sequences() {
        // Test that unknown escape sequences are preserved literally
        let input = r#""\z\q\8""#;
        let expected = "\\z\\q\\8"; // Should preserve backslashes for unknown escapes

        let mut scanner = BasicScanner::new(input.to_string());
        scanner.get_token().unwrap(); // Skip StreamStart

        if let Ok(Some(token)) = scanner.get_token() {
            if let TokenType::Scalar(value, _) = token.token_type {
                assert_eq!(value, expected);
            } else {
                panic!("Expected scalar token");
            }
        } else {
            panic!("Failed to get token");
        }
    }
}
