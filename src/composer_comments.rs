//! Comment-preserving YAML composer

use crate::{
    BasicParser, BasicScanner, CommentedValue, Comments, Error, Limits, Parser, Position,
    ResourceTracker, Result, Scanner, Style, TokenType, Value,
    composer::calculate_value_complexity, parser::EventType,
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
    /// Active YAML spec version for the current document.
    yaml_version: crate::version::YamlVersion,
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
            yaml_version: crate::version::YamlVersion::default(),
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
            EventType::DocumentStart { version, .. } => {
                // Capture the YAML version directive (if any) before recursing.
                self.yaml_version = version
                    .map(|(maj, min)| crate::version::YamlVersion::from_directive(maj, min))
                    .unwrap_or_default();
                self.compose_node()
            }
            EventType::DocumentEnd { .. } => {
                // Skip document end and try next
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
        let resolved_value = self.resolve_scalar_type(value, position)?;

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

    /// Resolve scalar type from string value (version-aware).
    ///
    /// Returns `Err` when the YAML 1.1 `=` value tag is detected — the
    /// `CommentedValue` tree has no tagged-scalar representation, so we
    /// surface the same error as the other composers (matches
    /// `ruamel.yaml typ="safe"`).
    fn resolve_scalar_type(&self, value: String, position: Position) -> Result<Value> {
        Ok(
            match crate::resolver::resolve_plain_scalar(&value, self.yaml_version) {
                crate::resolver::PlainScalarType::Null => Value::Null,
                crate::resolver::PlainScalarType::Bool(b) => Value::Bool(b),
                crate::resolver::PlainScalarType::Int(i) => Value::Int(i),
                crate::resolver::PlainScalarType::Float(f) => Value::Float(f),
                crate::resolver::PlainScalarType::Str => Value::String(value),
                crate::resolver::PlainScalarType::Value => {
                    return Err(crate::resolver::value_tag_error(position));
                }
            },
        )
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

        // Cap alias expansion depth (parity with BasicComposer).
        if self.alias_expansion_stack.len() >= self.limits.max_alias_depth {
            return Err(Error::parse(
                position,
                format!(
                    "Maximum alias expansion depth {} exceeded",
                    self.limits.max_alias_depth
                ),
            ));
        }

        self.alias_expansion_stack.push(anchor.clone());

        let result = match self.anchors.get(&anchor) {
            Some(value) => {
                // Cap cumulative alias materialization BEFORE the clone —
                // closes the billion-laughs gap on the comment-preserving
                // load path (#15).
                let nodes = calculate_value_complexity(&value.value)?;
                self.resource_tracker
                    .add_alias_materialization(&self.limits, nodes)?;
                self.resource_tracker.add_complexity(&self.limits, nodes)?;
                Ok(Some(value.clone()))
            }
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
