//! Comment-preserving YAML composer

use crate::{
    parser::EventType, BasicParser, BasicScanner, CommentedValue, Comments, Error, Limits, Parser,
    Position, ResourceTracker, Result, Scanner, Style, TokenType, Value,
};
use indexmap::IndexMap;
use std::collections::HashMap;

/// A composer that preserves comments during parsing
#[derive(Debug)]
pub struct CommentPreservingComposer {
    parser: BasicParser,
    scanner: BasicScanner,
    limits: Limits,
    resource_tracker: ResourceTracker,
    anchors: HashMap<String, CommentedValue>,
    current_depth: usize,
    alias_expansion_stack: Vec<String>,
    /// Map of positions to comments (position -> comment text)
    comment_map: HashMap<Position, String>,
    /// Stack of pending comments that might belong to the next value
    pending_comments: Vec<String>,
}

impl CommentPreservingComposer {
    /// Create a new comment-preserving composer
    pub fn new(input: String) -> Self {
        Self::with_limits(input, Limits::default())
    }

    /// Create a new comment-preserving composer with limits
    pub fn with_limits(input: String, limits: Limits) -> Self {
        // Use comment-preserving scanner
        let scanner = BasicScanner::new_with_comments_and_limits(input.clone(), limits.clone());
        let parser = BasicParser::new_eager_with_limits(input, limits.clone());

        Self {
            parser,
            scanner,
            limits,
            resource_tracker: ResourceTracker::new(),
            anchors: HashMap::new(),
            current_depth: 0,
            alias_expansion_stack: Vec::new(),
            comment_map: HashMap::new(),
            pending_comments: Vec::new(),
        }
    }

    /// Extract comments from the scanner and build a position map
    fn extract_comments(&mut self) -> Result<()> {
        // Scan all tokens to extract comments
        while self.scanner.check_token() {
            if let Some(token) = self.scanner.get_token()? {
                if let TokenType::Comment(comment_text) = token.token_type {
                    // Store comment associated with its position
                    self.comment_map
                        .insert(token.start_position, comment_text.trim().to_string());
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Get comments that should be associated with a value at the given position
    fn get_comments_for_position(&self, position: Position) -> Comments {
        let mut comments = Comments::new();

        // Enhanced comment correlation algorithm
        for (comment_pos, comment_text) in &self.comment_map {
            let line_diff = comment_pos.line as i32 - position.line as i32;

            // Comments on the same line after the value (trailing)
            if comment_pos.line == position.line && comment_pos.column > position.column {
                comments.set_trailing(comment_text.clone());
            }
            // Comments on lines before the value (leading)
            else if (-3..0).contains(&line_diff) {
                // Allow up to 3 lines before as leading comments
                comments.add_leading(comment_text.clone());
            }
            // Comments on the same line before the value (also leading)
            else if comment_pos.line == position.line && comment_pos.column < position.column {
                comments.add_leading(comment_text.clone());
            }
            // Comments immediately after (next line) could be inner comments
            else if line_diff == 1 {
                comments.add_inner(comment_text.clone());
            }
        }

        comments
    }

    /// Compose a single document with comment preservation
    pub fn compose_document(&mut self) -> Result<Option<CommentedValue>> {
        // First, extract all comments from the scanner
        self.extract_comments()?;

        // Reset state
        self.current_depth = 0;
        self.anchors.clear();
        self.alias_expansion_stack.clear();
        self.resource_tracker.reset();

        // Compose the document
        self.compose_node()
    }

    /// Compose a single node (value) with comments
    fn compose_node(&mut self) -> Result<Option<CommentedValue>> {
        // Check resource limits
        self.resource_tracker.add_complexity(&self.limits, 1)?;
        self.current_depth += 1;

        if self.current_depth > self.limits.max_depth {
            return Err(Error::limit_exceeded(format!(
                "Maximum nesting depth {} exceeded",
                self.limits.max_depth
            )));
        }

        // Get the next event from the parser
        let event = match self.parser.get_event()? {
            Some(event) => event,
            None => {
                self.current_depth -= 1;
                return Ok(None);
            }
        };

        let position = event.position;
        let result = match event.event_type {
            EventType::Scalar { value, anchor, .. } => self.compose_scalar(value, anchor, position),
            EventType::SequenceStart { anchor, .. } => self.compose_sequence(anchor, position),
            EventType::MappingStart { anchor, .. } => self.compose_mapping(anchor, position),
            EventType::Alias { anchor } => self.compose_alias(anchor, position),
            EventType::StreamStart | EventType::StreamEnd => {
                // Skip structural events and try next
                self.compose_node()
            }
            EventType::DocumentStart { .. } | EventType::DocumentEnd { .. } => {
                // Skip document markers and try next
                self.compose_node()
            }
            EventType::SequenceEnd | EventType::MappingEnd => {
                // These should be handled by their respective start handlers
                // If we encounter them here, it means unbalanced structure
                Ok(None)
            }
        };

        self.current_depth -= 1;
        result
    }

    /// Compose a scalar value
    fn compose_scalar(
        &mut self,
        value: String,
        anchor: Option<String>,
        position: Position,
    ) -> Result<Option<CommentedValue>> {
        // Resolve the scalar type properly
        let resolved_value = self.resolve_scalar_type(value);

        let commented_value = CommentedValue {
            value: resolved_value,
            comments: self.get_comments_for_position(position),
            style: Style::default(),
        };

        // Store anchor if present
        if let Some(anchor_name) = anchor {
            self.anchors.insert(anchor_name, commented_value.clone());
        }

        Ok(Some(commented_value))
    }

    /// Resolve scalar type from string value
    fn resolve_scalar_type(&self, value: String) -> Value {
        // Empty string
        if value.is_empty() {
            return Value::String(value);
        }

        // Try integer parsing
        if let Ok(int_value) = value.parse::<i64>() {
            return Value::Int(int_value);
        }

        // Try float parsing
        if let Ok(float_value) = value.parse::<f64>() {
            return Value::Float(float_value);
        }

        // Try boolean parsing
        match value.to_lowercase().as_str() {
            "true" | "yes" | "on" => return Value::Bool(true),
            "false" | "no" | "off" => return Value::Bool(false),
            "null" | "~" => return Value::Null,
            _ => {}
        }

        // Default to string
        Value::String(value)
    }

    /// Compose a sequence
    fn compose_sequence(
        &mut self,
        anchor: Option<String>,
        position: Position,
    ) -> Result<Option<CommentedValue>> {
        let mut sequence = Vec::new();
        let mut inner_comments = Vec::new();

        // Collect sequence items
        while let Some(item_event) = self.parser.peek_event()? {
            if matches!(item_event.event_type, EventType::SequenceEnd) {
                self.parser.get_event()?; // consume SequenceEnd
                break;
            }

            if let Some(item) = self.compose_node()? {
                self.collect_item_comments(&item, &mut inner_comments);
                sequence.push(item.value);
            }
        }

        let mut comments = self.get_comments_for_position(position);
        comments.inner = inner_comments;

        let commented_value = CommentedValue {
            value: Value::Sequence(sequence),
            comments,
            style: Style::default(),
        };

        // Store anchor if present
        if let Some(anchor_name) = anchor {
            self.anchors.insert(anchor_name, commented_value.clone());
        }

        Ok(Some(commented_value))
    }

    /// Compose a mapping
    fn compose_mapping(
        &mut self,
        anchor: Option<String>,
        position: Position,
    ) -> Result<Option<CommentedValue>> {
        let mut mapping = IndexMap::new();
        let mut inner_comments = Vec::new();

        // Collect mapping items
        while let Some(event) = self.parser.peek_event()? {
            if matches!(event.event_type, EventType::MappingEnd) {
                self.parser.get_event()?; // consume MappingEnd
                break;
            }

            // Get key
            let (key, key_comments) = match self.compose_node()? {
                Some(key_commented) => (key_commented.value, key_commented.comments),
                None => break,
            };

            // Get value
            let (value, value_comments) = match self.compose_node()? {
                Some(value_commented) => (value_commented.value, value_commented.comments),
                None => (Value::Null, Comments::new()),
            };

            // Collect comments from key-value pairs
            self.collect_comments(&key_comments, &mut inner_comments);
            self.collect_comments(&value_comments, &mut inner_comments);

            // Handle merge keys
            if let Value::String(key_str) = &key {
                if key_str == "<<" {
                    self.process_merge_key(&mut mapping, &value)?;
                    continue;
                }
            }

            mapping.insert(key, value);
        }

        let mut comments = self.get_comments_for_position(position);
        comments.inner.extend(inner_comments);

        let commented_value = CommentedValue {
            value: Value::Mapping(mapping),
            comments,
            style: Style::default(),
        };

        // Store anchor if present
        if let Some(anchor_name) = anchor {
            self.anchors.insert(anchor_name, commented_value.clone());
        }

        Ok(Some(commented_value))
    }

    /// Compose an alias reference
    fn compose_alias(
        &mut self,
        anchor: String,
        position: Position,
    ) -> Result<Option<CommentedValue>> {
        // Prevent cyclic references
        if self.alias_expansion_stack.contains(&anchor) {
            return Err(Error::parse(
                position,
                format!("Cyclic alias reference detected: '{}'", anchor),
            ));
        }

        self.alias_expansion_stack.push(anchor.clone());

        let result = match self.anchors.get(&anchor) {
            Some(value) => Ok(Some(value.clone())),
            None => Err(Error::parse(
                position,
                format!("Unknown anchor '{}'", anchor),
            )),
        };

        self.alias_expansion_stack.pop();
        result
    }

    /// Collect comments from a commented value's comments into inner comments
    fn collect_item_comments(&self, item: &CommentedValue, inner_comments: &mut Vec<String>) {
        if item.has_comments() {
            for leading in &item.comments.leading {
                inner_comments.push(leading.clone());
            }
            if let Some(ref trailing) = item.comments.trailing {
                inner_comments.push(trailing.clone());
            }
        }
    }

    /// Collect comments from a Comments struct into inner comments
    fn collect_comments(&self, comments: &Comments, inner_comments: &mut Vec<String>) {
        if !comments.leading.is_empty() || comments.trailing.is_some() {
            for leading in &comments.leading {
                inner_comments.push(leading.clone());
            }
            if let Some(ref trailing) = comments.trailing {
                inner_comments.push(trailing.clone());
            }
        }
    }

    /// Process a merge key by merging values into the current mapping
    fn process_merge_key(
        &self,
        mapping: &mut IndexMap<Value, Value>,
        merge_value: &Value,
    ) -> Result<()> {
        match merge_value {
            Value::Mapping(source_map) => {
                for (key, value) in source_map {
                    mapping.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }
            Value::Sequence(sources) => {
                for source in sources {
                    if let Value::Mapping(source_map) = source {
                        for (key, value) in source_map {
                            mapping.entry(key.clone()).or_insert_with(|| value.clone());
                        }
                    }
                }
            }
            _ => {
                // Invalid merge value - ignore
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_preservation() {
        let yaml = r#"
# Leading comment
key: value  # Trailing comment
# Another comment
nested:
  # Nested comment
  item: data
"#;

        let mut composer = CommentPreservingComposer::new(yaml.to_string());
        let result = composer.compose_document().unwrap();

        assert!(result.is_some());
        let commented_value = result.unwrap();

        // Should have preserved some comments
        println!("Preserved comments: {:?}", commented_value.comments);
    }
}
