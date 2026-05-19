//! YAML composer for converting events to nodes

use crate::resolver::{PlainScalarType, resolve_plain_scalar, value_tag_error};
#[cfg(test)]
use crate::scanner::Scanner;
use crate::tag::TagResolver;
use crate::version::YamlVersion;
use crate::{
    BasicParser, Error, Limits, Parser, Position, ResourceTracker, Result, Value, parser::EventType,
};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Calculate complexity score for a value (for resource limiting).
///
/// Iterative on purpose: pathological documents can produce arbitrarily
/// deep `Value` trees (e.g. via anchors), and a recursive implementation
/// stack-overflows long before any in-process limit fires (#16).
pub(crate) fn calculate_value_complexity(value: &Value) -> Result<usize> {
    let mut total: usize = 0;
    let mut stack: Vec<&Value> = vec![value];
    while let Some(node) = stack.pop() {
        match node {
            Value::Sequence(seq) => {
                total = total.saturating_add(1usize.saturating_add(seq.len()));
                for item in seq {
                    stack.push(item);
                }
            }
            Value::Mapping(map) => {
                total = total.saturating_add(1usize.saturating_add(map.len().saturating_mul(2)));
                for (k, v) in map {
                    stack.push(k);
                    stack.push(v);
                }
            }
            _ => total = total.saturating_add(1),
        }
    }
    Ok(total)
}

/// Calculate the maximum nesting depth of a value structure.
///
/// Iterative DFS with an explicit work-list, for the same stack-safety
/// reason as [`calculate_value_complexity`] (#16).
fn calculate_structure_depth(value: &Value) -> usize {
    let mut max_depth: usize = 1;
    let mut stack: Vec<(&Value, usize)> = vec![(value, 1)];
    while let Some((node, depth)) = stack.pop() {
        if depth > max_depth {
            max_depth = depth;
        }
        let next = depth.saturating_add(1);
        match node {
            Value::Sequence(seq) => {
                for item in seq {
                    stack.push((item, next));
                }
            }
            Value::Mapping(map) => {
                for (_, v) in map {
                    stack.push((v, next));
                }
            }
            _ => {}
        }
    }
    max_depth
}

/// Trait for YAML composers that convert event streams to node structures
pub trait Composer {
    /// Check if there are more documents available
    fn check_document(&self) -> bool;

    /// Compose the next document
    ///
    /// # Errors
    /// Returns an error if parsing or composition fails
    fn compose_document(&mut self) -> Result<Option<Value>>;

    /// Get the current position in the stream
    fn position(&self) -> Position;

    /// Reset the composer state
    fn reset(&mut self);
}

/// A basic composer implementation for converting events to nodes
#[derive(Debug)]
pub struct BasicComposer {
    parser: BasicParser,
    position: Position,
    anchors: HashMap<String, Value>,
    limits: Limits,
    resource_tracker: ResourceTracker,
    alias_expansion_stack: Vec<String>,
    current_depth: usize,
    tag_resolver: TagResolver,
    /// Active YAML spec version for the current document, set from the
    /// `%YAML` directive (when present) on each `DocumentStart` event.
    /// Defaults to [`YamlVersion::V1_2`].
    yaml_version: YamlVersion,
}

impl BasicComposer {
    /// Create a new composer from input string
    #[must_use]
    pub fn new(input: String) -> Self {
        Self::with_limits(input, Limits::default())
    }

    /// Create a new composer with custom limits
    #[must_use]
    pub fn with_limits(input: String, limits: Limits) -> Self {
        Self {
            parser: BasicParser::with_limits(input, limits.clone()),
            position: Position::new(),
            anchors: HashMap::new(),
            limits,
            resource_tracker: ResourceTracker::new(),
            alias_expansion_stack: Vec::new(),
            current_depth: 0,
            tag_resolver: TagResolver::new(),
            yaml_version: YamlVersion::default(),
        }
    }

    /// Create a new composer with eager parsing (for compatibility)
    #[must_use]
    pub fn new_eager(input: String) -> Self {
        Self::new_eager_with_limits(input, Limits::default())
    }

    /// Create a new composer with eager parsing and custom limits
    #[must_use]
    pub fn new_eager_with_limits(input: String, limits: Limits) -> Self {
        Self {
            parser: BasicParser::new_eager_with_limits(input, limits.clone()),
            position: Position::new(),
            anchors: HashMap::new(),
            limits,
            resource_tracker: ResourceTracker::new(),
            alias_expansion_stack: Vec::new(),
            current_depth: 0,
            tag_resolver: TagResolver::new(),
            yaml_version: YamlVersion::default(),
        }
    }

    /// Compose a node from events (recursive)
    fn compose_node(&mut self) -> Result<Option<Value>> {
        if !self.parser.check_event() {
            return Ok(None);
        }

        let Some(event) = self.parser.get_event()? else {
            return Ok(None);
        };

        self.position = event.position;

        match event.event_type {
            EventType::StreamStart | EventType::StreamEnd => {
                // Skip stream boundaries, these don't produce nodes
                self.compose_node()
            }

            EventType::DocumentStart { version, .. } => {
                // Capture the YAML version directive (if any) so plain-scalar
                // resolution in compose_scalar honors `%YAML 1.1`. The
                // compose_document peek loop also extracts it for the
                // implicit-StreamStart case, but reach this arm when events
                // are consumed via compose_node directly.
                self.yaml_version = version
                    .map(|(maj, min)| YamlVersion::from_directive(maj, min))
                    .unwrap_or_default();
                self.compose_node()
            }

            EventType::DocumentEnd { .. } => {
                // Document end, return None to indicate end of document
                Ok(None)
            }

            EventType::Scalar {
                value,
                anchor,
                tag,
                style,
                ..
            } => {
                let scalar_value = if let Some(tag_str) = tag {
                    // Apply tag if present
                    self.compose_tagged_scalar(value, tag_str)?
                } else {
                    // Use implicit typing
                    self.compose_scalar(value, style, event.position)?
                };

                // Store anchor if present
                if let Some(anchor_name) = anchor {
                    self.resource_tracker.add_anchor(&self.limits)?;
                    self.anchors.insert(anchor_name, scalar_value.clone());
                }

                Ok(Some(scalar_value))
            }

            EventType::SequenceStart { anchor, .. } => {
                let sequence = self.compose_sequence()?;

                // Store anchor if present
                if let Some(anchor_name) = anchor {
                    if let Some(ref seq) = sequence {
                        self.resource_tracker.add_anchor(&self.limits)?;
                        self.anchors.insert(anchor_name, seq.clone());
                    }
                }

                Ok(sequence)
            }

            EventType::MappingStart { anchor, .. } => {
                let mapping = self.compose_mapping()?;

                // Store anchor if present
                if let Some(anchor_name) = anchor {
                    if let Some(ref map) = mapping {
                        self.resource_tracker.add_anchor(&self.limits)?;
                        self.anchors.insert(anchor_name, map.clone());
                    }
                }

                Ok(mapping)
            }

            EventType::SequenceEnd | EventType::MappingEnd => {
                // These collection end events should normally be handled by their respective compose methods.
                // However, if we encounter them here, it means we're in an unexpected state.
                // This can happen when the parser generates a flattened structure instead of proper nesting.
                // Return None to indicate we've reached the end of the current node.
                Ok(None)
            }

            EventType::Alias { anchor } => {
                // Check for cyclic references
                if self.alias_expansion_stack.contains(&anchor) {
                    return Err(Error::construction(
                        event.position,
                        format!("Cyclic alias reference detected: '{anchor}'"),
                    ));
                }

                // Check alias expansion depth limit BEFORE pushing
                if self.alias_expansion_stack.len() >= self.limits.max_alias_depth {
                    return Err(Error::construction(
                        event.position,
                        format!(
                            "Maximum alias expansion depth {} exceeded",
                            self.limits.max_alias_depth
                        ),
                    ));
                }

                // Track alias expansion depth
                self.resource_tracker.enter_alias(&self.limits)?;
                self.alias_expansion_stack.push(anchor.clone());

                // Resolve alias to the anchored value
                let result = match self.anchors.get(&anchor) {
                    Some(value) => {
                        // Check if the resolved value's structure depth would exceed alias depth limit
                        let structure_depth = calculate_structure_depth(value);
                        if structure_depth > self.limits.max_alias_depth {
                            return Err(Error::construction(
                                event.position,
                                format!(
                                    "Alias '{}' creates structure with depth {} exceeding max_alias_depth {}",
                                    anchor, structure_depth, self.limits.max_alias_depth
                                ),
                            ));
                        }

                        let nodes = calculate_value_complexity(value)?;
                        // Cap cumulative alias materialization BEFORE the
                        // clone — closes the billion-laughs gap where wide
                        // fan-out allocates millions of nodes before
                        // max_complexity_score fires (#15).
                        self.resource_tracker
                            .add_alias_materialization(&self.limits, nodes)?;
                        self.resource_tracker.add_complexity(&self.limits, nodes)?;
                        Ok(Some(value.clone()))
                    }
                    None => Err(Error::construction(
                        event.position,
                        format!("Unknown anchor '{anchor}'"),
                    )),
                };

                // Clean up tracking
                self.alias_expansion_stack.pop();
                self.resource_tracker.exit_alias();

                result
            }
        }
    }

    /// Compose a scalar value.
    ///
    /// Single- and double-quoted scalars always become `Value::String`.
    /// Plain, literal, and folded scalars go through the shared
    /// [`resolve_plain_scalar`] helper so the YAML version (1.1 vs 1.2)
    /// governs which boolean forms are recognized.
    ///
    /// `position` is the scalar's source position, used only to anchor
    /// the error returned for the YAML 1.1 `!!value` (`=`) tag.
    fn compose_scalar(
        &self,
        value: String,
        style: crate::parser::ScalarStyle,
        position: crate::Position,
    ) -> Result<Value> {
        match style {
            crate::parser::ScalarStyle::SingleQuoted | crate::parser::ScalarStyle::DoubleQuoted => {
                return Ok(Value::String(value));
            }
            _ => {}
        }

        Ok(match resolve_plain_scalar(&value, self.yaml_version) {
            PlainScalarType::Null => Value::Null,
            PlainScalarType::Bool(b) => Value::Bool(b),
            PlainScalarType::Int(i) => Value::Int(i),
            PlainScalarType::Float(f) => Value::Float(f),
            PlainScalarType::Str => Value::String(value),
            PlainScalarType::Value => return Err(value_tag_error(position)),
        })
    }

    /// Compose a tagged scalar value
    fn compose_tagged_scalar(&mut self, value: String, tag_str: String) -> Result<Value> {
        // Resolve the tag (TagResolver should handle already-resolved URIs)
        let tag = self.tag_resolver.resolve(&tag_str)?;

        // Apply the tag to the value
        self.tag_resolver.apply_tag(&tag, &value)
    }

    /// Compose a sequence
    fn compose_sequence(&mut self) -> Result<Option<Value>> {
        // Track depth
        self.current_depth += 1;
        self.resource_tracker
            .check_depth(&self.limits, self.current_depth)?;

        let mut sequence = Vec::new();

        while self.parser.check_event() {
            // Peek at the next event to see if we're at the end
            if let Ok(Some(event)) = self.parser.peek_event() {
                if matches!(event.event_type, EventType::SequenceEnd) {
                    // Consume the end event
                    self.parser.get_event()?;
                    break;
                } else if matches!(
                    event.event_type,
                    EventType::DocumentEnd { .. }
                        | EventType::DocumentStart { .. }
                        | EventType::StreamEnd
                ) {
                    // Don't consume these - let compose_document handle them
                    break;
                }
            }

            // Compose the next element
            if let Some(node) = self.compose_node()? {
                self.resource_tracker.add_collection_item(&self.limits)?;
                self.resource_tracker.add_complexity(&self.limits, 1)?;
                sequence.push(node);
            } else {
                // If compose_node returns None, we might have hit a document boundary
                break;
            }
        }

        self.current_depth -= 1;
        Ok(Some(Value::Sequence(sequence)))
    }

    /// Compose a mapping
    fn compose_mapping(&mut self) -> Result<Option<Value>> {
        // Track depth
        self.current_depth += 1;
        self.resource_tracker
            .check_depth(&self.limits, self.current_depth)?;

        let mut mapping = IndexMap::new();

        while self.parser.check_event() {
            // Peek at the next event to see if we're at the end
            if let Ok(Some(event)) = self.parser.peek_event() {
                if matches!(event.event_type, EventType::MappingEnd) {
                    // Consume the end event
                    self.parser.get_event()?;
                    break;
                } else if matches!(
                    event.event_type,
                    EventType::DocumentEnd { .. }
                        | EventType::DocumentStart { .. }
                        | EventType::StreamEnd
                ) {
                    // Don't consume these - let compose_document handle them
                    break;
                }
            }

            // Compose key
            let Some(key) = self.compose_node()? else {
                break;
            };

            // Compose value
            let value = self.compose_node()?.unwrap_or(Value::Null);

            // Check for merge key (YAML 1.2 specification)
            if let Value::String(key_str) = &key {
                if key_str == "<<" {
                    // Handle merge key - the value should already be resolved by compose_node()
                    self.process_merge_key(&mut mapping, &value)?;
                    continue;
                }
            }

            self.resource_tracker.add_collection_item(&self.limits)?;
            self.resource_tracker.add_complexity(&self.limits, 2)?; // Key-value pair
            mapping.insert(key, value);
        }

        self.current_depth -= 1;
        Ok(Some(Value::Mapping(mapping)))
    }

    /// Process a merge key by merging values into the current mapping
    /// The `merge_value` should already be resolved by `compose_node()`
    fn process_merge_key(
        &self,
        mapping: &mut IndexMap<Value, Value>,
        merge_value: &Value,
    ) -> Result<()> {
        match merge_value {
            // Single mapping to merge
            Value::Mapping(source_map) => {
                for (key, value) in source_map {
                    // Only insert if key doesn't already exist (explicit keys override merged keys)
                    mapping.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }

            // Sequence of mappings to merge (in order)
            Value::Sequence(sources) => {
                for source in sources {
                    if let Value::Mapping(source_map) = source {
                        for (key, value) in source_map {
                            // Only insert if key doesn't already exist
                            mapping.entry(key.clone()).or_insert_with(|| value.clone());
                        }
                    } else {
                        return Err(Error::construction(
                            self.position,
                            "Merge key sequence can only contain mappings",
                        ));
                    }
                }
            }

            _ => {
                return Err(Error::construction(
                    self.position,
                    "Merge key value must be a mapping or sequence of mappings",
                ));
            }
        }

        Ok(())
    }
}

impl Composer for BasicComposer {
    fn check_document(&self) -> bool {
        // Check if there are events that could form a document
        if let Ok(Some(event)) = self.parser.peek_event() {
            !matches!(event.event_type, EventType::StreamEnd)
        } else {
            false
        }
    }

    fn compose_document(&mut self) -> Result<Option<Value>> {
        // Check for parser scanning errors first
        if let Some(error) = self.parser.take_scanning_error() {
            return Err(error);
        }

        // Process document start events and extract tag directives + YAML version.
        while let Ok(Some(event)) = self.parser.peek_event() {
            if let EventType::DocumentStart { tags, version, .. } = &event.event_type {
                // Reset YAML version per document (directives don't carry across).
                self.yaml_version = version
                    .map(|(maj, min)| YamlVersion::from_directive(maj, min))
                    .unwrap_or_default();

                // Clear previous document's tag directives
                self.tag_resolver.clear_directives();

                // Add new tag directives from this document
                for (handle, prefix) in tags {
                    self.tag_resolver
                        .add_directive(handle.clone(), prefix.clone());
                }

                self.parser.get_event()?; // consume the DocumentStart
            } else if matches!(event.event_type, EventType::DocumentStart { .. }) {
                self.parser.get_event()?; // consume the DocumentStart
            } else {
                break;
            }
        }

        // Compose the actual document content
        let document = self.compose_node()?;

        // Skip any document end event
        while let Ok(Some(event)) = self.parser.peek_event() {
            if matches!(event.event_type, EventType::DocumentEnd { .. }) {
                self.parser.get_event()?; // consume the DocumentEnd
            } else {
                break;
            }
        }

        Ok(document)
    }

    fn position(&self) -> Position {
        self.position
    }

    fn reset(&mut self) {
        self.position = Position::new();
        self.anchors.clear();
        self.resource_tracker.reset();
        self.alias_expansion_stack.clear();
        self.current_depth = 0;
        self.tag_resolver = TagResolver::new();
    }
}

impl Default for BasicComposer {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    /// Build a sequence-of-sequence chain `depth` levels deep without using
    /// the parser, so we can stress the complexity/depth helpers past any
    /// `max_depth` the parser would normally enforce.
    fn build_deep_sequence(depth: usize) -> Value {
        let mut v = Value::Int(1);
        for _ in 0..depth {
            v = Value::Sequence(vec![v]);
        }
        v
    }

    /// Tear down a deep Value iteratively so the test's cleanup phase
    /// doesn't stack-overflow inside `Value::drop` / `Vec::drop`.
    /// This is only needed because `Value`'s default `Drop` is recursive.
    fn drop_deep(mut v: Value) {
        let mut stack: Vec<Value> = Vec::new();
        stack.push(std::mem::replace(&mut v, Value::Null));
        while let Some(node) = stack.pop() {
            if let Value::Sequence(seq) = node {
                for item in seq {
                    stack.push(item);
                }
            }
        }
    }

    #[test]
    fn iterative_complexity_handles_100k_deep_value() {
        // Regression for #16: a recursive implementation would
        // stack-overflow long before reaching this depth (Rust's default
        // 8 MB stack at ~120 bytes/frame ≈ 65 k frames). The iterative
        // implementation must return a sane count without panicking.
        let deep = build_deep_sequence(100_000);
        let complexity = calculate_value_complexity(&deep).expect("must not error");
        // Each Sequence contributes 1 (self) + 1 (len), plus the inner
        // scalar contributes 1. So 100_000 * 2 + 1 = 200_001.
        assert_eq!(complexity, 200_001);
        drop_deep(deep);
    }

    #[test]
    fn iterative_structure_depth_handles_100k_deep_value() {
        // Same regression for calculate_structure_depth (#16).
        let deep = build_deep_sequence(100_000);
        let depth = calculate_structure_depth(&deep);
        // 100_000 wrapping sequences + the inner scalar leaf = 100_001.
        assert_eq!(depth, 100_001);
        drop_deep(deep);
    }

    #[test]
    fn test_check_document() {
        let mut composer = BasicComposer::new_eager("42".to_string());
        assert!(composer.check_document());

        let _document = composer.compose_document().unwrap();
        // After composing, may or may not have more documents depending on implementation
    }

    #[test]
    fn test_scalar_composition() {
        let mut composer = BasicComposer::new_eager("42".to_string());
        let document = composer.compose_document().unwrap().unwrap();
        assert_eq!(document, Value::Int(42));
    }

    #[test]
    fn test_boolean_composition() {
        let mut composer = BasicComposer::new_eager("true".to_string());
        let document = composer.compose_document().unwrap().unwrap();
        assert_eq!(document, Value::Bool(true));
    }

    #[test]
    fn test_null_composition() {
        let mut composer = BasicComposer::new_eager("~".to_string());
        let document = composer.compose_document().unwrap().unwrap();
        assert_eq!(document, Value::Null);
    }

    #[test]
    fn test_string_composition() {
        let mut composer = BasicComposer::new_eager("hello world".to_string());
        let document = composer.compose_document().unwrap().unwrap();
        assert_eq!(document, Value::String("hello world".to_string()));
    }

    #[test]
    fn test_flow_sequence_composition() {
        let mut composer = BasicComposer::new_eager("[1, 2, 3]".to_string());
        let document = composer.compose_document().unwrap().unwrap();

        let expected = Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(document, expected);
    }

    #[test]
    fn test_flow_mapping_composition() {
        let mut composer = BasicComposer::new_eager("{'key': 'value', 'number': 42}".to_string());
        let document = composer.compose_document().unwrap().unwrap();

        let mut expected_map = IndexMap::new();
        expected_map.insert(
            Value::String("key".to_string()),
            Value::String("value".to_string()),
        );
        expected_map.insert(Value::String("number".to_string()), Value::Int(42));
        let expected = Value::Mapping(expected_map);

        assert_eq!(document, expected);
    }

    #[test]
    fn test_nested_composition() {
        let yaml_content = "{'users': [{'name': 'Alice', 'age': 30}]}";
        let mut composer = BasicComposer::new_eager(yaml_content.to_string());
        let document = composer.compose_document().unwrap().unwrap();

        // Build expected structure
        let mut user = IndexMap::new();
        user.insert(
            Value::String("name".to_string()),
            Value::String("Alice".to_string()),
        );
        user.insert(Value::String("age".to_string()), Value::Int(30));

        let users = Value::Sequence(vec![Value::Mapping(user)]);

        let mut expected = IndexMap::new();
        expected.insert(Value::String("users".to_string()), users);

        assert_eq!(document, Value::Mapping(expected));
    }

    #[test]
    fn test_multiple_types() {
        let yaml_content = "[42, 'hello', true, null]";
        let mut composer = BasicComposer::new_eager(yaml_content.to_string());
        let document = composer.compose_document().unwrap().unwrap();

        let expected = Value::Sequence(vec![
            Value::Int(42),
            Value::String("hello".to_string()),
            Value::Bool(true),
            Value::Null,
        ]);

        assert_eq!(document, expected);
    }

    #[test]
    fn test_merge_keys_simple() {
        let yaml_content = r"
base: &base
  key: value
  timeout: 30

test:
  <<: *base
  environment: prod
";
        let mut composer = BasicComposer::new_eager(yaml_content.to_string());
        let document = composer.compose_document().unwrap().unwrap();

        if let Value::Mapping(ref map) = document {
            // Check that base mapping exists
            assert!(map.contains_key(&Value::String("base".to_string())));

            // Check that test mapping exists and has merged keys
            if let Some(Value::Mapping(test_map)) = map.get(&Value::String("test".to_string())) {
                assert!(test_map.contains_key(&Value::String("key".to_string())));
                assert!(test_map.contains_key(&Value::String("timeout".to_string())));
                assert!(test_map.contains_key(&Value::String("environment".to_string())));

                // Verify values
                assert_eq!(
                    test_map.get(&Value::String("key".to_string())),
                    Some(&Value::String("value".to_string()))
                );
                assert_eq!(
                    test_map.get(&Value::String("timeout".to_string())),
                    Some(&Value::Int(30))
                );
                assert_eq!(
                    test_map.get(&Value::String("environment".to_string())),
                    Some(&Value::String("prod".to_string()))
                );
            } else {
                panic!("test mapping not found or not a mapping");
            }
        } else {
            panic!("Document should be a mapping, got: {:?}", document);
        }
    }

    #[test]
    fn test_debug_alias_tokens() {
        let yaml_content = r"
base: &base
  key: value

ref: *base
";

        let mut scanner = crate::BasicScanner::new_eager(yaml_content.to_string());

        println!("Scanning tokens for alias test:");
        let mut token_count = 0;

        while scanner.check_token() {
            if let Ok(Some(token)) = scanner.get_token() {
                token_count += 1;
                println!(
                    "{}: {:?} at {:?}-{:?}",
                    token_count, token.token_type, token.start_position, token.end_position
                );
            } else {
                println!("No more tokens");
                break;
            }
        }

        println!("Total tokens: {}", token_count);
    }

    #[test]
    fn test_debug_alias_events() {
        let yaml_content = r"
base: &base
  key: value

ref: *base
";

        let mut parser = BasicParser::new_eager(yaml_content.to_string());

        println!("Parsing events for alias test:");
        let mut event_count = 0;

        while parser.check_event() {
            if let Ok(Some(event)) = parser.get_event() {
                event_count += 1;
                println!(
                    "{}: {:?} at {:?}",
                    event_count, event.event_type, event.position
                );
            } else {
                println!("No more events");
                break;
            }
        }

        println!("Total events: {}", event_count);
    }

    #[test]
    fn test_simple_scalar_alias_resolution() {
        // Test with a simple scalar alias first
        let yaml_content = r"
base: &base 'hello world'
ref: *base
";
        let mut composer = BasicComposer::new_eager(yaml_content.to_string());
        let document = composer.compose_document().unwrap().unwrap();

        println!("Simple alias document: {:?}", document);

        if let Value::Mapping(ref map) = document {
            println!("Mapping keys: {:?}", map.keys().collect::<Vec<_>>());

            let base_value = map
                .get(&Value::String("base".to_string()))
                .expect("base should exist");
            let ref_value = map
                .get(&Value::String("ref".to_string()))
                .expect("ref should exist");

            println!("base_value: {:?}", base_value);
            println!("ref_value: {:?}", ref_value);

            assert_eq!(base_value, ref_value);
        } else {
            panic!("Document should be a mapping, got: {:?}", document);
        }
    }

    #[test]
    fn test_basic_alias_resolution() {
        let yaml_content = r"
base: &base
  key: value

ref: *base
";
        let mut composer = BasicComposer::new_eager(yaml_content.to_string());
        let document = composer.compose_document().unwrap().unwrap();

        println!("Composed document: {:?}", document);

        if let Value::Mapping(ref map) = document {
            println!("Mapping keys: {:?}", map.keys().collect::<Vec<_>>());

            // Check that both base and ref exist and are equal
            let base_value = map
                .get(&Value::String("base".to_string()))
                .expect("base should exist");
            let ref_value = map
                .get(&Value::String("ref".to_string()))
                .expect("ref should exist");

            println!("base_value: {:?}", base_value);
            println!("ref_value: {:?}", ref_value);

            // Verify both values are the same nested mapping
            assert_eq!(
                base_value, ref_value,
                "Alias should resolve to the same value as the anchor"
            );

            // Verify the structure is correct
            if let Value::Mapping(nested) = base_value {
                assert_eq!(
                    nested.get(&Value::String("key".to_string())),
                    Some(&Value::String("value".to_string()))
                );
            } else {
                panic!("base value should be a nested mapping");
            }

            println!("✅ Alias resolution working correctly!");
        } else {
            panic!("Document should be a mapping, got: {:?}", document);
        }
    }
}
