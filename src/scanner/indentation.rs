//! Indentation management for YAML scanner

use super::{Token, TokenType};
use crate::{Error, Position, Result};

/// Indentation style detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum IndentationStyle {
    Spaces(usize),
    Tabs,
    Mixed,
    Unknown,
}

/// Indentation manager for tracking YAML indentation levels
#[derive(Debug)]
pub struct IndentationManager {
    /// Stack of indentation levels
    pub indent_stack: Vec<usize>,
    /// Detected indentation style
    pub indentation_style: IndentationStyle,
    /// Current indentation level
    pub current_indent: usize,
    /// Whether we've analyzed the indentation pattern
    pattern_analyzed: bool,
}

impl IndentationManager {
    /// Create a new indentation manager
    pub fn new() -> Self {
        Self {
            indent_stack: vec![0],
            indentation_style: IndentationStyle::Unknown,
            current_indent: 0,
            pattern_analyzed: false,
        }
    }

    /// Reset the indentation manager
    pub fn reset(&mut self) {
        self.indent_stack.clear();
        self.indent_stack.push(0);
        self.indentation_style = IndentationStyle::Unknown;
        self.current_indent = 0;
        self.pattern_analyzed = false;
    }

    /// Push a new indentation level
    pub fn push_indent(&mut self, level: usize) {
        self.indent_stack.push(level);
        self.current_indent = level;
    }

    /// Pop an indentation level
    pub fn pop_indent(&mut self) -> Option<usize> {
        if self.indent_stack.len() > 1 {
            let level = self.indent_stack.pop();
            self.current_indent = *self.indent_stack.last().unwrap_or(&0);
            level
        } else {
            None
        }
    }

    /// Get the current indentation level
    pub fn current_level(&self) -> usize {
        *self.indent_stack.last().unwrap_or(&0)
    }

    /// Check if we're at a dedent position
    pub fn is_dedent(&self, column: usize) -> bool {
        column < self.current_level()
    }

    /// Check if we're at an indent position
    pub fn is_indent(&self, column: usize) -> bool {
        column > self.current_level()
    }

    /// Count dedent levels needed
    pub fn count_dedents(&self, column: usize) -> usize {
        self.indent_stack
            .iter()
            .rev()
            .take_while(|&&level| level > column)
            .count()
    }

    /// Analyze indentation pattern from a line
    pub fn analyze_pattern(&mut self, line: &str) -> IndentationStyle {
        if self.pattern_analyzed {
            return self.indentation_style;
        }

        let mut spaces = 0;
        let mut tabs = 0;

        for ch in line.chars() {
            match ch {
                ' ' => spaces += 1,
                '\t' => tabs += 1,
                _ => break,
            }
        }

        self.indentation_style = if tabs > 0 && spaces > 0 {
            IndentationStyle::Mixed
        } else if tabs > 0 {
            IndentationStyle::Tabs
        } else if spaces > 0 {
            // Try to detect common space widths (2, 4, 8)
            for &width in &[2, 4, 8] {
                if spaces % width == 0 {
                    self.indentation_style = IndentationStyle::Spaces(width);
                    break;
                }
            }
            if self.indentation_style == IndentationStyle::Unknown {
                self.indentation_style = IndentationStyle::Spaces(spaces);
            }
            self.indentation_style
        } else {
            IndentationStyle::Unknown
        };

        self.pattern_analyzed = true;
        self.indentation_style
    }

    /// Validate indentation consistency
    pub fn validate_indentation(&self, column: usize, position: Position) -> Result<()> {
        match self.indentation_style {
            IndentationStyle::Spaces(width) if width > 0 => {
                if column % width != 0 {
                    return Err(Error::scan(
                        position,
                        format!(
                            "Inconsistent indentation: expected multiple of {} spaces, got {}",
                            width, column
                        ),
                    ));
                }
            }
            IndentationStyle::Mixed => {
                return Err(Error::scan(
                    position,
                    "Mixed indentation (tabs and spaces) is not allowed",
                ));
            }
            _ => {}
        }
        Ok(())
    }

    /// Generate BlockEnd tokens for dedentation
    pub fn generate_block_ends(&mut self, column: usize, position: Position) -> Vec<Token> {
        let mut tokens = Vec::new();
        let dedent_count = self.count_dedents(column);

        for _ in 0..dedent_count {
            self.pop_indent();
            tokens.push(Token::simple(TokenType::BlockEnd, position));
        }

        tokens
    }
}

impl Default for IndentationManager {
    fn default() -> Self {
        Self::new()
    }
}
