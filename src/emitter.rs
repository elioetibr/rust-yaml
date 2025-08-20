//! YAML emitter for generating text output

use crate::{CommentedValue, Comments, Error, IndentStyle, QuoteStyle, Result, Value};
use std::collections::HashMap;
use std::io::Write;

/// Trait for YAML emitters that generate text output from values
pub trait Emitter {
    /// Emit a value to the output
    fn emit<W: Write>(&mut self, value: &Value, writer: W) -> Result<()>;

    /// Emit a commented value to the output with comment preservation
    fn emit_commented<W: Write>(&mut self, value: &CommentedValue, writer: W) -> Result<()>;

    /// Emit a commented value with specific indent style
    fn emit_with_style<W: Write>(
        &mut self,
        value: &CommentedValue,
        indent_style: &IndentStyle,
        writer: W,
    ) -> Result<()>;

    /// Reset the emitter state
    fn reset(&mut self);
}

/// Information about shared values for anchor/alias emission
#[derive(Debug, Clone)]
struct ValueInfo {
    anchor_name: String,
    first_occurrence: bool,
}

/// Basic emitter implementation that generates clean YAML
#[derive(Debug)]
pub struct BasicEmitter {
    indent: usize,
    current_indent: usize,
    shared_values: HashMap<Value, ValueInfo>,
    anchor_counter: usize,
    indent_style: IndentStyle,
    yaml_version: Option<(u8, u8)>,
    tag_directives: Vec<(String, String)>,
}

#[allow(dead_code)]
impl BasicEmitter {
    /// Create a new emitter with default settings
    pub fn new() -> Self {
        Self {
            indent: 2,
            current_indent: 0,
            shared_values: HashMap::new(),
            anchor_counter: 0,
            indent_style: IndentStyle::default(),
            yaml_version: None,
            tag_directives: Vec::new(),
        }
    }

    /// Create an emitter with custom indent
    pub fn with_indent(indent: usize) -> Self {
        Self {
            indent,
            current_indent: 0,
            shared_values: HashMap::new(),
            anchor_counter: 0,
            indent_style: IndentStyle::Spaces(indent),
            yaml_version: None,
            tag_directives: Vec::new(),
        }
    }

    /// Create an emitter with specific indent style
    pub fn with_indent_style(indent_style: IndentStyle) -> Self {
        let indent = match &indent_style {
            IndentStyle::Spaces(width) => *width,
            IndentStyle::Tabs => 1, // Tabs count as 1 indent level
        };
        Self {
            indent,
            current_indent: 0,
            shared_values: HashMap::new(),
            anchor_counter: 0,
            indent_style,
            yaml_version: None,
            tag_directives: Vec::new(),
        }
    }

    /// Set the YAML version directive
    pub fn set_yaml_version(&mut self, major: u8, minor: u8) {
        self.yaml_version = Some((major, minor));
    }

    /// Add a TAG directive
    pub fn add_tag_directive(&mut self, handle: String, prefix: String) {
        self.tag_directives.push((handle, prefix));
    }

    /// Clear all directives
    pub fn clear_directives(&mut self) {
        self.yaml_version = None;
        self.tag_directives.clear();
    }

    /// Emit directives to the writer
    fn emit_directives<W: Write>(&self, writer: &mut W) -> Result<()> {
        // Emit YAML version directive if set
        if let Some((major, minor)) = self.yaml_version {
            writeln!(writer, "%YAML {}.{}", major, minor).map_err(|e| Error::Emission {
                message: format!("Failed to write YAML directive: {}", e),
            })?;
        }

        // Emit TAG directives
        for (handle, prefix) in &self.tag_directives {
            writeln!(writer, "%TAG {} {}", handle, prefix).map_err(|e| Error::Emission {
                message: format!("Failed to write TAG directive: {}", e),
            })?;
        }

        // If we emitted any directives, emit document start marker
        if self.yaml_version.is_some() || !self.tag_directives.is_empty() {
            writeln!(writer, "---").map_err(|e| Error::Emission {
                message: format!("Failed to write document start marker: {}", e),
            })?;
        }

        Ok(())
    }

    /// Analyze the value tree to identify shared values that need anchors
    fn analyze_shared_values(&mut self, value: &Value) {
        let mut value_counts = HashMap::new();
        self.count_value_occurrences(value, &mut value_counts);

        // Generate anchors for values that occur more than once and are complex
        for (val, count) in value_counts {
            if count > 1 && self.is_complex_value(&val) {
                let anchor_name = format!("anchor{}", self.anchor_counter);
                self.anchor_counter += 1;
                self.shared_values.insert(
                    val,
                    ValueInfo {
                        anchor_name,
                        first_occurrence: true,
                    },
                );
            }
        }
    }

    /// Recursively count occurrences of each value
    fn count_value_occurrences(&self, value: &Value, counts: &mut HashMap<Value, usize>) {
        // Only track complex values (sequences and mappings)
        if self.is_complex_value(value) {
            *counts.entry(value.clone()).or_insert(0) += 1;
        }

        // Recurse into child values
        match value {
            Value::Sequence(seq) => {
                for item in seq {
                    self.count_value_occurrences(item, counts);
                }
            }
            Value::Mapping(map) => {
                for (key, val) in map {
                    self.count_value_occurrences(key, counts);
                    self.count_value_occurrences(val, counts);
                }
            }
            _ => {}
        }
    }

    /// Check if a value is complex enough to warrant anchor/alias handling
    const fn is_complex_value(&self, value: &Value) -> bool {
        matches!(value, Value::Sequence(_) | Value::Mapping(_))
    }

    /// Generate next anchor name
    fn next_anchor_name(&mut self) -> String {
        let name = format!("anchor{}", self.anchor_counter);
        self.anchor_counter += 1;
        name
    }

    /// Update the indent style (useful for round-trip preservation)
    pub const fn set_indent_style(&mut self, indent_style: IndentStyle) {
        self.indent = match &indent_style {
            IndentStyle::Spaces(width) => *width,
            IndentStyle::Tabs => 1,
        };
        self.indent_style = indent_style;
    }

    /// Write indentation to the output
    fn write_indent<W: Write>(&self, writer: &mut W) -> Result<()> {
        match &self.indent_style {
            IndentStyle::Spaces(_width) => {
                let total_spaces = self.current_indent;
                for _ in 0..total_spaces {
                    write!(writer, " ")?;
                }
            }
            IndentStyle::Tabs => {
                let indent_levels = self.current_indent / self.indent;
                for _ in 0..indent_levels {
                    write!(writer, "\t")?;
                }
            }
        }
        Ok(())
    }

    /// Write leading comments to the output
    fn emit_leading_comments<W: Write>(&self, comments: &[String], writer: &mut W) -> Result<()> {
        for comment in comments {
            self.write_indent(writer)?;
            writeln!(writer, "# {}", comment)?;
        }
        Ok(())
    }

    /// Write a trailing comment on the same line
    fn emit_trailing_comment<W: Write>(&self, comment: &str, writer: &mut W) -> Result<()> {
        write!(writer, " # {}", comment)?;
        Ok(())
    }

    /// Write inner comments (between collection items)
    fn emit_inner_comments<W: Write>(&self, comments: &[String], writer: &mut W) -> Result<()> {
        for comment in comments {
            writeln!(writer)?;
            self.write_indent(writer)?;
            writeln!(writer, "# {}", comment)?;
        }
        Ok(())
    }

    /// Emit a scalar value
    fn emit_scalar<W: Write>(&self, value: &Value, writer: &mut W) -> Result<()> {
        self.emit_scalar_with_comments(value, None, writer)
    }

    /// Emit a scalar value with optional comments and style
    fn emit_scalar_with_comments<W: Write>(
        &self,
        value: &Value,
        comments: Option<&Comments>,
        writer: &mut W,
    ) -> Result<()> {
        self.emit_scalar_with_comments_and_style(value, comments, None, writer)
    }

    /// Emit a scalar value with optional comments and style information
    fn emit_scalar_with_comments_and_style<W: Write>(
        &self,
        value: &Value,
        comments: Option<&Comments>,
        quote_style: Option<&QuoteStyle>,
        writer: &mut W,
    ) -> Result<()> {
        // Emit leading comments
        if let Some(comments) = comments {
            self.emit_leading_comments(&comments.leading, writer)?;
        }

        // Emit the scalar value
        match value {
            Value::Null => write!(writer, "null")?,
            Value::Bool(b) => write!(writer, "{}", b)?,
            Value::Int(i) => write!(writer, "{}", i)?,
            Value::Float(f) => {
                // Handle special float values
                if f.is_nan() {
                    write!(writer, ".nan")?;
                } else if f.is_infinite() {
                    if f.is_sign_positive() {
                        write!(writer, ".inf")?;
                    } else {
                        write!(writer, "-.inf")?;
                    }
                } else {
                    // Ensure the float is written with decimal point to preserve type
                    if f.fract() == 0.0 {
                        write!(writer, "{:.1}", f)?;
                    } else {
                        write!(writer, "{}", f)?;
                    }
                }
            }
            Value::String(s) => {
                self.emit_string_with_style(s, quote_style, writer)?;
            }
            _ => return Err(Error::emission("Non-scalar passed to emit_scalar")),
        }

        // Emit trailing comment
        if let Some(comments) = comments {
            if let Some(ref trailing) = comments.trailing {
                self.emit_trailing_comment(trailing, writer)?;
            }
        }

        Ok(())
    }

    /// Emit a string, choosing appropriate quoting style
    fn emit_string<W: Write>(&self, s: &str, writer: &mut W) -> Result<()> {
        self.emit_string_with_style(s, None, writer)
    }

    /// Emit a string with specific quote style
    fn emit_string_with_style<W: Write>(
        &self,
        s: &str,
        preferred_style: Option<&QuoteStyle>,
        writer: &mut W,
    ) -> Result<()> {
        match preferred_style {
            Some(QuoteStyle::Single) => self.emit_single_quoted_string(s, writer),
            Some(QuoteStyle::Double) => self.emit_double_quoted_string(s, writer),
            Some(QuoteStyle::Plain) | None => {
                // Check if string needs quoting
                if self.needs_quoting(s) {
                    // Default to double quotes when quoting is needed
                    self.emit_double_quoted_string(s, writer)
                } else {
                    write!(writer, "{}", s)?;
                    Ok(())
                }
            }
        }
    }

    /// Check if a string needs to be quoted
    fn needs_quoting(&self, s: &str) -> bool {
        if s.is_empty() {
            return true;
        }

        // String needs quoting if it could be interpreted as another type
        if s == "null"
            || s == "~"
            || s == "true"
            || s == "false"
            || s == "yes"
            || s == "no"
            || s == "on"
            || s == "off"
            || s.parse::<i64>().is_ok()
            || s.parse::<f64>().is_ok()
        {
            return true;
        }

        // Check for version-like strings that could be ambiguous
        // e.g., "2.1.0", "1.2.3", etc. - these contain dots and could be misinterpreted
        if s.chars().any(|c| c == '.') && s.chars().any(|c| c.is_ascii_digit()) {
            // If it looks like a version number (contains dots and digits), quote it
            // to prevent parsing ambiguity
            return true;
        }

        // Check for special characters
        if s.contains('\n')
            || s.contains('\r')
            || s.contains('\t')
            || s.contains('"')
            || s.contains('\'')
            || s.contains(':')
            || s.contains('#')
            || s.contains('-')
            || s.contains('[')
            || s.contains(']')
            || s.contains('{')
            || s.contains('}')
            || s.starts_with(' ')
            || s.ends_with(' ')
        {
            return true;
        }

        false
    }

    /// Emit a double-quoted string
    fn emit_double_quoted_string<W: Write>(&self, s: &str, writer: &mut W) -> Result<()> {
        write!(writer, "\"")?;
        for ch in s.chars() {
            match ch {
                '"' => write!(writer, "\\\"")?,
                '\\' => write!(writer, "\\\\")?,
                '\n' => write!(writer, "\\n")?,
                '\r' => write!(writer, "\\r")?,
                '\t' => write!(writer, "\\t")?,
                c if c.is_control() => write!(writer, "\\u{:04x}", c as u32)?,
                c => write!(writer, "{}", c)?,
            }
        }
        write!(writer, "\"")?;
        Ok(())
    }

    /// Emit a single-quoted string
    fn emit_single_quoted_string<W: Write>(&self, s: &str, writer: &mut W) -> Result<()> {
        write!(writer, "'")?;
        for ch in s.chars() {
            match ch {
                '\'' => write!(writer, "''")?, // In YAML, single quotes are escaped by doubling
                c => write!(writer, "{}", c)?,
            }
        }
        write!(writer, "'")?;
        Ok(())
    }

    /// Emit a quoted string (legacy method)
    fn emit_quoted_string<W: Write>(&self, s: &str, writer: &mut W) -> Result<()> {
        self.emit_double_quoted_string(s, writer)
    }

    /// Emit a sequence (array/list)
    fn emit_sequence<W: Write>(&mut self, seq: &[Value], writer: &mut W) -> Result<()> {
        if seq.is_empty() {
            write!(writer, "[]")?;
            return Ok(());
        }

        for (index, item) in seq.iter().enumerate() {
            if index > 0 {
                writeln!(writer)?;
            }
            self.write_indent(writer)?;
            write!(writer, "- ")?;

            match item {
                Value::Sequence(_) | Value::Mapping(_) => {
                    writeln!(writer)?; // Add newline before nested structure
                    self.current_indent += self.indent;
                    self.emit_value(item, writer)?;
                    self.current_indent -= self.indent;
                }
                _ => {
                    self.emit_scalar(item, writer)?;
                }
            }
        }

        Ok(())
    }

    /// Emit a mapping with an anchor
    fn emit_mapping_with_anchor<W: Write>(
        &mut self,
        map: &indexmap::IndexMap<Value, Value>,
        anchor: &str,
        writer: &mut W,
    ) -> Result<()> {
        if map.is_empty() {
            write!(writer, "&{} {{}}", anchor)?;
            return Ok(());
        }

        // For anchored mappings, we need to format it as:
        // &anchor
        // key1: value1
        // key2: value2
        writeln!(writer, "&{}", anchor)?;

        let mut first = true;
        for (key, value) in map {
            if !first {
                writeln!(writer)?;
            }
            first = false;

            self.write_indent(writer)?;

            // Handle both simple and complex keys
            let is_complex_key = matches!(key, Value::Sequence(_) | Value::Mapping(_));

            if is_complex_key {
                // Complex key - emit with explicit key marker
                write!(writer, "? ")?;
                self.emit_value(key, writer)?;
                writeln!(writer)?;
                self.write_indent(writer)?;
            } else {
                // Simple key
                self.emit_scalar(key, writer)?;
            }

            write!(writer, ": ")?;

            match value {
                Value::Sequence(_) | Value::Mapping(_) => {
                    writeln!(writer)?; // Add newline before nested structure
                    self.current_indent += self.indent;
                    self.emit_value(value, writer)?;
                    self.current_indent -= self.indent;
                }
                _ => {
                    self.emit_scalar(value, writer)?;
                }
            }
        }

        Ok(())
    }

    /// Emit a mapping (dictionary/object)
    fn emit_mapping<W: Write>(
        &mut self,
        map: &indexmap::IndexMap<Value, Value>,
        writer: &mut W,
    ) -> Result<()> {
        if map.is_empty() {
            write!(writer, "{{}}")?;
            return Ok(());
        }

        let mut first = true;
        for (key, value) in map {
            if !first {
                writeln!(writer)?;
            }
            first = false;

            self.write_indent(writer)?;

            // Handle both simple and complex keys
            let is_complex_key = matches!(key, Value::Sequence(_) | Value::Mapping(_));

            if is_complex_key {
                // Complex key - emit with explicit key marker and flow style
                write!(writer, "? ")?;
                match key {
                    Value::Mapping(map) => {
                        // Emit mapping in flow style to avoid ambiguity
                        self.emit_mapping_flow_style(map, writer)?;
                    }
                    Value::Sequence(seq) => {
                        // Emit sequence in flow style
                        self.emit_sequence_flow_style(seq, writer)?;
                    }
                    _ => {
                        self.emit_value(key, writer)?;
                    }
                }
                writeln!(writer)?;
                self.write_indent(writer)?;
            } else {
                // Simple key
                self.emit_scalar(key, writer)?;
            }

            write!(writer, ": ")?;

            match value {
                Value::Sequence(_) | Value::Mapping(_) => {
                    writeln!(writer)?; // Add newline before nested structure
                    self.current_indent += self.indent;
                    self.emit_value(value, writer)?;
                    self.current_indent -= self.indent;
                }
                _ => {
                    self.emit_scalar(value, writer)?;
                }
            }
        }

        Ok(())
    }

    /// Emit a mapping in flow style for complex keys
    fn emit_mapping_flow_style<W: Write>(
        &self,
        map: &indexmap::IndexMap<Value, Value>,
        writer: &mut W,
    ) -> Result<()> {
        write!(writer, "{{")?;
        let mut first = true;
        for (key, value) in map {
            if !first {
                write!(writer, ", ")?;
            }
            first = false;

            // Emit key (handle nested complex values)
            match key {
                Value::Mapping(nested_map) => {
                    self.emit_mapping_flow_style(nested_map, writer)?;
                }
                Value::Sequence(nested_seq) => {
                    self.emit_sequence_flow_style(nested_seq, writer)?;
                }
                _ => {
                    self.emit_scalar(key, writer)?;
                }
            }

            write!(writer, ": ")?;

            // Emit value (handle nested complex values)
            match value {
                Value::Mapping(nested_map) => {
                    self.emit_mapping_flow_style(nested_map, writer)?;
                }
                Value::Sequence(nested_seq) => {
                    self.emit_sequence_flow_style(nested_seq, writer)?;
                }
                _ => {
                    self.emit_scalar(value, writer)?;
                }
            }
        }
        write!(writer, "}}")?;
        Ok(())
    }

    /// Emit a sequence in flow style for complex keys
    fn emit_sequence_flow_style<W: Write>(&self, seq: &[Value], writer: &mut W) -> Result<()> {
        write!(writer, "[")?;
        let mut first = true;
        for item in seq {
            if !first {
                write!(writer, ", ")?;
            }
            first = false;
            // Handle nested complex values
            match item {
                Value::Mapping(nested_map) => {
                    self.emit_mapping_flow_style(nested_map, writer)?;
                }
                Value::Sequence(nested_seq) => {
                    self.emit_sequence_flow_style(nested_seq, writer)?;
                }
                _ => {
                    self.emit_scalar(item, writer)?;
                }
            }
        }
        write!(writer, "]")?;
        Ok(())
    }

    /// Emit any value, dispatching to the appropriate method with anchor/alias support
    fn emit_value<W: Write>(&mut self, value: &Value, writer: &mut W) -> Result<()> {
        // Check if this value has an anchor/alias
        if let Some(info) = self.shared_values.get(value).cloned() {
            if info.first_occurrence {
                // First occurrence: emit with anchor
                match value {
                    Value::Sequence(seq) => {
                        write!(writer, "&{} ", info.anchor_name)?;
                        self.emit_sequence(seq, writer)?;
                    }
                    Value::Mapping(map) => {
                        self.emit_mapping_with_anchor(map, &info.anchor_name, writer)?;
                    }
                    _ => self.emit_scalar(value, writer)?, // Scalars shouldn't be shared
                }
                // Mark as no longer first occurrence
                if let Some(info_mut) = self.shared_values.get_mut(value) {
                    info_mut.first_occurrence = false;
                }
            } else {
                // Subsequent occurrence: emit alias
                write!(writer, "*{}", info.anchor_name)?;
            }
        } else {
            // Regular value without sharing
            match value {
                Value::Sequence(seq) => self.emit_sequence(seq, writer),
                Value::Mapping(map) => self.emit_mapping(map, writer),
                _ => self.emit_scalar(value, writer),
            }?;
        }
        Ok(())
    }

    /// Emit any value (old method for backwards compatibility)
    fn emit_value_simple<W: Write>(&mut self, value: &Value, writer: &mut W) -> Result<()> {
        match value {
            Value::Sequence(seq) => self.emit_sequence(seq, writer),
            Value::Mapping(map) => self.emit_mapping(map, writer),
            _ => self.emit_scalar(value, writer),
        }
    }

    /// Emit a commented value with full comment and style support
    fn emit_commented_value<W: Write>(
        &mut self,
        commented: &CommentedValue,
        writer: &mut W,
    ) -> Result<()> {
        let comments = if commented.has_comments() {
            Some(&commented.comments)
        } else {
            None
        };

        let quote_style = commented.quote_style();

        match &commented.value {
            Value::Sequence(_) | Value::Mapping(_) => {
                // For collections, emit leading comments first
                if let Some(comments) = comments {
                    self.emit_leading_comments(&comments.leading, writer)?;
                }
                self.emit_value(&commented.value, writer)?;

                // Emit inner comments for collections
                if let Some(comments) = comments {
                    if !comments.inner.is_empty() {
                        self.emit_inner_comments(&comments.inner, writer)?;
                    }
                }

                // Trailing comments for collections go on a new line
                if let Some(comments) = comments {
                    if let Some(ref trailing) = comments.trailing {
                        writeln!(writer)?;
                        self.write_indent(writer)?;
                        writeln!(writer, "# {}", trailing)?;
                    }
                }
            }
            _ => {
                // For scalars, use the scalar comment and style method
                self.emit_scalar_with_comments_and_style(
                    &commented.value,
                    comments,
                    quote_style,
                    writer,
                )?;
            }
        }

        Ok(())
    }

    /// Emit a CommentedValue with comment preservation (public API)
    pub fn emit_commented_value_public<W: Write>(
        &mut self,
        commented: &CommentedValue,
        writer: W,
    ) -> Result<()> {
        let mut writer = writer;
        self.emit_commented_value(commented, &mut writer)
    }
}

impl Default for BasicEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Emitter for BasicEmitter {
    fn emit<W: Write>(&mut self, value: &Value, mut writer: W) -> Result<()> {
        // Reset state
        self.current_indent = 0;
        self.shared_values.clear();
        self.anchor_counter = 0;

        // Emit directives if any
        self.emit_directives(&mut writer)?;

        // Analyze for shared values first
        self.analyze_shared_values(value);

        // For top-level sequences, add a leading newline for proper formatting
        if matches!(value, Value::Sequence(_)) {
            writeln!(writer)?;
        }

        // Emit the value
        self.emit_value(value, &mut writer)?;
        writeln!(writer)?; // Add final newline
        Ok(())
    }

    fn emit_commented<W: Write>(&mut self, value: &CommentedValue, mut writer: W) -> Result<()> {
        // Reset state
        self.current_indent = 0;
        self.shared_values.clear();
        self.anchor_counter = 0;

        // Emit directives if any
        self.emit_directives(&mut writer)?;

        // Analyze for shared values first
        self.analyze_shared_values(&value.value);

        // Emit the commented value
        self.emit_commented_value(value, &mut writer)?;
        writeln!(writer)?; // Add final newline
        Ok(())
    }

    fn emit_with_style<W: Write>(
        &mut self,
        value: &CommentedValue,
        indent_style: &IndentStyle,
        mut writer: W,
    ) -> Result<()> {
        // Store current style and temporarily update
        let original_style = self.indent_style.clone();
        self.set_indent_style(indent_style.clone());

        // Reset state
        self.current_indent = 0;
        self.shared_values.clear();
        self.anchor_counter = 0;

        // Emit directives if any
        self.emit_directives(&mut writer)?;

        // Analyze for shared values first
        self.analyze_shared_values(&value.value);

        // Emit the commented value with the specified style
        let result = self.emit_commented_value(value, &mut writer);
        if result.is_ok() {
            writeln!(writer)?; // Add final newline
        }

        // Restore original style
        self.set_indent_style(original_style);

        result
    }

    fn reset(&mut self) {
        self.current_indent = 0;
        self.shared_values.clear();
        self.anchor_counter = 0;
        // Note: We don't reset directives here as they might need to persist
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn test_emit_scalar() {
        let mut emitter = BasicEmitter::new();
        let mut output = Vec::new();

        emitter.emit(&Value::Int(42), &mut output).unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "42\n");
    }

    #[test]
    fn test_emit_string() {
        let mut emitter = BasicEmitter::new();
        let mut output = Vec::new();

        emitter
            .emit(&Value::String("hello".to_string()), &mut output)
            .unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "hello\n");
    }

    #[test]
    fn test_emit_quoted_string() {
        let mut emitter = BasicEmitter::new();
        let mut output = Vec::new();

        emitter
            .emit(&Value::String("true".to_string()), &mut output)
            .unwrap();
        assert_eq!(String::from_utf8(output).unwrap(), "\"true\"\n");
    }

    #[test]
    fn test_emit_sequence() {
        let mut emitter = BasicEmitter::new();
        let mut output = Vec::new();

        let seq = Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);

        emitter.emit(&seq, &mut output).unwrap();
        let result = String::from_utf8(output).unwrap();
        let expected = "\n- 1\n- 2\n- 3\n";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_emit_mapping() {
        let mut emitter = BasicEmitter::new();
        let mut output = Vec::new();

        let mut map = IndexMap::new();
        map.insert(
            Value::String("key".to_string()),
            Value::String("value".to_string()),
        );
        map.insert(Value::String("number".to_string()), Value::Int(42));

        emitter.emit(&Value::Mapping(map), &mut output).unwrap();
        let result = String::from_utf8(output).unwrap();

        // Should contain the key-value pairs
        assert!(result.contains("key: value"));
        assert!(result.contains("number: 42"));
    }

    #[test]
    fn test_emit_nested_structure() {
        let mut emitter = BasicEmitter::new();
        let mut output = Vec::new();

        let inner_seq = Value::Sequence(vec![Value::Int(1), Value::Int(2)]);
        let mut outer_map = IndexMap::new();
        outer_map.insert(Value::String("items".to_string()), inner_seq);

        emitter
            .emit(&Value::Mapping(outer_map), &mut output)
            .unwrap();
        let result = String::from_utf8(output).unwrap();

        assert!(result.contains("items:"));
        assert!(result.contains("- 1"));
        assert!(result.contains("- 2"));
    }
}
