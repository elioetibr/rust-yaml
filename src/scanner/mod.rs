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

/// Block-scalar chomping mode per YAML 1.2 §8.1.1.2.
///
/// - `Strip` (`-`): drop the final line break and trailing empty lines.
/// - `Clip` (default): keep exactly one final line break, drop trailing empty lines.
/// - `Keep` (`+`): preserve the final line break and all trailing empty lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChompingMode {
    Strip,
    Clip,
    Keep,
}

/// Apply chomping mode to a block-scalar tail.
///
/// The collectors emit a `\n` for every line (content or blank). This helper
/// trims that tail according to spec §8.1.1.2:
///
/// - **Strip:** remove every trailing `\n`.
/// - **Clip:** keep exactly one trailing `\n` if content exists; drop the rest.
///   Empty input stays empty.
/// - **Keep:** preserve everything.
fn apply_chomping(mut s: String, mode: ChompingMode) -> String {
    match mode {
        ChompingMode::Keep => s,
        ChompingMode::Strip => {
            while s.ends_with('\n') {
                s.pop();
            }
            s
        }
        ChompingMode::Clip => {
            // Strip trailing newlines. If anything remains, restore one.
            // §8.1.1.2: clip keeps the final line break only when the
            // scalar has actual content (yaml-test-suite K858: an empty
            // clip scalar `>` is `""`, not `"\n"`).
            while s.ends_with('\n') {
                s.pop();
            }
            if !s.is_empty() {
                s.push('\n');
            }
            s
        }
    }
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
    // Track compact-notation sequences (where `-` is at the same indent as
    // the parent mapping keys). These are NOT on indent_stack, so we need
    // separate tracking to know when to emit BlockEnd for them.
    compact_sequence_indents: Vec<usize>,
    // Parallel to indent_stack: true when the entry was pushed by a block
    // sequence, false when by a mapping. Lets us distinguish "continuing a
    // regular sequence" from "starting a compact sequence at same indent".
    indent_is_sequence: Vec<bool>,
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
                compact_sequence_indents: Vec::new(),
                indent_is_sequence: vec![false],
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
            compact_sequence_indents: Vec::new(),
            indent_is_sequence: vec![false],
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
        // In flow context: if there is a non-trivial enclosing block
        // (indent_stack has more than the implicit root level), each
        // continuation line that has content must be indented MORE than
        // that enclosing block's indent. \`flow: [a,\\nb,c]\` with \`b\`
        // at col 1 violates this rule because the block mapping enclosing
        // \`flow:\` sits at indent 0 (yaml-test-suite 9C9N).
        //
        // Top-level flow (no enclosing block; indent_stack is just \[0\])
        // is exempt — `[a,\\nb]` is fine there because the flow content
        // isn't nested inside any block (yaml-test-suite 4ZYM).
        if self.flow_level > 0 {
            if self.indent_stack.len() > 1 || !self.compact_sequence_indents.is_empty() {
                let mut probe = 0usize;
                let mut i = self.current_char_index;
                while i < self.char_cache.len() {
                    match self.char_cache[i] {
                        ' ' => {
                            probe += 1;
                            i += 1;
                        }
                        '\t' => i += 1,
                        _ => break,
                    }
                }
                let has_content = self
                    .char_cache
                    .get(i)
                    .map_or(false, |c| !matches!(c, '\n' | '\r'));
                // A line that begins with the matching flow closer
                // (\`]\` / \`}\`) is allowed at the parent indent — it
                // closes the flow collection, not adds content
                // (yaml-test-suite NKF9 trailing-line \`}\` at col 1).
                let is_closer = matches!(self.char_cache.get(i).copied(), Some(']' | '}'));
                if has_content && !is_closer {
                    let parent_indent = self.indent_stack.last().copied().unwrap_or(0);
                    if probe <= parent_indent {
                        return Err(Error::scan(
                            self.position,
                            "Flow content line is not indented enough".to_string(),
                        ));
                    }
                }
            }
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

        // YAML 1.2 §6.1 does NOT require all indents to be multiples
        // of a single "indent width". Siblings must share a column;
        // children must indent further; but any positive amount works
        // (e.g. `key:\n  child:\n   grandchild:` with widths 2, 1
        // is legal). The earlier strict-multiple-of-N check rejected
        // valid spec fixtures like 6HB6, 8G76, A2M4, P94K, Q9WF,
        // UGM3. We rely on the indent_stack-driven open/close logic
        // (and the per-block "more than parent" rule enforced
        // elsewhere) to catch genuine mis-indentation.

        // Update previous indentation level for future comparisons
        if indent > 0 {
            self.previous_indent_level = indent;
        }

        // Update current indentation level
        self.current_indent = indent;

        // Close compact-notation sequences whose scope ends at this line.
        // A compact sequence (where `-` shares the indent of the parent
        // mapping keys) ends when the next content line at that indent is
        // NOT a block entry (`- `).  We must emit the sequence's BlockEnd
        // BEFORE popping the indent_stack so that the nesting order is
        // correct (sequence closes before its parent mapping).
        let has_content =
            self.current_char.is_some() && !matches!(self.current_char, Some('\n' | '\r' | '#'));
        if has_content {
            let is_block_entry = self.current_char == Some('-')
                && self.peek_char(1).map_or(true, |c| c.is_whitespace());
            while let Some(&seq_indent) = self.compact_sequence_indents.last() {
                if indent < seq_indent || (indent == seq_indent && !is_block_entry) {
                    self.compact_sequence_indents.pop();
                    self.tokens
                        .push(Token::simple(TokenType::BlockEnd, line_start_pos));
                } else {
                    break;
                }
            }
        }

        // Check if we need to emit block end tokens for decreased indentation
        let pre_pop_top = self.indent_stack.last().copied().unwrap_or(0);
        while let Some(&last_indent) = self.indent_stack.last() {
            if indent < last_indent && last_indent > 0 {
                self.indent_stack.pop();
                self.indent_is_sequence.pop();
                self.tokens
                    .push(Token::simple(TokenType::BlockEnd, line_start_pos));
            } else {
                break;
            }
        }

        // §6.1: after a dedent, the new line's indent must match some
        // existing container level — keys/items at a sibling level
        // must share a column. Landing at a column that is between
        // two stack levels (e.g. parent at 0, just-closed at 3, new
        // line at 1) is invalid because no open mapping/sequence sits
        // at indent 1 (yaml-test-suite DMG6, N4JP).
        //
        // The check applies only when:
        //   * we actually dedented (pre-pop top was deeper than now),
        //   * the new line has content (the next char is not blank /
        //     newline / EOF / comment),
        //   * indent doesn't match the new top.
        if pre_pop_top > 0
            && pre_pop_top > self.indent_stack.last().copied().unwrap_or(0)
            && self
                .current_char
                .map_or(false, |c| !matches!(c, '\n' | '\r' | '#'))
            && indent != self.indent_stack.last().copied().unwrap_or(0)
        {
            // Allow if indent is a valid deeper level — e.g.
            // sibling at depth then deeper child — but for the
            // dedent path indent must equal a known stack level.
            return Err(Error::scan(
                self.position,
                format!(
                    "Indentation {indent} doesn't match any open container (expected {} or deeper)",
                    self.indent_stack.last().copied().unwrap_or(0)
                ),
            ));
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
        // Prevent mixed indentation (tabs + spaces on same line).
        // Carve-out: a tab AFTER one or more spaces and BEFORE
        // value-position content (not a key) is content-area
        // whitespace, not indentation. \`foo:\\n \\tbar\` — the 1
        // space is indent, the tab is a separator before \`bar\`
        // which is the value of \`foo:\` (yaml-test-suite DK95/00).
        if has_tabs && has_spaces {
            // Peek ahead: if the content after the tab+spaces area
            // contains a key marker (`: ` or `:`+EOL), treat as
            // indentation (invalid). Otherwise it's a value line.
            let looks_like_key = self.line_after_indent_is_implicit_key();
            if looks_like_key {
                let context =
                    crate::error::ErrorContext::from_input(&self.input, &self.position, 4)
                        .with_suggestion(
                            "Use either tabs OR spaces for indentation, not both".to_string(),
                        );
                return Err(Error::invalid_character_with_context(
                    self.position,
                    '\t',
                    "mixed indentation",
                    context,
                ));
            }
        }
        // §6.1: indentation must be space characters only. Pure-tab
        // indentation (\`\\tkey: value\`) is invalid (yaml-test-suite
        // 4EJS). Two carve-outs:
        //   * The mixed case is caught by the earlier branch.
        //   * Tabs before a flow-collection opener (\`\\t[\`, \`\\t{\`)
        //     at the root are not "block indentation" — there's no
        //     enclosing block — and yaml-test-suite 6CA3 / Q5MG accept
        //     them.
        if has_tabs && !has_spaces && !matches!(self.current_char, Some('[' | '{')) {
            let context = crate::error::ErrorContext::from_input(&self.input, &self.position, 4)
                .with_suggestion("Use space characters for indentation".to_string());
            return Err(Error::invalid_character_with_context(
                self.position,
                '\t',
                "indentation",
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

            // YAML 1.2 §6.1 does NOT require all indents to be multiples
            // of a single "indent width". Sibling lines must share a
            // column and children must indent deeper than parents, but
            // any positive amount works. The "multiple of N" check
            // rejected valid spec fixtures (6HB6, M5C3, P94K, Q9WF,
            // RZP5, UGM3, XW4D, A2M4); we rely on the indent_stack
            // open/close logic for genuine mis-indentation. The detected
            // style is still recorded for later style-preservation use
            // (e.g. emitter), it just no longer drives validation.
            // self.validate_indentation_consistency(current_indent)?;
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
            // Pure indicators — never start a plain scalar.
            ',' | '[' | ']' | '{' | '}' | '#' | '&' | '*' | '!' | '|' | '>' | '\'' | '"' | '%'
            | '@' | '`' => false,
            // YAML 1.2 §7.3.3: `?`, `:`, `-` may start a plain scalar when
            // the next character is non-whitespace (and, in flow context,
            // not a flow indicator). Otherwise they act as indicators
            // (complex-key marker / value separator / block-entry marker).
            '?' | ':' | '-' => match self.peek_char(1) {
                None => false,
                Some(c) if c.is_whitespace() => false,
                Some(c) if self.flow_level > 0 && ",[]{}".contains(c) => false,
                Some(_) => true,
            },
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

    /// Normalize a scalar value based on YAML rules.
    ///
    /// The scanner preserves the original text of plain scalars. Type
    /// resolution (including version-aware bool/null mapping) happens in
    /// the composer (see `crate::resolver::resolve_plain_scalar`). This
    /// preserves enough information for the composer to apply the
    /// YAML 1.1 vs 1.2 distinction and for round-trip emitters to
    /// recover the original spelling.
    fn normalize_scalar(value: String) -> String {
        value
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
        let start_col = start_pos.column;
        let mut value = String::new();
        let mut multi_line = false;

        loop {
            // Scan content on the current line until we hit a stop condition.
            while let Some(ch) = self.current_char {
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
                    match ch {
                        // Same line-break handling as block context: stop
                        // collecting raw content at `\n`/`\r`, then let the
                        // outer fold logic decide whether the next line
                        // continues this scalar (yaml-test-suite 8KB6,
                        // 8UDB, 9BXH).
                        '\n' | '\r' => break,
                        ',' | '[' | ']' | '{' | '}' => break,
                        // In flow context, `:` is a key-value separator
                        // when followed by whitespace OR any flow indicator
                        // (`,`, `[`, `]`, `{`, `}`). Tracked by yaml-test-
                        // suite FRK4 (`{ ? foo :, ... }`).
                        ':' if self
                            .peek_char(1)
                            .map_or(true, |c| c.is_whitespace() || ",[]{}".contains(c)) =>
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

            // If we didn't stop at a newline, this scalar is complete.
            if !matches!(self.current_char, Some('\n' | '\r')) {
                break;
            }

            // Per §6.5 line folding, trailing whitespace on the line is
            // dropped (it gets replaced by the fold separator that the
            // next continuation block emits).
            while matches!(value.chars().last(), Some(' ' | '\t')) {
                value.pop();
            }

            // YAML 1.2 §6.5 / §7.3.3: try to fold continuation lines into
            // the same plain scalar. A continuation line must be:
            //   * indented strictly more than the scalar's start column,
            //   * not a document marker (`---` / `...`),
            //   * not a comment-only line,
            //   * not empty-with-EOF.
            // Save state for backtracking if continuation isn't allowed.
            let saved_position = self.position;
            let saved_index = self.current_char_index;
            let saved_char = self.current_char;

            // Count physical newlines we skip; whitespace within the lines
            // is also consumed.
            let mut newlines = 0usize;
            loop {
                match self.current_char {
                    Some('\n') => {
                        newlines += 1;
                        self.advance();
                    }
                    Some('\r') => {
                        self.advance();
                    }
                    Some(' ' | '\t') => {
                        self.advance();
                    }
                    _ => break,
                }
            }

            let next_col = self.position.column;
            let next_ch = self.current_char;
            let is_doc_marker = matches!(next_ch, Some('-' | '.'))
                && self.peek_char(1) == next_ch
                && self.peek_char(2) == next_ch
                && self.peek_char(3).map_or(true, |c| c.is_whitespace());

            // Continuation column rule:
            //   * Flow context: no column rule, only flow indicators
            //     terminate (8KB6, 8UDB, 9BXH).
            //   * Block context: must be strictly deeper than the parent
            //     block's key column. The parent indent is the max of
            //     `indent_stack.last()` (block mapping/sequence indent)
            //     and `compact_sequence_indents.last()` — the latter
            //     tracks sequences opened compactly (e.g. `? - x` where
            //     the dash didn't push to indent_stack). Without the
            //     compact-stack check, `? - Detroit Tigers\n  - Chicago`
            //     would fold both lines into one scalar (yaml-test-
            //     suite M5DY).
            //     Fall back to `next_col >= start_col` for top-level
            //     scalars where there's no enclosing block.
            let column_ok = if self.flow_level > 0 {
                true
            } else {
                let block_indent = self.indent_stack.last().copied().unwrap_or(0);
                let compact_indent = self.compact_sequence_indents.last().copied().unwrap_or(0);
                let parent_indent = block_indent.max(compact_indent);
                next_col >= parent_indent + 2 || next_col >= start_col
            };
            let can_continue = next_ch.is_some()
                && !matches!(next_ch, Some('\n' | '\r' | '#'))
                && column_ok
                && !is_doc_marker
                && !(self.flow_level > 0 && matches!(next_ch, Some(',' | ']' | '}')));

            if !can_continue {
                self.position = saved_position;
                self.current_char_index = saved_index;
                self.current_char = saved_char;
                break;
            }

            // Append fold separator: single newline → space; N>1 newlines
            // collapse to N-1 retained newlines (YAML §6.5 line folding).
            if newlines <= 1 {
                value.push(' ');
            } else {
                for _ in 0..(newlines - 1) {
                    value.push('\n');
                }
            }
            multi_line = true;
        }

        // YAML 1.2 §8.1.3: implicit keys must be on a single line. If the
        // plain scalar folded across line breaks AND the next non-
        // whitespace char is `:` (key-value separator), it's about to be
        // used as an implicit key — reject (yaml-test-suite G7JE).
        if multi_line && self.flow_level == 0 {
            let mut off = 0isize;
            while matches!(self.peek_char(off), Some(' ' | '\t')) {
                off += 1;
            }
            if self.peek_char(off) == Some(':') {
                return Err(Error::scan(
                    self.position,
                    "Multi-line plain scalar may not be used as an implicit key".to_string(),
                ));
            }
        }

        self.resource_tracker
            .check_string_length(&self.limits, value.len())?;

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
        let mut closed = false;
        let mut multi_line = false;
        // High-water mark of bytes contributed by escape sequences. The
        // trailing-whitespace strip at fold time must not pop past it,
        // because an escape-produced \t / space is literal content
        // (yaml-test-suite DE56/00, DE56/01).
        let mut escape_end: usize = 0;

        while let Some(ch) = self.current_char {
            if ch == quote_char {
                // YAML 1.2 §7.3.2 (Single-Quoted): `''` is the only escape,
                // collapsing to a single `'`. Detect that here BEFORE
                // treating the quote as the closing delimiter.
                if quote_char == '\'' && self.peek_char(1) == Some('\'') {
                    value.push('\'');
                    self.advance();
                    self.advance();
                    continue;
                }
                self.advance(); // Skip closing quote
                closed = true;
                break;
            } else if ch == '\\' && quote_char == '"' {
                self.advance();
                if let Some(escaped) = self.current_char {
                    match escaped {
                        // YAML 1.2 §5.7 double-quoted escape allowlist.
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        'r' => value.push('\r'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        '0' => value.push('\0'),
                        'a' => value.push('\x07'),
                        'b' => value.push('\x08'),
                        'f' => value.push('\x0C'),
                        'v' => value.push('\x0B'),
                        'e' => value.push('\x1B'),
                        ' ' => value.push(' '),
                        '/' => value.push('/'),
                        'N' => value.push('\u{0085}'),
                        '_' => value.push('\u{00A0}'),
                        'L' => value.push('\u{2028}'),
                        'P' => value.push('\u{2029}'),
                        '\n' => {
                            // Escaped line break (§7.3.2): the newline is
                            // dropped AND leading whitespace on the next
                            // line is excluded from the content.
                            self.advance();
                            while matches!(self.current_char, Some(' ' | '\t')) {
                                self.advance();
                            }
                            continue;
                        }
                        '\t' => value.push('\t'), // literal tab after `\` → tab (yaml-test-suite 3RLN/DE56)
                        // Hex / Unicode escapes per YAML 1.2 §5.7:
                        //   \xNN     — 2 hex digits, codepoint  ≤ 0xFF
                        //   \uNNNN   — 4 hex digits, codepoint  ≤ 0xFFFF
                        //   \UNNNNNNNN — 8 hex digits, full Unicode codepoint
                        'x' | 'u' | 'U' => {
                            let n = match escaped {
                                'x' => 2,
                                'u' => 4,
                                _ => 8,
                            };
                            self.advance(); // consume the x/u/U
                            let mut codepoint: u32 = 0;
                            for _ in 0..n {
                                let c = self.current_char.ok_or_else(|| {
                                    Error::scan(
                                        self.position,
                                        format!("Truncated \\{escaped} escape"),
                                    )
                                })?;
                                let d = c.to_digit(16).ok_or_else(|| {
                                    Error::scan(
                                        self.position,
                                        format!("Invalid hex digit `{c}` in \\{escaped} escape"),
                                    )
                                })?;
                                codepoint = (codepoint << 4) | d;
                                self.advance();
                            }
                            let ch = char::from_u32(codepoint).ok_or_else(|| {
                                Error::scan(
                                    self.position,
                                    format!("Invalid Unicode codepoint U+{codepoint:X}"),
                                )
                            })?;
                            value.push(ch);
                            escape_end = value.len();
                            continue; // already advanced past hex digits
                        }
                        // Everything else is invalid per spec.
                        _ => {
                            return Err(Error::scan(
                                self.position,
                                format!("Invalid escape sequence: \\{escaped}"),
                            ));
                        }
                    }
                    escape_end = value.len();
                    self.advance();
                }
            } else if ch == '\\' {
                // Single-quoted strings have no backslash escapes — `\` is
                // a literal character. (Single-quote escape is `''`.)
                value.push(ch);
                self.advance();
            } else if ch == '\n' || ch == '\r' {
                // YAML 1.2 §7.3.2 (double-quoted) / §7.3.3 (single-quoted)
                // line folding: a single newline within a quoted scalar
                // folds to a space; N>1 consecutive newlines retain N-1;
                // leading whitespace on the continuation line is excluded.
                let mut newlines = 0usize;
                // §6.1: tabs cannot be indentation. A continuation
                // line that BEGINS with a tab (no leading spaces) in
                // an enclosing block context is invalid (yaml-test-
                // suite DK95/01). Tabs that appear AFTER spaces in
                // the same indent area are content, not indentation.
                let mut just_after_newline = false;
                while let Some(c) = self.current_char {
                    match c {
                        '\n' => {
                            newlines += 1;
                            multi_line = true;
                            self.advance();
                            just_after_newline = true;
                        }
                        '\r' => {
                            self.advance();
                        }
                        ' ' => {
                            self.advance();
                            just_after_newline = false;
                        }
                        '\t' if just_after_newline
                            && self.flow_level == 0
                            && (self.indent_stack.len() > 1
                                || !self.compact_sequence_indents.is_empty()) =>
                        {
                            return Err(Error::scan(
                                self.position,
                                "Tab cannot serve as indentation of quoted scalar continuation"
                                    .to_string(),
                            ));
                        }
                        '\t' => {
                            self.advance();
                        }
                        _ => break,
                    }
                }
                // §8.1.4: a multi-line quoted scalar inside a block
                // context must indent each continuation more than the
                // enclosing block. \`quoted: "a\\nb"\` with \`b\` at col 1
                // violates the rule because \`quoted:\` sits at indent 0
                // (yaml-test-suite QB6E). Only fires when there IS an
                // enclosing block (indent_stack > [0] or compact-seq
                // active) — top-level quoted scalars with continuation
                // at col 1 are legal.
                if newlines > 0
                    && self.flow_level == 0
                    && (self.indent_stack.len() > 1 || !self.compact_sequence_indents.is_empty())
                    && !matches!(self.current_char, None | Some('\n' | '\r'))
                {
                    let parent_indent = self.indent_stack.last().copied().unwrap_or(0);
                    let indent = self.position.column.saturating_sub(1);
                    if indent <= parent_indent {
                        return Err(Error::scan(
                            self.position,
                            "Quoted scalar continuation line is not indented enough".to_string(),
                        ));
                    }
                }
                // §6.8: a doc-start/end marker (`---` or `...`) at
                // column 1 always terminates the current document.
                // Encountering one inside an unterminated quoted
                // scalar is invalid — the quote escapes nothing past
                // the doc boundary (yaml-test-suite 5TRB, RXY3,
                // 9MQT/01).
                if self.position.column == 1 {
                    let next3: String = self
                        .char_cache
                        .get(self.current_char_index..self.current_char_index + 3)
                        .map(|s| s.iter().collect())
                        .unwrap_or_default();
                    if (next3 == "---" || next3 == "...")
                        && self
                            .char_cache
                            .get(self.current_char_index + 3)
                            .map_or(true, |c| c.is_whitespace())
                    {
                        return Err(Error::scan(
                            self.position,
                            format!(
                                "Document {} marker `{}` inside quoted scalar",
                                if next3 == "---" { "start" } else { "end" },
                                next3
                            ),
                        ));
                    }
                }
                // Drop trailing whitespace on the prior line (the bytes
                // we already pushed) before applying the fold. Don't
                // strip past `escape_end` — escape-produced whitespace
                // is literal content, not "trailing" line whitespace.
                while value.len() > escape_end && matches!(value.chars().last(), Some(' ' | '\t')) {
                    value.pop();
                }
                if newlines <= 1 {
                    value.push(' ');
                } else {
                    for _ in 0..(newlines - 1) {
                        value.push('\n');
                    }
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
        if !closed {
            return Err(Error::scan(
                self.position,
                format!(
                    "Unclosed {} quoted string",
                    if quote_char == '"' {
                        "double"
                    } else {
                        "single"
                    }
                ),
            ));
        }

        self.resource_tracker
            .check_string_length(&self.limits, value.len())?;

        // YAML 1.2 §7.3.1 / §7.3.2: after the closing quote, the rest of
        // the line (or sub-expression in flow context) must be empty save
        // for a separator. Skip horizontal whitespace and look at the next
        // non-space char; if it's content rather than `,`/`:`/`}`/`]`/`#`/
        // newline/EOF, it's a trailing-content error (yaml-test-suite
        // Q4CL: `"quoted2" trailing content`).
        {
            let mut offset = 0isize;
            let mut saw_space = false;
            while matches!(self.peek_char(offset), Some(' ' | '\t')) {
                saw_space = true;
                offset += 1;
            }
            let next = self.peek_char(offset);
            // A `#` is a comment indicator ONLY when preceded by whitespace
            // (YAML 1.2 §6.6); `"value"#cmt` is invalid.
            let ok = match next {
                None => true,
                Some('#') => saw_space,
                Some(c) => matches!(c, ',' | ':' | '}' | ']' | '\n' | '\r'),
            };
            if !ok {
                return Err(Error::scan(
                    self.position,
                    format!("Unexpected `{}` after quoted scalar", next.unwrap_or(' ')),
                ));
            }
            // YAML 1.2 §8.1.3: implicit keys must be on a single line.
            // If the scalar folded across line breaks AND the next non-
            // whitespace char is `:` (key-value separator), the scalar
            // is being used as an implicit key — error.
            if multi_line && self.flow_level == 0 && next == Some(':') {
                return Err(Error::scan(
                    self.position,
                    "Multi-line quoted scalar may not be used as an implicit key".to_string(),
                ));
            }
        }

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
            // Doc markers are invalid inside flow collections.
            if self.flow_level > 0 {
                return Err(Error::scan(
                    self.position,
                    "`---` document-start marker is not allowed inside a flow collection"
                        .to_string(),
                ));
            }
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

            // YAML 1.2 §6.8.1: the directive line must end after the
            // version (modulo whitespace and an optional comment). Extra
            // tokens (e.g. `%YAML 1.2 foo`) are invalid — yaml-test-suite
            // H7TQ. Also `%YAML 1.1#...` (yaml-test-suite MUS6/00) needs
            // whitespace before `#`.
            let mut saw_space = false;
            while matches!(self.current_char, Some(' ' | '\t')) {
                saw_space = true;
                self.advance();
            }
            match self.current_char {
                None | Some('\n' | '\r') => {}
                Some('#') if saw_space => {
                    while let Some(ch) = self.current_char {
                        if ch == '\n' || ch == '\r' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some(c) => {
                    return Err(Error::scan(
                        self.position,
                        format!("Unexpected `{c}` after %YAML directive"),
                    ));
                }
            }

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
            // Doc markers are invalid inside flow collections.
            if self.flow_level > 0 {
                return Err(Error::scan(
                    self.position,
                    "`...` document-end marker is not allowed inside a flow collection".to_string(),
                ));
            }
            let start_pos = self.position;
            self.advance(); // .
            self.advance(); // .
            self.advance(); // .

            // YAML 1.2 §6.4: `...` must be followed only by whitespace or
            // end-of-line (comments allowed). Inline content after `...`
            // is invalid (yaml-test-suite 3HFZ).
            while let Some(ch) = self.current_char {
                match ch {
                    ' ' | '\t' => {
                        self.advance();
                    }
                    '\n' | '\r' | '#' => break,
                    _ => {
                        return Err(Error::scan(
                            self.position,
                            "Content after `...` document-end marker is invalid".to_string(),
                        ));
                    }
                }
            }

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

            // YAML 1.2 §6.8.4: a YAML processor MUST ignore directives it
            // does not recognize. Skip the line silently — parsing continues
            // with whatever follows on the next line.
            if self.current_char == Some('%') {
                while let Some(ch) = self.current_char {
                    if ch == '\n' || ch == '\r' {
                        break;
                    }
                    self.advance();
                }
                return Ok(());
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
                    // YAML 1.2 §6.6: a comment must be preceded by whitespace
                    // OR be at the start of a line. Inputs like `,#invalid`
                    // (yaml-test-suite CVW2) are not valid comments.
                    let prev = self.peek_char(-1);
                    let at_line_start = self.position.column == 1;
                    let preceded_by_space = prev.map_or(true, |c| c.is_whitespace());
                    if !at_line_start && !preceded_by_space {
                        return Err(Error::scan(
                            self.position,
                            "Comment `#` must be preceded by whitespace".to_string(),
                        ));
                    }
                    if self.preserve_comments {
                        let comment_token = self.scan_comment()?;
                        self.tokens.push(comment_token);
                    } else {
                        while let Some(ch) = self.current_char {
                            if ch == '\n' || ch == '\r' {
                                break;
                            }
                            self.advance();
                        }
                    }
                    break;
                }

                // Flow indicators. §7.4 allows a flow collection as
                // the implicit key of a block mapping (`[a]: b`,
                // `{x: y}: z`). When the flow-open is at line-start
                // (block context) and a `:` follows on the same line,
                // open the wrapping block mapping at the column of the
                // flow-open token, just as we do for line-start
                // properties (yaml-test-suite LX3P, 4FJ6, M2N8/01).
                '[' => {
                    if self.flow_level == 0
                        && self.position.column == self.current_indent + 1
                        && self.check_for_mapping_ahead()
                    {
                        self.maybe_open_block_mapping_for_key()?;
                    }
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
                    // YAML 1.2 §7.4: `]` is only valid inside an open
                    // flow sequence. Stray `]` is a syntax error
                    // (yaml-test-suite 4H7K).
                    if self.flow_level == 0 {
                        let context = ErrorContext::from_input(&self.input, &self.position, 2)
                            .with_suggestion(
                                "Remove the extra `]` or open a flow sequence with `[` first"
                                    .to_string(),
                            );
                        return Err(Error::scan_with_context(
                            self.position,
                            "Unexpected `]` outside flow context",
                            context,
                        ));
                    }
                    let pos = self.position;
                    self.advance();
                    self.flow_level -= 1;
                    self.tokens
                        .push(Token::new(TokenType::FlowSequenceEnd, pos, self.position));
                }
                '{' => {
                    if self.flow_level == 0
                        && self.position.column == self.current_indent + 1
                        && self.check_for_mapping_ahead()
                    {
                        self.maybe_open_block_mapping_for_key()?;
                    }
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
                    if self.flow_level == 0 {
                        let context = ErrorContext::from_input(&self.input, &self.position, 2)
                            .with_suggestion(
                                "Remove the extra `}` or open a flow mapping with `{` first"
                                    .to_string(),
                            );
                        return Err(Error::scan_with_context(
                            self.position,
                            "Unexpected `}` outside flow context",
                            context,
                        ));
                    }
                    let pos = self.position;
                    self.advance();
                    self.flow_level -= 1;
                    self.tokens
                        .push(Token::new(TokenType::FlowMappingEnd, pos, self.position));
                }
                ',' => {
                    // §7.4: \`,\` is a flow indicator. Outside flow
                    // context it's not meaningful as a structural
                    // separator (yaml-test-suite U99R: \`- !!str, xxx\`
                    // — the comma after a tag in block context is
                    // invalid).
                    if self.flow_level == 0 {
                        return Err(Error::scan(
                            self.position,
                            "Unexpected `,` outside flow context".to_string(),
                        ));
                    }
                    let pos = self.position;
                    self.advance();
                    self.tokens
                        .push(Token::new(TokenType::FlowEntry, pos, self.position));
                }

                // Key-value separator. YAML 1.2 §7.3.3 / §7.4:
                //   * Block context: `:` separates key from value only when
                //     followed by whitespace / EOF — otherwise it's part of
                //     a plain scalar (e.g. `:foo`, `URL://path`).
                //   * Flow context: same, plus `:` may be adjacent to a
                //     value when the previous token completed a key node
                //     (quoted/plain scalar, alias, or closed flow
                //     collection) — see yaml-test-suite 5MUD, 5T43.
                ':' if self.peek_char(1).map_or(true, |c| {
                    c.is_whitespace() || (self.flow_level > 0 && ",[]{}".contains(c))
                }) || (self.flow_level > 0
                    && matches!(
                        self.tokens.last().map(|t| &t.token_type),
                        Some(
                            TokenType::Scalar(_, _)
                                | TokenType::Alias(_)
                                | TokenType::FlowMappingEnd
                                | TokenType::FlowSequenceEnd
                        )
                    )) =>
                {
                    // §6.2: a \`:\` at line-start (the explicit-value
                    // counterpart of an explicit \`?\` key) must be
                    // followed by a SPACE — a tab as separator is
                    // invalid (yaml-test-suite Y79Y/007, /009).
                    if self.flow_level == 0
                        && self.position.column == self.current_indent + 1
                        && self.peek_char(1) == Some('\t')
                    {
                        return Err(Error::scan(
                            self.position,
                            "Tab cannot follow line-start `:` as explicit-value separator"
                                .to_string(),
                        ));
                    }
                    // §8.22: an implicit key in block context must fit
                    // on a single line. If the previous token is a
                    // flow-collection close whose matching open is on
                    // a different line, the flow node spans multiple
                    // lines and can't serve as the key (yaml-test-
                    // suite C2SP \`[23\\n]: 42\`).
                    if self.flow_level == 0 {
                        let mut is_flow_close = false;
                        let mut close_end_line = 0;
                        if let Some(last) = self.tokens.last() {
                            if matches!(
                                last.token_type,
                                TokenType::FlowSequenceEnd | TokenType::FlowMappingEnd
                            ) {
                                is_flow_close = true;
                                close_end_line = last.end_position.line;
                            }
                        }
                        if is_flow_close {
                            let mut depth = 0i32;
                            let mut open_idx: Option<usize> = None;
                            for (idx, t) in self.tokens.iter().enumerate().rev() {
                                match &t.token_type {
                                    TokenType::FlowSequenceEnd | TokenType::FlowMappingEnd => {
                                        depth += 1;
                                    }
                                    TokenType::FlowSequenceStart | TokenType::FlowMappingStart => {
                                        depth -= 1;
                                        if depth == 0 {
                                            open_idx = Some(idx);
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            if let Some(oi) = open_idx {
                                let open_line = self.tokens[oi].start_position.line;
                                // If a `?` (Key) token precedes the
                                // matching flow open on the same line
                                // as the key, the key is explicit and
                                // may span lines (yaml-test-suite M5DY
                                // \`? [ ...spans... ]: [ ... ]\`).
                                let key_marker_before = self.tokens[..oi].iter().rev().any(|t| {
                                    matches!(t.token_type, TokenType::Key)
                                        && t.start_position.line == open_line
                                });
                                if !key_marker_before && open_line != close_end_line {
                                    return Err(Error::scan(
                                        self.position,
                                        "Implicit key in block context: flow collection key spans multiple lines"
                                            .to_string(),
                                    ));
                                }
                            }
                        }
                    }
                    let pos = self.position;
                    self.advance();
                    self.tokens
                        .push(Token::new(TokenType::Value, pos, self.position));
                }

                // §6.2: the explicit-key marker \`?\` must be followed
                // by a SPACE (or EOL), not a tab. Tab as separator
                // after \`?\` is invalid (yaml-test-suite Y79Y/006, /008).
                '?' if self.flow_level == 0 && self.peek_char(1) == Some('\t') => {
                    return Err(Error::scan(
                        self.position,
                        "Tab cannot follow `?` as block-key separator".to_string(),
                    ));
                }

                // Explicit key marker. An indented `?` at line-start
                // (e.g. `mapping:\\n  ? key`) opens an implicit block
                // mapping at this column — same as a line-start scalar
                // key. Without this, scan_plain_scalar wouldn't see
                // the inner mapping's indent and would wrongly fold
                // the key content into a multi-line scalar
                // (yaml-test-suite S9E8, KK5P).
                '?' if self.flow_level == 0
                    && (self.peek_char(1).map_or(true, |c| c.is_whitespace())
                        || self.peek_char(1).is_none()) =>
                {
                    if self.position.column == self.current_indent + 1 {
                        self.maybe_open_block_mapping_for_key()?;
                    }
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
                    // A block-entry \`-\` immediately after a flow
                    // collection's close (\`}\`, \`]\`) ON THE SAME LINE
                    // is invalid — no separator between the closed
                    // flow node and the next sibling (yaml-test-suite
                    // P2EQ \`- { y: z }- invalid\`). The same-line guard
                    // is essential — a \`}\` on a previous line with a
                    // new \`-\` on the next line is perfectly valid.
                    //
                    // Likewise, a block-entry \`-\` immediately after a
                    // property (Anchor / Tag) on the same line is
                    // invalid — the property must precede a node, and
                    // a block sequence's first \`-\` must begin a line
                    // (yaml-test-suite SY6V \`&anchor - x\`).
                    if let Some(last) = self.tokens.last() {
                        if matches!(
                            last.token_type,
                            TokenType::FlowMappingEnd | TokenType::FlowSequenceEnd
                        ) && last.end_position.line == self.position.line
                        {
                            return Err(Error::scan(
                                self.position,
                                "Block-entry `-` immediately after flow collection close"
                                    .to_string(),
                            ));
                        }
                        if matches!(last.token_type, TokenType::Anchor(_) | TokenType::Tag(_))
                            && last.end_position.line == self.position.line
                        {
                            return Err(Error::scan(
                                self.position,
                                "Block-entry `-` cannot follow a property on the same line"
                                    .to_string(),
                            ));
                        }
                        // §8.22: a block sequence's first \`-\` must
                        // begin on a new line. \`key: - a\` (implicit
                        // key, then dash on same line) is invalid
                        // (yaml-test-suite 5U3A). But \`? key\\n: - x\`
                        // (explicit value-separator on the same line
                        // as the dash) IS valid: the \`?\` key sits
                        // on a previous line. We distinguish by
                        // walking back from the Value: if the
                        // preceding non-property token is a Scalar
                        // on the same line as the Value, the key
                        // is implicit; otherwise it's after \`?\`.
                        if matches!(last.token_type, TokenType::Value)
                            && last.end_position.line == self.position.line
                        {
                            let value_line = last.start_position.line;
                            let mut prior_scalar_line = None;
                            for t in self.tokens.iter().rev().skip(1) {
                                match &t.token_type {
                                    TokenType::Anchor(_) | TokenType::Tag(_) => {}
                                    TokenType::Scalar(..) => {
                                        prior_scalar_line = Some(t.end_position.line);
                                        break;
                                    }
                                    _ => break,
                                }
                            }
                            if prior_scalar_line == Some(value_line) {
                                return Err(Error::scan(
                                    self.position,
                                    "Block sequence value cannot start on the same line as its key"
                                        .to_string(),
                                ));
                            }
                        }
                    }
                    let pos = self.position;
                    self.advance();

                    // Check if we need to start a new block sequence.
                    // `unwrap_or(0)` mirrors the pattern in
                    // src/scanner/indentation.rs and is safer than
                    // `.unwrap()` here: an error-recovery pop in another
                    // path could otherwise leave the stack empty and
                    // panic on crafted input (#18).
                    let last_indent = self.indent_stack.last().copied().unwrap_or(0);

                    // If a compact sequence (opened from `? - x` or
                    // similar) is already active at this dash's column,
                    // the dash continues it — don't open a new nested
                    // block sequence (yaml-test-suite M5DY).
                    let dash_indent = pos.column.saturating_sub(1);
                    let compact_active_here = self
                        .compact_sequence_indents
                        .last()
                        .map_or(false, |&si| si == dash_indent);
                    if compact_active_here {
                        // Continuation of an existing compact sequence.
                    } else if self.current_indent > last_indent {
                        // Deeper indentation - start new nested sequence
                        self.indent_stack.push(self.current_indent);
                        self.indent_is_sequence.push(true);
                        // Check depth limit
                        self.resource_tracker
                            .check_depth(&self.limits, self.flow_level + self.indent_stack.len())?;
                        self.tokens
                            .push(Token::simple(TokenType::BlockSequenceStart, pos));
                    } else if self.current_indent == last_indent
                        && *self.indent_is_sequence.last().unwrap_or(&false)
                    {
                        // Same indent and the top of stack is already a sequence
                        // → continuation of that sequence; no new start needed.
                    } else if self.current_indent >= last_indent {
                        // Same or root level — compact notation.
                        // Start a new sequence only if we don't already have one
                        // tracked at this exact indent.
                        // For a dash that's *not* at line-start (e.g.
                        // `? - x` where current_indent is still the
                        // line's indent but the dash sits in mid-line),
                        // use the dash column - 1 as the sequence's
                        // indent so scan_plain_scalar's continuation
                        // check correctly sees the deeper context
                        // (yaml-test-suite M5DY).
                        let dash_indent = pos.column.saturating_sub(1);
                        let seq_indent = dash_indent.max(self.current_indent);
                        let has_active_compact = self
                            .compact_sequence_indents
                            .last()
                            .map_or(false, |&si| si == seq_indent);

                        if !has_active_compact {
                            self.compact_sequence_indents.push(seq_indent);
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

                    // After emitting BlockEntry, check if the next
                    // token is another dash (nested sequence). §6.2
                    // requires SPACE separation between dashes — a
                    // tab between the outer and inner \`-\` is invalid
                    // (yaml-test-suite Y79Y/004, /005). Track whether
                    // a tab was consumed while skipping the inter-
                    // dash whitespace and reject if so.
                    let mut saw_tab_between = false;
                    while let Some(c) = self.current_char {
                        if c == ' ' {
                            self.advance();
                        } else if c == '\t' {
                            saw_tab_between = true;
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if self.current_char == Some('-')
                        && self.peek_char(1).map_or(true, |c| c.is_whitespace())
                        && saw_tab_between
                    {
                        return Err(Error::scan(
                            self.position,
                            "Tab between block-entries on same line".to_string(),
                        ));
                    }
                    if self.current_char == Some('-')
                        && self.peek_char(1).map_or(true, |c| c.is_whitespace())
                    {
                        // We have a nested sequence on the same line!
                        // Track this as an inline sequence
                        self.inline_sequence_depth += 1;
                        // Push the *indent* (column - 1), not the
                        // column, so it matches the convention used by
                        // maybe_open_block_mapping_for_key. With column
                        // here the next-line indent (column - 1) would
                        // be strictly less than the stored value and
                        // wrongly trigger an early close, breaking
                        // multi-line nested sequences (yaml-test-suite
                        // 3ALJ, 57H4).
                        self.indent_stack
                            .push(self.position.column.saturating_sub(1));
                        self.indent_is_sequence.push(true);
                        // Check depth limit
                        self.resource_tracker
                            .check_depth(&self.limits, self.flow_level + self.indent_stack.len())?;
                        self.tokens
                            .push(Token::simple(TokenType::BlockSequenceStart, self.position));
                        // Continue processing - the next iteration will handle the nested dash
                    } else if self.current_char.is_some()
                        && !matches!(self.current_char, Some('\n' | '\r'))
                    {
                        // Content follows "- " on the same line.
                        // Update current_indent to the content's column position so that
                        // any mapping started here will be at a deeper indent level than
                        // the sequence. This ensures handle_indentation properly closes
                        // the mapping when the next sibling "- " appears.
                        self.current_indent = self.position.column - 1;
                    }
                }

                // Quoted strings — same implicit-key mapping detection
                // as for plain scalars (yaml-test-suite 6H3V, 6SLA).
                '"' | '\'' => {
                    if self.flow_level == 0 && self.check_for_mapping_ahead() {
                        self.maybe_open_block_mapping_for_key()?;
                    }
                    let token = self.scan_quoted_string(ch)?;
                    self.tokens.push(token);
                }

                // Document markers (only if not a block entry).
                //
                // Reached only when `-` is at column = current_indent + 1 AND
                // the next character is non-whitespace — i.e. either the
                // `---` document-start marker OR a plain scalar starting
                // with `-` (e.g. `---word1`, `-foo`). If `scan_document_start`
                // declines, we MUST consume the run as a plain scalar — not
                // consulting `is_plain_scalar_start` here, because that helper
                // unconditionally rejects `-`, which would leave the outer
                // `while let` loop spinning on the same character.
                '-' if self.position.column == self.current_indent + 1
                    && !self.peek_char(1).map_or(true, |c| c.is_whitespace()) =>
                {
                    if let Some(token) = self.scan_document_start()? {
                        self.tokens.push(token);
                    } else {
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
                // Only scan as number if the entire token is numeric (no trailing letters)
                _ if (ch.is_ascii_digit()
                    || (ch == '-' && self.peek_char(1).map_or(false, |c| c.is_ascii_digit())))
                    && self.is_pure_number() =>
                {
                    let token = self.scan_number()?;
                    self.tokens.push(token);
                }

                // Anchors and aliases. §6.9: a node's properties
                // (anchor/tag) are prefixes of the node. When an `&`,
                // `*`, or `!` is at the start of a line (column ==
                // current_indent + 1) and a `: ` follows on the same
                // line, the property/alias is part of an implicit
                // key's leading position. The block mapping that
                // contains this key therefore opens at this column,
                // *before* the property/alias token is emitted
                // (yaml-test-suite 7BMT, 6BFJ, 9KAX, U3XV, 26DV).
                '&' => {
                    // Mirror H7J7 check for anchors (yaml-test-suite
                    // G9HC \`seq:\\n&anchor\\n- a\`).
                    if self.flow_level == 0
                        && self.position.column == self.current_indent + 1
                        && !self.check_for_mapping_ahead()
                        && self.indent_stack.len() > 1
                        && self.current_indent == self.indent_stack[self.indent_stack.len() - 2]
                        && self.most_recent_token_is_value_separator()
                    {
                        return Err(Error::scan(
                            self.position,
                            "Anchor at line-start with insufficient indent for value position"
                                .to_string(),
                        ));
                    }
                    if self.flow_level == 0
                        && self.position.column == self.current_indent + 1
                        && self.check_for_mapping_ahead()
                    {
                        self.maybe_open_block_mapping_for_key()?;
                    }
                    let token = self.scan_anchor()?;
                    self.tokens.push(token);
                }
                '*' => {
                    // §6.9.2: alias/anchor names may contain \`:\` (only
                    // flow indicators and whitespace terminate them).
                    // So \`*a:\` is an alias named \`a:\`, NOT an alias
                    // \`*a\` followed by a key separator. Don't open
                    // an implicit block mapping in that case (yaml-
                    // test-suite 2SXE).
                    if self.flow_level == 0
                        && self.position.column == self.current_indent + 1
                        && self.check_for_mapping_ahead()
                        && !self.colon_belongs_to_alias_anchor_name()
                    {
                        self.maybe_open_block_mapping_for_key()?;
                    }
                    let token = self.scan_alias()?;
                    self.tokens.push(token);
                }

                // Block scalars
                '|' => {
                    let token = self.scan_literal_block_scalar()?;
                    self.tokens.push(token);
                    // Block scalar collection rewinds the cursor to the
                    // start of the next under-indented line. `current_indent`
                    // is still set to the inline content's column from the
                    // enclosing `- |` / `key: |` site, so the next iteration
                    // would mis-dispatch. Break out so the outer loop
                    // re-enters `process_line` and reruns indent handling
                    // (yaml-test-suite 4QFQ, M6YH, P2AD).
                    break;
                }
                '>' => {
                    let token = self.scan_folded_block_scalar()?;
                    self.tokens.push(token);
                    break;
                }

                // Tags. Same line-start property-opens-mapping rule
                // (yaml-test-suite ZH7C variants).
                //
                // §6.9: a property at the SAME indent as the
                // enclosing mapping/sequence cannot apply to that
                // collection's value — the value must be more
                // indented. If we're at a line-start \`!\` whose column
                // equals the enclosing mapping's indent + 1 AND that
                // mapping currently has a key awaiting a value, the
                // tag is misplaced (yaml-test-suite H7J7).
                '!' => {
                    if self.flow_level == 0
                        && self.position.column == self.current_indent + 1
                        && !self.check_for_mapping_ahead()
                        && self.indent_stack.len() > 1
                        && self.current_indent == self.indent_stack[self.indent_stack.len() - 2]
                        && self.most_recent_token_is_value_separator()
                    {
                        return Err(Error::scan(
                            self.position,
                            "Tag at line-start with insufficient indent for value position"
                                .to_string(),
                        ));
                    }
                    if self.flow_level == 0
                        && self.position.column == self.current_indent + 1
                        && self.check_for_mapping_ahead()
                    {
                        self.maybe_open_block_mapping_for_key()?;
                    }
                    let token = self.scan_tag()?;
                    self.tokens.push(token);
                }

                // Plain scalars
                _ if self.is_plain_scalar_start() => {
                    // A plain scalar starting on the SAME line as a
                    // flow-collection close (\`}\` or \`]\`) means there's
                    // no separator between the closed flow node and
                    // the new content (yaml-test-suite 62EZ
                    // \`x: { y: z }in: valid\`).
                    if self.flow_level == 0 {
                        if let Some(last) = self.tokens.last() {
                            if matches!(
                                last.token_type,
                                TokenType::FlowMappingEnd | TokenType::FlowSequenceEnd
                            ) && last.end_position.line == self.position.line
                            {
                                return Err(Error::scan(
                                    self.position,
                                    "Plain scalar immediately after flow collection close"
                                        .to_string(),
                                ));
                            }
                        }
                    }
                    if self.flow_level == 0 && self.check_for_mapping_ahead() {
                        self.maybe_open_block_mapping_for_key()?;
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

        // Inline sequences (nested \`- -\` on one line) used to be
        // closed unconditionally at end-of-line. But a nested sequence
        // can span lines (`- - a\n  - b\n- c`) — in that case the inner
        // sequence must remain open until handle_indentation sees a
        // dedent. Reset the inline-sequence counter (so the next line
        // is judged on its own merits) but DO NOT emit BlockEnd —
        // handle_indentation's indent_stack pop, the end-of-stream
        // close at scan_next_token, and the explicit-dedent close at
        // handle_indentation's bottom each provide a correct close.
        self.inline_sequence_depth = 0;

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

        // Close any remaining compact sequences (before their parent mappings)
        while self.compact_sequence_indents.pop().is_some() {
            self.tokens
                .push(Token::simple(TokenType::BlockEnd, self.position));
        }

        // Close any remaining blocks
        while self.indent_stack.len() > 1 {
            self.indent_stack.pop();
            self.indent_is_sequence.pop();
            self.tokens
                .push(Token::simple(TokenType::BlockEnd, self.position));
        }

        self.tokens
            .push(Token::simple(TokenType::StreamEnd, self.position));
        self.done = true;
        Ok(())
    }

    /// Peek at a character at the given offset (can be negative)
    /// Check if the current position starts a pure number (digits/dots/minus only,
    /// not followed by letters). Values like 500m, 128Mi should be treated as plain scalars.
    fn is_pure_number(&self) -> bool {
        let mut offset: isize = 0;
        let first = self.peek_char(0);
        // Skip leading minus
        if first == Some('-') {
            offset = 1;
        }
        // Scan digits and at most one dot
        let mut has_digit = false;
        let mut dot_count = 0;
        loop {
            match self.peek_char(offset) {
                Some(c) if c.is_ascii_digit() => {
                    has_digit = true;
                    offset += 1;
                }
                Some('.') => {
                    dot_count += 1;
                    if dot_count > 1 {
                        // Multiple dots (e.g. 0.5.8) — not a number
                        return false;
                    }
                    offset += 1;
                }
                Some(c) if c.is_ascii_alphabetic() || c == '_' => {
                    // Letters follow the digits — not a pure number (e.g. 500m, 128Mi)
                    return false;
                }
                Some(c) => {
                    // For a token to be a pure number, what follows
                    // the digits must be end-of-token. In flow
                    // context that's a flow indicator. In block
                    // context the rest of the line must be pure
                    // whitespace (possibly trailing a comment) — if
                    // there's more non-whitespace content on this
                    // line, the digits are part of a larger plain
                    // scalar like \`1 - 3\` (yaml-test-suite P76L)
                    // or \`20:03:20\` (yaml-test-suite U9NS).
                    if self.flow_level > 0 && ",[]{}".contains(c) {
                        return has_digit;
                    }
                    if c == '\n' || c == '\r' {
                        return has_digit;
                    }
                    if c == ' ' || c == '\t' {
                        // Look ahead: rest of line must be whitespace
                        // or a comment.
                        let mut probe = offset + 1;
                        loop {
                            match self.peek_char(probe) {
                                None => return has_digit,
                                Some('\n' | '\r') => return has_digit,
                                Some('#') => return has_digit,
                                Some(' ' | '\t') => probe += 1,
                                Some(_) => return false,
                            }
                        }
                    }
                    if c == ':' {
                        let next = self.peek_char(offset + 1);
                        return has_digit && next.map_or(true, |nc| nc.is_whitespace());
                    }
                    return false;
                }
                None => return has_digit,
            }
        }
    }

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
        // Per YAML 1.2 §6.9.2 (ns-anchor-name = ns-anchor-char+), the only
        // exclusions are whitespace and the flow indicators `,[]{}`. This
        // accepts ASCII alphanumeric, underscore, hyphen, AND full unicode
        // codepoints (including emoji), matching the spec exactly.
        let mut identifier = String::new();
        while let Some(ch) = self.current_char {
            if ch.is_whitespace() || matches!(ch, ',' | '[' | ']' | '{' | '}') {
                break;
            }
            identifier.push(ch);
            self.advance();
        }
        Ok(identifier)
    }

    /// Scan a tag token (`!tag`, `!!tag`, or `!<verbatim>`).
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

            // Scan tag name/suffix.
            //
            // Per YAML 1.2 §5.6, tag suffixes are URI references — they may
            // contain any URI character (RFC 3986 unreserved + sub-delims +
            // a few others) or `%XX` percent-encoded bytes. The handful of
            // characters listed below covers the alphanumeric + URI-safe
            // punctuation set used by yaml-test-suite. Percent decoding of
            // `%XX` happens later in `TagResolver::resolve`.
            //
            // §5.3: inside a flow collection, the flow indicators
            // `,`, `[`, `]`, `{`, `}` always terminate a node — so we
            // must NOT consume them into the tag suffix even though
            // RFC 3986 permits them in URIs (yaml-test-suite WZ62).
            // YAML 1.2 in practice treats `,` as a flow indicator that
            // must be percent-encoded (\`%2C\`) when it appears inside
            // a tag suffix — bare \`,\` is not allowed in EITHER block
            // or flow context (yaml-test-suite U99R).
            while let Some(ch) = self.current_char {
                if matches!(ch, ',') {
                    break;
                }
                if self.flow_level > 0 && matches!(ch, '[' | ']' | '{' | '}') {
                    break;
                }
                // §6.8 / §5.6: `:` IS a valid tag URI character — e.g.
                // `tag:yaml.org,2002:str` legitimately contains two
                // colons inside its URI. But a `:` followed by
                // whitespace, EOL or EOF is the YAML mapping-value
                // indicator and MUST terminate the tag, otherwise
                // `!handle!suffix: value` is mis-scanned as
                // `Tag("!handle!suffix:") Scalar("value")` and the
                // implicit-key mapping structure is lost. Mirrors the
                // `,` carve-out above (a valid URI char that's also a
                // YAML flow indicator in some contexts).
                if ch == ':' {
                    match self.peek_char(1) {
                        None => break,
                        Some(c) if c.is_whitespace() => break,
                        _ => {}
                    }
                }
                if ch.is_alphanumeric() || "-._~:/?#[]@!$&'()*+;=%".contains(ch) {
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
        let (chomping, explicit_indent) = self.scan_block_scalar_header()?;

        // Skip to next line
        self.skip_to_next_line()?;

        // Determine indentation. `base_indent` is the surrounding
        // block's indent — i.e. the indent of the sequence or
        // mapping that contains this scalar. `self.current_indent`
        // is sometimes set to the inline indicator column (e.g. 2
        // for `- |`), which would make `base_indent + explicit`
        // wrong; use the top of `indent_stack` instead
        // (yaml-test-suite 4QFQ `|1`).
        let base_indent = self.indent_stack.last().copied().unwrap_or(0);
        let content_indent = if let Some(explicit) = explicit_indent {
            base_indent + explicit
        } else {
            // Find the first non-empty content line to determine indentation
            self.find_block_scalar_indent(base_indent)?
        };

        // Collect the literal block content
        let content = self.collect_literal_block_content(content_indent, chomping)?;

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
        let (chomping, explicit_indent) = self.scan_block_scalar_header()?;

        // Skip to next line
        self.skip_to_next_line()?;

        // See scan_literal_block_scalar for why we read `indent_stack`
        // rather than `current_indent`.
        let base_indent = self.indent_stack.last().copied().unwrap_or(0);
        let content_indent = if let Some(explicit) = explicit_indent {
            base_indent + explicit
        } else {
            // Find the first non-empty content line to determine indentation
            self.find_block_scalar_indent(base_indent)?
        };

        // Collect the folded block content
        let content = self.collect_folded_block_content(content_indent, chomping)?;

        Ok(Token::new(
            TokenType::BlockScalarFolded(content),
            start_pos,
            self.position,
        ))
    }

    /// Parse block scalar header indicators (+, -, and explicit indent)
    fn scan_block_scalar_header(&mut self) -> Result<(ChompingMode, Option<usize>)> {
        let mut chomping = ChompingMode::Clip;
        let mut explicit_indent: Option<usize> = None;
        // §6.6: a comment must be preceded by whitespace. \`|#x\` and
        // \`>#x\` are invalid (yaml-test-suite X4QW).
        let mut seen_separator_ws = false;

        // Parse indicators in any order
        while let Some(ch) = self.current_char {
            match ch {
                '+' => {
                    chomping = ChompingMode::Keep;
                    self.advance();
                }
                '-' => {
                    chomping = ChompingMode::Strip;
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
                    // YAML 1.2 §8.1.1.1: explicit indent indicator is
                    // 1..=9. `|0` and `>0` are invalid
                    // (yaml-test-suite 2G84/00).
                    if digit == 0 {
                        let context = ErrorContext::from_input(&self.input, &self.position, 2)
                            .with_suggestion(
                                "Block-scalar indent indicator must be 1-9".to_string(),
                            );
                        return Err(Error::scan_with_context(
                            self.position,
                            "Block-scalar indent indicator `0` is invalid",
                            context,
                        ));
                    }
                    explicit_indent = Some(digit);
                    self.advance();
                }
                ' ' | '\t' => {
                    seen_separator_ws = true;
                    self.advance(); // Skip whitespace
                }
                '#' => {
                    if !seen_separator_ws {
                        return Err(Error::scan(
                            self.position,
                            "Comment in block-scalar header must be preceded by whitespace"
                                .to_string(),
                        ));
                    }
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

        Ok((chomping, explicit_indent))
    }

    /// Advance the cursor PAST the next line break, but do not consume
    /// any leading whitespace on the line that follows. The block-
    /// scalar header parser uses this to step from the indicator line
    /// to the start of the content line — the next line's leading
    /// spaces are part of its content_indent, not header whitespace.
    fn skip_to_next_line(&mut self) -> Result<()> {
        // If we're already at column 1 (the comment handler in
        // scan_block_scalar_header may have already advanced past a
        // newline), do nothing — the next line's leading whitespace
        // belongs to its content_indent.
        if self.position.column == 1 {
            return Ok(());
        }
        while let Some(ch) = self.current_char {
            match ch {
                '\n' | '\r' => {
                    self.advance();
                    return Ok(());
                }
                ' ' | '\t' => {
                    self.advance();
                }
                _ => return Ok(()),
            }
        }
        Ok(())
    }

    /// Find the content indentation for a block scalar.
    ///
    /// Per spec §8.1.1.1, indent is the leading-space count of the first
    /// non-empty content line (or the longest blank-line indent if no
    /// non-empty line exists). A non-empty line whose indent is not
    /// strictly deeper than `base_indent` is outside the scalar's
    /// scope — that line is a sibling structure, not content
    /// (yaml-test-suite K858).
    fn find_block_scalar_indent(&mut self, base_indent: usize) -> Result<usize> {
        let saved_position = self.position;
        let saved_char = self.current_char;
        let saved_char_index = self.current_char_index;

        let mut max_blank_indent: usize = 0;
        let mut found = false;
        let mut content_indent: usize = 1;

        loop {
            let mut line_indent = 0;
            while self.current_char == Some(' ') {
                line_indent += 1;
                self.advance();
            }
            // §6.1 + §8.1: tabs cannot serve as block-scalar
            // indentation. A line that BEGINS with a tab (no leading
            // spaces) inside the block scalar's indent search is
            // invalid (yaml-test-suite Y79Y/000 \`foo: |\\n\\tbar\`).
            // Tabs that appear AFTER one or more spaces are content,
            // not indentation, and remain valid (yaml-test-suite
            // 96NN/00 \`foo: |-\\n \\tbar\`).
            if line_indent == 0 && self.current_char == Some('\t') {
                return Err(Error::scan(
                    self.position,
                    "Tab cannot serve as block-scalar indentation".to_string(),
                ));
            }

            match self.current_char {
                None => {
                    if line_indent > max_blank_indent {
                        max_blank_indent = line_indent;
                    }
                    break;
                }
                Some('\n' | '\r') => {
                    if line_indent > max_blank_indent {
                        max_blank_indent = line_indent;
                    }
                    self.advance();
                    // fall through to next iteration
                }
                Some(_) => {
                    // If we're nested inside another block — either
                    // via the `indent_stack` (normal mapping/sequence
                    // open) or `compact_sequence_indents` (a
                    // compact block sequence at the same indent as
                    // its parent) — and this candidate line is not
                    // strictly deeper than base_indent, it's a
                    // sibling outside the scalar's scope (yaml-test-
                    // suite K858, P2AD).
                    let inside_block =
                        self.indent_stack.len() > 1 || !self.compact_sequence_indents.is_empty();
                    if inside_block && line_indent <= base_indent {
                        content_indent = max_blank_indent.max(base_indent + 1);
                    } else {
                        content_indent = line_indent;
                    }
                    // §8.1.2.1: leading blank lines may not exceed the
                    // detected content indent — that ambiguity is
                    // invalid (yaml-test-suite W9L4, S98Z).
                    if max_blank_indent > content_indent {
                        self.position = saved_position;
                        self.current_char = saved_char;
                        self.current_char_index = saved_char_index;
                        return Err(Error::scan(
                            self.position,
                            "Block scalar leading blank-line indent exceeds content indent"
                                .to_string(),
                        ));
                    }
                    found = true;
                    break;
                }
            }
        }

        if !found {
            content_indent = max_blank_indent;
        }

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

    /// Collect content for a literal block scalar.
    ///
    /// Each line is preserved with its terminating newline. After collection
    /// we apply the chomping mode per spec §8.1.1.2.
    fn collect_literal_block_content(
        &mut self,
        content_indent: usize,
        chomping: ChompingMode,
    ) -> Result<String> {
        let mut content = String::new();

        loop {
            // Count current line's leading-space indent.
            let mut line_indent = 0;
            let save_pos = self.position;
            let save_ch = self.current_char;
            let save_idx = self.current_char_index;
            while self.current_char == Some(' ') {
                line_indent += 1;
                self.advance();
            }

            let line_is_blank = matches!(self.current_char, Some('\n' | '\r') | None);

            if !line_is_blank && line_indent < content_indent {
                // Non-empty line with less indent ends the scalar; rewind.
                self.position = save_pos;
                self.current_char = save_ch;
                self.current_char_index = save_idx;
                break;
            }

            // Document marker at line start always ends the scalar,
            // regardless of content_indent (allows zero-indented
            // block scalars per yaml-test-suite FP8R).
            if line_indent == 0 && self.is_doc_marker_here() {
                self.position = save_pos;
                self.current_char = save_ch;
                self.current_char_index = save_idx;
                break;
            }

            if line_is_blank {
                // A blank line counts when there's an actual line break
                // to consume. EOF after we've consumed some whitespace
                // on the trailing line ALSO counts as one final blank
                // line (yaml-test-suite JEF9/02: `- |+\n        `).
                if matches!(self.current_char, Some('\n' | '\r')) {
                    // Whitespace beyond content_indent is literal content
                    // even on blank lines (yaml-test-suite 6FWR).
                    for _ in content_indent..line_indent {
                        content.push(' ');
                    }
                    content.push('\n');
                    self.advance();
                    continue;
                }
                if line_indent > 0 {
                    for _ in content_indent..line_indent {
                        content.push(' ');
                    }
                    content.push('\n');
                }
                break;
            }

            // Content line: we already consumed `line_indent` spaces, but
            // only `content_indent` of them belong to indentation. Any
            // extra leading spaces are literal content.
            let mut line = String::new();
            for _ in content_indent..line_indent {
                line.push(' ');
            }
            while let Some(ch) = self.current_char {
                if ch == '\n' || ch == '\r' {
                    self.advance();
                    break;
                }
                line.push(ch);
                self.advance();
            }
            content.push_str(&line);
            content.push('\n');

            if self.current_char.is_none() {
                break;
            }
        }

        Ok(apply_chomping(content, chomping))
    }

    /// Check if cursor is at `---` or `...` followed by whitespace/EOL.
    fn is_doc_marker_here(&self) -> bool {
        let c0 = self.current_char;
        let c1 = self.peek_char(1);
        let c2 = self.peek_char(2);
        let c3 = self.peek_char(3);
        let trailing_ok = c3.map_or(true, |c| c.is_whitespace());
        (c0 == Some('-') && c1 == Some('-') && c2 == Some('-') && trailing_ok)
            || (c0 == Some('.') && c1 == Some('.') && c2 == Some('.') && trailing_ok)
    }

    /// Collect content for a folded block scalar.
    ///
    /// Folding rules (§8.1.3): a sequence of single blank lines between
    /// equally-indented non-empty content lines collapses into a single
    /// space; runs of blank lines emit `n-1` newlines; more-indented
    /// lines preserve their newline boundaries. After collection, apply
    /// chomping (§8.1.1.2).
    fn collect_folded_block_content(
        &mut self,
        content_indent: usize,
        chomping: ChompingMode,
    ) -> Result<String> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum LineKind {
            Normal,
            MoreIndented,
            Empty,
        }
        struct Line {
            text: String,
            kind: LineKind,
        }

        let mut lines: Vec<Line> = Vec::new();

        loop {
            let mut line_indent = 0;
            let save_pos = self.position;
            let save_ch = self.current_char;
            let save_idx = self.current_char_index;
            while self.current_char == Some(' ') {
                line_indent += 1;
                self.advance();
            }

            let line_is_blank = matches!(self.current_char, Some('\n' | '\r') | None);

            if !line_is_blank && line_indent < content_indent {
                self.position = save_pos;
                self.current_char = save_ch;
                self.current_char_index = save_idx;
                break;
            }

            if line_indent == 0 && self.is_doc_marker_here() {
                self.position = save_pos;
                self.current_char = save_ch;
                self.current_char_index = save_idx;
                break;
            }

            if line_is_blank {
                if matches!(self.current_char, Some('\n' | '\r')) {
                    lines.push(Line {
                        text: String::new(),
                        kind: LineKind::Empty,
                    });
                    self.advance();
                    continue;
                }
                break;
            }

            // Capture extra-indent leading spaces as part of content.
            let mut text = String::new();
            for _ in content_indent..line_indent {
                text.push(' ');
            }
            while let Some(ch) = self.current_char {
                if ch == '\n' || ch == '\r' {
                    self.advance();
                    break;
                }
                text.push(ch);
                self.advance();
            }
            // §8.1.3.2: "more indented" means the content (after the
            // common indent strip) begins with extra whitespace —
            // either spaces or tabs (yaml-test-suite MJS9).
            let kind = if text.starts_with(' ') || text.starts_with('\t') {
                LineKind::MoreIndented
            } else {
                LineKind::Normal
            };
            lines.push(Line { text, kind });

            if self.current_char.is_none() {
                break;
            }
        }

        // Build the folded output.
        let mut content = String::new();
        let mut idx = 0;
        while idx < lines.len() {
            let line = &lines[idx];
            match line.kind {
                LineKind::Normal | LineKind::MoreIndented => {
                    content.push_str(&line.text);
                    // Lookahead: count immediately-following empty lines.
                    let mut j = idx + 1;
                    let mut empties = 0;
                    while j < lines.len() && lines[j].kind == LineKind::Empty {
                        empties += 1;
                        j += 1;
                    }
                    if j < lines.len() {
                        // Spec §8.1.3.2: folding behaviour depends on
                        // whether either surrounding content line is
                        // "more indented" than the content indent.
                        // - both Normal, 0 empties → fold to space.
                        // - both Normal, k empties → k newlines (one
                        //   break folded out).
                        // - any MoreIndented, 0 empties → 1 newline.
                        // - any MoreIndented, k empties → k+1 newlines
                        //   (every break preserved).
                        let mi_adjacent = line.kind == LineKind::MoreIndented
                            || lines[j].kind == LineKind::MoreIndented;
                        if empties == 0 {
                            if mi_adjacent {
                                content.push('\n');
                            } else {
                                content.push(' ');
                            }
                        } else {
                            let breaks = if mi_adjacent { empties + 1 } else { empties };
                            for _ in 0..breaks {
                                content.push('\n');
                            }
                        }
                        idx = j;
                    } else {
                        // End of stream after content (possibly trailing empties).
                        // Always emit final `\n` for the last content line; extra
                        // trailing empties contribute additional `\n`s, and chomping
                        // will trim them later if needed.
                        content.push('\n');
                        for _ in 0..empties {
                            content.push('\n');
                        }
                        break;
                    }
                }
                LineKind::Empty => {
                    // Leading empty lines (no preceding content): emit as `\n`s.
                    content.push('\n');
                    idx += 1;
                }
            }
        }

        Ok(apply_chomping(content, chomping))
    }

    /// Emit a `BlockMappingStart` token if the current position is the
    /// start of an implicit key and no mapping is yet active at this
    /// indent level. Shared by plain and quoted scalar dispatch.
    fn maybe_open_block_mapping_for_key(&mut self) -> Result<()> {
        // Use `unwrap_or(0)` for parity with the indentation module's
        // helpers — defends against error-recovery pop paths that could
        // leave the stack momentarily empty (#18).
        let last_indent = self.indent_stack.last().copied().unwrap_or(0);
        let should_start_new_mapping = if self.current_indent > last_indent {
            true
        } else if self.current_indent == last_indent {
            !self.check_active_mapping_at_level(self.current_indent)
        } else {
            false
        };
        if should_start_new_mapping {
            // §6.1 + §8.22: opening a NEW block mapping at deeper
            // indent than the parent only makes sense if the parent
            // has a key WITHOUT a value (the new mapping IS that
            // value). If the parent's last content is a complete
            // (key, value) pair — i.e. the most recent meaningful
            // token is a value-position scalar/alias/close — then
            // there's no node to host the deeper mapping (yaml-test-
            // suite U44R: \`map:\\n  key1: q\\n   key2: bad\` — key2
            // is deeper than key1 but key1's value is already \`q\`).
            if self.current_indent > last_indent && last_indent > 0 {
                let mut depth = 0i32;
                let mut last_meaningful = None;
                for t in self.tokens.iter().rev() {
                    match &t.token_type {
                        TokenType::BlockEnd => depth += 1,
                        TokenType::BlockMappingStart | TokenType::BlockSequenceStart => {
                            if depth == 0 {
                                break;
                            }
                            depth -= 1;
                        }
                        TokenType::Anchor(_) | TokenType::Tag(_) => {}
                        other => {
                            if depth == 0 {
                                last_meaningful = Some(other.clone());
                                break;
                            }
                        }
                    }
                }
                if matches!(
                    last_meaningful,
                    Some(
                        TokenType::Scalar(..)
                            | TokenType::Alias(_)
                            | TokenType::FlowSequenceEnd
                            | TokenType::FlowMappingEnd
                            | TokenType::BlockScalarLiteral(..)
                            | TokenType::BlockScalarFolded(..)
                    )
                ) {
                    return Err(Error::scan(
                        self.position,
                        "Indentation increase has no parent in current mapping/sequence"
                            .to_string(),
                    ));
                }
            }
            self.indent_stack.push(self.current_indent);
            self.indent_is_sequence.push(false);
            self.resource_tracker
                .check_depth(&self.limits, self.flow_level + self.indent_stack.len())?;
            self.tokens
                .push(Token::simple(TokenType::BlockMappingStart, self.position));
        }
        Ok(())
    }

    /// Look ahead on the current line for a `:` that marks a mapping key.
    ///
    /// Per YAML 1.2 §7.3.3, a plain scalar may contain a `:` that is not
    /// followed by whitespace. Only `: ` terminates the scalar. If the
    /// line begins with `"` or `'`, the leading quoted scalar's contents
    /// are scanned past (including `''` and `\"` escapes) before looking
    /// for the `: ` that would make this scalar a key. This handles
    /// yaml-test-suite 6H3V (`'foo: bar\': baz'`) and 6SLA.
    /// For an alias/anchor at the current position, scan past
    /// the `&`/`*` and the name characters; if the FIRST char that
    /// would terminate the name is `:`, the colon is PART of the
    /// alias/anchor name (yaml-test-suite 2SXE). Returns true in
    /// that case so the caller can skip the implicit-key fast-path.
    fn colon_belongs_to_alias_anchor_name(&self) -> bool {
        // Start after the `&` / `*` introducer.
        let mut i = self.current_char_index + 1;
        let n = self.char_cache.len();
        // Per scan_identifier rules: stop at whitespace or flow indicator.
        while i < n {
            let c = self.char_cache[i];
            if c.is_whitespace() || matches!(c, ',' | '[' | ']' | '{' | '}') {
                break;
            }
            i += 1;
        }
        // If the next char (or last consumed?) at termination is `:`,
        // then the name ended with `:`. Look at the LAST consumed
        // char. Actually our scan_identifier accepts `:` as part of
        // name — so the colon is already in the name. There's no
        // separate "value indicator" colon after.
        //
        // For the implicit-key fast path to be wrong, we need the
        // name to END with `:` (last char of name is `:`).
        if i > self.current_char_index + 1 {
            let last_name_char = self.char_cache[i - 1];
            if last_name_char == ':' {
                return true;
            }
        }
        false
    }

    /// Scan ahead on the current line (the rest of the post-indent
    /// content) to determine whether it looks like an implicit
    /// mapping key — i.e. has a `: ` separator (or `:` at line end)
    /// before any newline.
    fn line_after_indent_is_implicit_key(&self) -> bool {
        let mut i = self.current_char_index;
        let n = self.char_cache.len();
        while i < n {
            let ch = self.char_cache[i];
            if ch == '\n' || ch == '\r' {
                return false;
            }
            if ch == ':' {
                let next = self.char_cache.get(i + 1).copied();
                if next.is_none() || next.map_or(false, |c| c.is_whitespace()) {
                    return true;
                }
            }
            i += 1;
        }
        false
    }

    /// Walk back through recent tokens; if the last non-property
    /// token was `Value` (`:`), the parser is in value-expectation
    /// mode (key not yet matched with a value).
    fn most_recent_token_is_value_separator(&self) -> bool {
        for t in self.tokens.iter().rev() {
            match t.token_type {
                TokenType::Anchor(_) | TokenType::Tag(_) => {}
                TokenType::Value => return true,
                _ => return false,
            }
        }
        false
    }

    fn check_for_mapping_ahead(&self) -> bool {
        let mut i = self.current_char_index;
        let n = self.char_cache.len();
        if i < n {
            let first = self.char_cache[i];
            if first == '\'' || first == '"' {
                let quote = first;
                i += 1;
                while i < n {
                    let c = self.char_cache[i];
                    if c == '\n' || c == '\r' {
                        return false; // unterminated quote on line
                    }
                    if quote == '\'' && c == '\'' && self.char_cache.get(i + 1) == Some(&'\'') {
                        // `''` is the in-string single-quote escape.
                        i += 2;
                        continue;
                    }
                    if quote == '"' && c == '\\' {
                        // Skip the escaped char.
                        i += 2;
                        continue;
                    }
                    if c == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
        }
        // Skip balanced flow collections — a `:` *inside* `[...]` or
        // `{...}` does NOT make the line a block-mapping key (the flow
        // collection itself can BE the key, but its inner colons are
        // part of its own structure). yaml-test-suite: `{key: v}` is
        // a standalone flow mapping; `[a]: outer` is a block-map key.
        let mut flow_depth: i32 = 0;
        while i < n {
            let ch = self.char_cache[i];
            match ch {
                '\n' | '\r' => return false,
                '[' | '{' => flow_depth += 1,
                ']' | '}' => flow_depth -= 1,
                ':' if flow_depth <= 0 => {
                    let next = self.char_cache.get(i + 1).copied();
                    match next {
                        None => return true,
                        Some(c) if c.is_whitespace() => return true,
                        _ => {}
                    }
                }
                _ => {}
            }
            i += 1;
        }
        false
    }

    /// Check if there's an active mapping at the specified indentation level
    /// This method properly handles BlockEnd tokens by tracking mapping start/end pairs
    fn check_active_mapping_at_level(&self, _target_indent: usize) -> bool {
        let mut depth = 0;

        // Walk backwards through tokens to find the innermost unmatched block start.
        // Every BlockEnd increments depth; BlockMappingStart and BlockSequenceStart
        // decrement it (both open blocks that need a matching BlockEnd).
        // When depth == 0 we have found the block start that is still "open".
        for token in self.tokens.iter().rev() {
            match &token.token_type {
                TokenType::BlockMappingStart => {
                    if depth == 0 {
                        // The innermost open block is a mapping — active at this level.
                        return true;
                    }
                    depth -= 1;
                }
                TokenType::BlockSequenceStart => {
                    if depth == 0 {
                        // The innermost open block is a sequence, not a mapping.
                        return false;
                    }
                    depth -= 1;
                }
                TokenType::BlockEnd => {
                    depth += 1;
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

    /// Drive the parser pipeline on `input` in a dedicated thread, returning
    /// `None` if it doesn't finish within `Duration::from_secs(2)`. Used by
    /// regression tests for parser hangs so a still-broken parser doesn't
    /// block the whole `cargo test` run.
    fn parse_with_timeout(input: &str) -> Option<Vec<crate::parser::Event>> {
        use crate::parser::{BasicParser, Parser as ParserTrait};
        use std::sync::mpsc;
        use std::thread;
        use std::time::Duration;

        let owned = input.to_string();
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let mut p = BasicParser::new_eager(owned);
            let _ = p.take_scanning_error();
            let mut events = Vec::new();
            loop {
                match p.get_event() {
                    Ok(Some(ev)) => events.push(ev),
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
            let _ = tx.send(events);
        });
        rx.recv_timeout(Duration::from_secs(2)).ok()
    }

    /// Regression: `---` directly followed by non-space text used to spin the
    /// scanner forever because the `-` match arm at line-start dispatched to
    /// `scan_document_start` (which correctly returned None) and then to
    /// `is_plain_scalar_start` (which returns false for `-`, so no consumption
    /// occurred — outer `while let` re-entered with the same char). Fix:
    /// fall through to `scan_plain_scalar` unconditionally when not a doc
    /// marker — the guard already ensures the char is non-whitespace.
    /// See yaml-test-suite tests 82AN / EXG3.
    #[test]
    fn three_dashes_directly_followed_by_text_does_not_hang() {
        let events = parse_with_timeout("---word1\nword2\n")
            .expect("parser hung — `---word1` should not produce an infinite loop");
        // We must produce at least one scalar whose value starts with `---`,
        // proving that the dashes were consumed as part of a plain scalar
        // (not interpreted as a document marker, which would consume them
        // separately).
        let starts_with_dashes = events.iter().any(|e| {
            matches!(&e.event_type,
                crate::parser::EventType::Scalar { value, .. } if value.starts_with("---")
            )
        });
        assert!(
            starts_with_dashes,
            "expected a plain scalar starting with `---`, got events: {events:?}"
        );
    }

    /// YAML 1.2 §7.3.3: `?`, `:`, and `-` may start a plain scalar provided
    /// the next character is non-space (and, in flow context, not a flow
    /// indicator). The previous `is_plain_scalar_start` unconditionally
    /// rejected those three characters, so plain scalars like `?foo`,
    /// `:foo`, `-foo` were reported as `Invalid character`.
    /// Tracked by yaml-test-suite 2EBW.
    #[test]
    fn question_mark_followed_by_text_starts_plain_scalar() {
        use crate::parser::{BasicParser, EventType, Parser as ParserTrait};
        let mut p = BasicParser::new_eager("?foo: bar\n".to_string());
        assert!(p.take_scanning_error().is_none());
        let mut keys = Vec::new();
        while let Ok(Some(ev)) = p.get_event() {
            if let EventType::Scalar { value, .. } = ev.event_type {
                keys.push(value);
            }
        }
        assert_eq!(keys, vec!["?foo", "bar"]);
    }

    #[test]
    fn colon_followed_by_text_starts_plain_scalar() {
        use crate::parser::{BasicParser, EventType, Parser as ParserTrait};
        let mut p = BasicParser::new_eager(":foo: bar\n".to_string());
        assert!(p.take_scanning_error().is_none());
        let mut keys = Vec::new();
        while let Ok(Some(ev)) = p.get_event() {
            if let EventType::Scalar { value, .. } = ev.event_type {
                keys.push(value);
            }
        }
        assert_eq!(keys, vec![":foo", "bar"]);
    }

    /// YAML 1.2: every started document must be closed with a DocumentEnd
    /// event before StreamEnd. The previous `TokenType::StreamEnd` handler
    /// only emitted `-DOC` for `DocumentContent` / `BlockNode` states —
    /// the `DocumentStart` state (entered after `---` and a single scalar
    /// like `"foo"`) was skipped, dropping the `-DOC` event. Affected by
    /// yaml-test-suite 27NA, 2G84/*, 2LFX and several others.
    #[test]
    fn explicit_doc_with_only_a_scalar_emits_doc_end_before_stream_end() {
        use crate::parser::{BasicParser, EventType, Parser as ParserTrait};
        let mut p = BasicParser::new_eager("---\n\"foo\"\n".to_string());
        assert!(p.take_scanning_error().is_none());
        let mut kinds = Vec::new();
        while let Ok(Some(ev)) = p.get_event() {
            kinds.push(match ev.event_type {
                EventType::StreamStart => "+STR",
                EventType::StreamEnd => "-STR",
                EventType::DocumentStart { .. } => "+DOC",
                EventType::DocumentEnd { .. } => "-DOC",
                EventType::Scalar { .. } => "=VAL",
                _ => "?",
            });
        }
        // Critical: -DOC must come before -STR.
        let doc_end_idx = kinds.iter().position(|s| *s == "-DOC");
        let str_end_idx = kinds.iter().position(|s| *s == "-STR");
        assert!(
            doc_end_idx.is_some(),
            "missing -DOC in event stream: {kinds:?}"
        );
        assert!(
            doc_end_idx < str_end_idx,
            "expected -DOC before -STR, got {kinds:?}"
        );
    }

    /// YAML 1.2 §5.7 hex / Unicode escapes in double-quoted strings.
    #[test]
    fn double_quoted_hex_escapes_decode_to_codepoint() {
        use crate::parser::{BasicParser, EventType, Parser as ParserTrait};
        for (input, expected) in [
            (r#""\x41""#, "A"),
            (r#""é""#, "é"),
            (r#""\U0001F600""#, "\u{1f600}"),
        ] {
            let mut p = BasicParser::new_eager(input.to_string());
            assert!(
                p.take_scanning_error().is_none(),
                "no scan error for {input}"
            );
            let mut found = None;
            while let Ok(Some(ev)) = p.get_event() {
                if let EventType::Scalar { value, .. } = ev.event_type {
                    found = Some(value);
                    break;
                }
            }
            assert_eq!(found.as_deref(), Some(expected), "input {input}");
        }
    }

    #[test]
    fn truncated_hex_escape_is_a_scan_error() {
        use crate::parser::BasicParser;
        let mut p = BasicParser::new_eager(r#""\x4""#.to_string());
        assert!(
            p.take_scanning_error().is_some(),
            "truncated \\x escape must error"
        );
    }

    /// YAML 1.2 §5.7: double-quoted strings have a strict allowlist of escape
    /// sequences. `\.` (and any other unknown escape) must be reported as a
    /// scan error. Tracked by yaml-test-suite 55WF.
    #[test]
    fn invalid_double_quoted_escape_is_a_scan_error() {
        use crate::parser::{BasicParser, Parser as ParserTrait};
        let mut p = BasicParser::new_eager("---\n\"\\.\"\n".to_string());
        let scan_err = p.take_scanning_error();
        let mut parse_err = false;
        if scan_err.is_none() {
            loop {
                match p.get_event() {
                    Ok(Some(_)) => {}
                    Ok(None) => break,
                    Err(_) => {
                        parse_err = true;
                        break;
                    }
                }
            }
        }
        assert!(
            scan_err.is_some() || parse_err,
            "`\\.` is not a valid double-quoted escape and must error"
        );
    }

    /// YAML 1.2: a complex-key marker (`?`) is the first content after an
    /// explicit document start (`---`) — it should open an implicit block
    /// mapping. The previous parser handled `?` only in
    /// `ImplicitDocumentStart` / `DocumentContent` / already-in-mapping
    /// states and errored out for `DocumentStart`, breaking inputs like
    /// `--- !!set\n? Mark McGwire\n...`. Tracked by yaml-test-suite 2XXW.
    #[test]
    fn complex_key_directly_after_explicit_doc_start_opens_mapping() {
        use crate::parser::{BasicParser, EventType, Parser as ParserTrait};
        let mut p = BasicParser::new_eager("--- !!set\n? Mark McGwire\n? Sammy Sosa\n".to_string());
        assert!(p.take_scanning_error().is_none());
        let mut saw_map_start = false;
        let mut saw_error = false;
        loop {
            match p.get_event() {
                Ok(Some(ev)) => {
                    if matches!(ev.event_type, EventType::MappingStart { .. }) {
                        saw_map_start = true;
                    }
                }
                Ok(None) => break,
                Err(_) => {
                    saw_error = true;
                    break;
                }
            }
        }
        assert!(!saw_error, "complex key after `--- !!set` must not error");
        assert!(saw_map_start, "expected a MappingStart event");
    }

    /// YAML 1.2 §6.9.2: anchor / alias names exclude only whitespace and
    /// the flow indicators `,[]{}`. Earlier implementations restricted
    /// `scan_identifier` to ASCII alphanumeric / `_` / `-`, which rejected
    /// valid unicode anchors like `&😁`. Tracked by yaml-test-suite 8XYN.
    #[test]
    fn anchor_name_may_contain_unicode_symbols() {
        use crate::parser::{BasicParser, EventType, Parser as ParserTrait};
        let mut p = BasicParser::new_eager("---\n- &😁 unicode anchor\n".to_string());
        assert!(
            p.take_scanning_error().is_none(),
            "unicode anchor must not error"
        );
        let mut anchors = Vec::new();
        while let Ok(Some(ev)) = p.get_event() {
            if let EventType::Scalar {
                anchor: Some(a), ..
            } = ev.event_type
            {
                anchors.push(a);
            }
        }
        assert_eq!(anchors, vec!["😁"]);
    }

    /// YAML 1.2 §5.6 / RFC 3986 percent-encoding: tag suffixes may contain
    /// `%XX` percent-escaped characters, which must be URI-decoded when
    /// resolved. The scanner used to reject `%` in tag suffixes as
    /// "Invalid character", so e.g. `!e!tag%21 baz` failed before the
    /// resolver got a chance to decode it. Tracked by yaml-test-suite 6CK3.
    #[test]
    fn tag_suffix_with_percent_escape_resolves_to_decoded_uri() {
        use crate::parser::{BasicParser, EventType, Parser as ParserTrait};
        let mut p = BasicParser::new_eager(
            "%TAG !e! tag:example.com,2000:app/\n---\n- !e!tag%21 baz\n".to_string(),
        );
        assert!(
            p.take_scanning_error().is_none(),
            "tag percent-escapes must not error"
        );
        let mut tags = Vec::new();
        while let Ok(Some(ev)) = p.get_event() {
            if let EventType::Scalar { tag: Some(t), .. } = ev.event_type {
                tags.push(t);
            }
        }
        assert_eq!(tags, vec!["tag:example.com,2000:app/tag!"]);
    }

    /// YAML 1.2 §6.8.4: "A YAML processor should ignore any directive it
    /// does not recognize." A `%FOO` reserved directive must NOT be treated
    /// as a scan error — the directive line is silently skipped and parsing
    /// continues. Tracked by yaml-test-suite test 2LFX.
    #[test]
    fn reserved_directive_is_ignored_not_an_error() {
        use crate::parser::{BasicParser, EventType, Parser as ParserTrait};
        let mut p = BasicParser::new_eager(
            "%FOO  bar baz # Should be ignored\n              # with a warning.\n---\n\"foo\"\n"
                .to_string(),
        );
        assert!(
            p.take_scanning_error().is_none(),
            "unknown directives must NOT produce a scan error"
        );
        let mut scalars = Vec::new();
        while let Ok(Some(ev)) = p.get_event() {
            if let EventType::Scalar { value, .. } = ev.event_type {
                scalars.push(value);
            }
        }
        assert_eq!(scalars, vec!["foo"]);
    }

    /// Spec requires the two physical lines of `---word1\nword2` to fold into
    /// a single plain scalar `"---word1 word2"`. Tracked by yaml-test-suite 82AN.
    #[test]
    fn three_dashes_followed_by_text_folds_continuation_line() {
        let events = parse_with_timeout("---word1\nword2\n").expect("parser hung");
        let scalars: Vec<&str> = events
            .iter()
            .filter_map(|e| match &e.event_type {
                crate::parser::EventType::Scalar { value, .. } => Some(value.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(scalars, vec!["---word1 word2"]);
    }

    /// Regression: tab between block-entry marker and a `-N` value used to
    /// hang the scanner via the same `-` match arm. See yaml-test-suite
    /// Y79Y/010.
    #[test]
    fn dash_tab_negative_number_does_not_hang() {
        let events = parse_with_timeout("-\t-1\n")
            .expect("parser hung — `-\\t-1` should not produce an infinite loop");
        assert!(!events.is_empty(), "expected event stream, got none");
    }

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
        // YAML 1.2 §5.7 double-quoted escape sequences. Single-quoted strings
        // have NO backslash escapes — `''` is the only escape — so this set
        // is restricted to the double-quoted cases.
        let test_cases = vec![
            (r#""Line 1\nLine 2""#, "Line 1\nLine 2"),
            (r#""Col1\tCol2""#, "Col1\tCol2"),
            (r#""First\rSecond""#, "First\rSecond"),
            (r#""Path\\to\\file""#, "Path\\to\\file"),
            (r#""He said \"Hello\"""#, "He said \"Hello\""),
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
            (r#""\/""#, "/"),    // literal forward slash
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
        // YAML 1.2 §5.7: unknown double-quoted escapes are scan errors, not
        // preserved literals. (Earlier versions of this scanner kept the
        // backslash + char verbatim — see commit history.)
        for input in [r#""\z""#, r#""\q""#, r#""\8""#] {
            let mut scanner = BasicScanner::new(input.to_string());
            scanner.get_token().unwrap(); // StreamStart
            assert!(
                scanner.get_token().is_err(),
                "expected scan error for invalid escape in {input}"
            );
        }
    }
}
