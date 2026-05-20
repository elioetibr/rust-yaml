//! Zero-copy YAML composer for converting events to borrowed nodes
//!
//! This module provides a composer that minimizes allocations by using
//! borrowed data structures where possible.

use crate::{
    BasicParser, Error, Limits, Parser, Position, ResourceTracker, Result,
    parser::{EventType, ScalarStyle},
    value_borrowed::BorrowedValue,
};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Calculate the maximum nesting depth of a borrowed value structure
/// Iterative DFS — see [`crate::composer::calculate_structure_depth`] (#16).
fn calculate_borrowed_structure_depth(value: &BorrowedValue) -> usize {
    let mut max_depth: usize = 1;
    let mut stack: Vec<(&BorrowedValue, usize)> = vec![(value, 1)];
    while let Some((node, depth)) = stack.pop() {
        if depth > max_depth {
            max_depth = depth;
        }
        let next = depth.saturating_add(1);
        match node {
            BorrowedValue::Sequence(seq) => {
                for item in seq {
                    stack.push((item, next));
                }
            }
            BorrowedValue::Mapping(map) => {
                for (_, v) in map {
                    stack.push((v, next));
                }
            }
            _ => {}
        }
    }
    max_depth
}

/// Cost of materialising a borrowed value, used to charge alias expansion
/// against `max_total_alias_nodes` (#15). Iterative for stack safety (#16).
fn calculate_borrowed_complexity(value: &BorrowedValue) -> usize {
    let mut total: usize = 0;
    let mut stack: Vec<&BorrowedValue> = vec![value];
    while let Some(node) = stack.pop() {
        match node {
            BorrowedValue::Sequence(seq) => {
                total = total.saturating_add(1usize.saturating_add(seq.len()));
                for item in seq {
                    stack.push(item);
                }
            }
            BorrowedValue::Mapping(map) => {
                total = total.saturating_add(1usize.saturating_add(map.len().saturating_mul(2)));
                for (k, v) in map {
                    stack.push(k);
                    stack.push(v);
                }
            }
            _ => total = total.saturating_add(1),
        }
    }
    total
}

/// Trait for zero-copy YAML composers
pub trait BorrowedComposer<'a> {
    /// Check if there are more documents available
    fn check_document(&self) -> bool;

    /// Compose the next document with minimal allocations
    fn compose_document(&mut self) -> Result<Option<BorrowedValue<'a>>>;

    /// Get the current position in the stream
    fn position(&self) -> Position;

    /// Reset the composer state
    fn reset(&mut self);
}

/// A zero-copy composer implementation
pub struct ZeroCopyComposer<'a> {
    parser: BasicParser,
    position: Position,
    /// Store anchors as borrowed values when possible
    anchors: HashMap<&'a str, BorrowedValue<'a>>,
    limits: Limits,
    resource_tracker: ResourceTracker,
    alias_expansion_stack: Vec<&'a str>,
    current_depth: usize,
    /// Reference to the input string for borrowing
    input: &'a str,
    /// Active YAML spec version for the current document.
    yaml_version: crate::version::YamlVersion,
}

impl<'a> ZeroCopyComposer<'a> {
    /// Create a new zero-copy composer
    pub fn new(input: &'a str) -> Self {
        Self::with_limits(input, Limits::default())
    }

    /// Create a new zero-copy composer with custom limits
    pub fn with_limits(input: &'a str, limits: Limits) -> Self {
        Self {
            parser: BasicParser::with_limits(input.to_string(), limits.clone()),
            position: Position::new(),
            anchors: HashMap::new(),
            limits,
            resource_tracker: ResourceTracker::new(),
            alias_expansion_stack: Vec::new(),
            current_depth: 0,
            input,
            yaml_version: crate::version::YamlVersion::default(),
        }
    }

    /// Compose a node from events with minimal allocations
    fn compose_node(&mut self) -> Result<Option<BorrowedValue<'a>>> {
        if !self.parser.check_event() {
            return Ok(None);
        }

        let Some(event) = self.parser.get_event()? else {
            return Ok(None);
        };

        self.position = event.position;

        match event.event_type {
            EventType::StreamStart | EventType::StreamEnd => self.compose_node(),

            EventType::DocumentStart { .. } => self.compose_node(),

            EventType::DocumentEnd { .. } => Ok(None),

            EventType::Scalar {
                value,
                anchor,
                style,
                ..
            } => {
                let scalar_value = self.compose_scalar_borrowed(&value, style)?;

                // Store anchor if present - we need to clone here unfortunately
                if let Some(anchor_name) = anchor {
                    // We need to leak the string to get a 'static reference
                    // In a real implementation, we'd use an arena allocator
                    let anchor_str = Box::leak(anchor_name.into_boxed_str());
                    self.anchors
                        .insert(anchor_str, scalar_value.clone_if_needed());
                }

                Ok(Some(scalar_value))
            }

            EventType::SequenceStart { anchor, .. } => {
                let sequence = self.compose_sequence()?;

                // Store anchor if present
                if let Some(anchor_name) = anchor {
                    if let Some(ref seq) = sequence {
                        let anchor_str = Box::leak(anchor_name.into_boxed_str());
                        self.anchors.insert(anchor_str, seq.clone_if_needed());
                    }
                }

                Ok(sequence)
            }

            EventType::MappingStart { anchor, .. } => {
                let mapping = self.compose_mapping()?;

                // Store anchor if present
                if let Some(anchor_name) = anchor {
                    if let Some(ref map) = mapping {
                        let anchor_str = Box::leak(anchor_name.into_boxed_str());
                        self.anchors.insert(anchor_str, map.clone_if_needed());
                    }
                }

                Ok(mapping)
            }

            EventType::SequenceEnd | EventType::MappingEnd => Ok(None),

            EventType::Alias { anchor } => {
                // Check for cyclic references
                let anchor_str = anchor.as_str();
                if self.alias_expansion_stack.iter().any(|&a| a == anchor_str) {
                    return Err(Error::construction(
                        event.position,
                        format!("Cyclic alias reference detected: '{}'", anchor_str),
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

                // Track alias expansion
                self.resource_tracker.enter_alias(&self.limits)?;

                // Look up the anchor - try to avoid cloning if possible
                let result = match self.anchors.get(anchor_str) {
                    Some(value) => {
                        // Check if the resolved value's structure depth would exceed alias depth limit
                        let structure_depth = calculate_borrowed_structure_depth(value);
                        if structure_depth > self.limits.max_alias_depth {
                            return Err(Error::construction(
                                event.position,
                                format!(
                                    "Alias '{}' creates structure with depth {} exceeding max_alias_depth {}",
                                    anchor_str, structure_depth, self.limits.max_alias_depth
                                ),
                            ));
                        }

                        // Cap cumulative alias materialization BEFORE
                        // committing the clone (#15 billion-laughs gap).
                        let nodes = calculate_borrowed_complexity(value);
                        self.resource_tracker
                            .add_alias_materialization(&self.limits, nodes)?;
                        self.resource_tracker.add_complexity(&self.limits, nodes)?;
                        Ok(Some(value.clone_if_needed()))
                    }
                    None => Err(Error::construction(
                        event.position,
                        format!("Unknown anchor '{}'", anchor_str),
                    )),
                };

                self.resource_tracker.exit_alias();
                result
            }
        }
    }

    /// Compose a scalar value with borrowing when possible
    fn compose_scalar_borrowed(
        &self,
        value: &str,
        style: ScalarStyle,
    ) -> Result<BorrowedValue<'a>> {
        // Explicitly-quoted scalars are always strings.
        if matches!(style, ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted) {
            return Ok(BorrowedValue::owned_string(value.to_string()));
        }

        Ok(
            match crate::resolver::resolve_plain_scalar(value, self.yaml_version) {
                crate::resolver::PlainScalarType::Null => BorrowedValue::Null,
                crate::resolver::PlainScalarType::Bool(b) => BorrowedValue::Bool(b),
                crate::resolver::PlainScalarType::Int(i) => BorrowedValue::Int(i),
                crate::resolver::PlainScalarType::Float(f) => BorrowedValue::Float(f),
                crate::resolver::PlainScalarType::Str => {
                    BorrowedValue::owned_string(value.to_string())
                }
                crate::resolver::PlainScalarType::Value => {
                    return Err(crate::resolver::value_tag_error(self.position));
                }
            },
        )
    }

    /// Compose a sequence with minimal allocations
    fn compose_sequence(&mut self) -> Result<Option<BorrowedValue<'a>>> {
        self.current_depth += 1;
        self.resource_tracker
            .check_depth(&self.limits, self.current_depth)?;

        let mut sequence = Vec::new();

        while self.parser.check_event() {
            if let Ok(Some(event)) = self.parser.peek_event() {
                if matches!(event.event_type, EventType::SequenceEnd) {
                    self.parser.get_event()?;
                    break;
                } else if matches!(
                    event.event_type,
                    EventType::DocumentEnd { .. }
                        | EventType::DocumentStart { .. }
                        | EventType::StreamEnd
                ) {
                    break;
                }
            }

            if let Some(node) = self.compose_node()? {
                self.resource_tracker.add_collection_item(&self.limits)?;
                self.resource_tracker.add_complexity(&self.limits, 1)?;
                sequence.push(node);
            } else {
                break;
            }
        }

        self.current_depth -= 1;
        Ok(Some(BorrowedValue::Sequence(sequence)))
    }

    /// Compose a mapping with minimal allocations
    fn compose_mapping(&mut self) -> Result<Option<BorrowedValue<'a>>> {
        self.current_depth += 1;
        self.resource_tracker
            .check_depth(&self.limits, self.current_depth)?;

        let mut mapping = IndexMap::new();

        while self.parser.check_event() {
            if let Ok(Some(event)) = self.parser.peek_event() {
                if matches!(event.event_type, EventType::MappingEnd) {
                    self.parser.get_event()?;
                    break;
                } else if matches!(
                    event.event_type,
                    EventType::DocumentEnd { .. }
                        | EventType::DocumentStart { .. }
                        | EventType::StreamEnd
                ) {
                    break;
                }
            }

            let Some(key) = self.compose_node()? else {
                break;
            };

            let value = self.compose_node()?.unwrap_or(BorrowedValue::Null);

            self.resource_tracker.add_collection_item(&self.limits)?;
            self.resource_tracker.add_complexity(&self.limits, 2)?;

            mapping.insert(key, value);
        }

        self.current_depth -= 1;
        Ok(Some(BorrowedValue::Mapping(mapping)))
    }
}

impl<'a> BorrowedComposer<'a> for ZeroCopyComposer<'a> {
    fn check_document(&self) -> bool {
        if let Ok(Some(event)) = self.parser.peek_event() {
            !matches!(event.event_type, EventType::StreamEnd)
        } else {
            false
        }
    }

    fn compose_document(&mut self) -> Result<Option<BorrowedValue<'a>>> {
        if let Some(error) = self.parser.take_scanning_error() {
            return Err(error);
        }

        // Consume document start events, capturing the YAML version directive.
        while let Ok(Some(event)) = self.parser.peek_event() {
            if let EventType::DocumentStart { version, .. } = &event.event_type {
                self.yaml_version = version
                    .map(|(maj, min)| crate::version::YamlVersion::from_directive(maj, min))
                    .unwrap_or_default();
                self.parser.get_event()?;
            } else {
                break;
            }
        }

        let document = self.compose_node()?;

        // Skip any document end event
        while let Ok(Some(event)) = self.parser.peek_event() {
            if matches!(event.event_type, EventType::DocumentEnd { .. }) {
                self.parser.get_event()?;
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_copy_scalar() {
        let input = "hello world";
        let mut composer = ZeroCopyComposer::new(input);
        let result = composer.compose_document().unwrap().unwrap();

        // Verify we got a string (currently owned due to implementation limitations)
        if let BorrowedValue::String(cow) = result {
            // Note: Currently returns owned strings due to implementation limitations
            // TODO: Implement true zero-copy borrowing with arena allocator
            assert!(matches!(cow, std::borrow::Cow::Owned(_)));
            assert_eq!(cow.as_ref(), "hello world");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_zero_copy_sequence() {
        let input = "[1, 2, 3]";
        let mut composer = ZeroCopyComposer::new(input);
        let result = composer.compose_document().unwrap().unwrap();

        if let BorrowedValue::Sequence(seq) = result {
            assert_eq!(seq.len(), 3);
            assert_eq!(seq[0], BorrowedValue::Int(1));
            assert_eq!(seq[1], BorrowedValue::Int(2));
            assert_eq!(seq[2], BorrowedValue::Int(3));
        } else {
            panic!("Expected sequence");
        }
    }

    #[test]
    fn test_zero_copy_mapping() {
        let input = r#"{"key": "value"}"#;
        let mut composer = ZeroCopyComposer::new(input);
        let result = composer.compose_document().unwrap().unwrap();

        if let BorrowedValue::Mapping(map) = result {
            assert_eq!(map.len(), 1);
            let key = BorrowedValue::owned_string("key".to_string());
            assert!(map.contains_key(&key));
        } else {
            panic!("Expected mapping");
        }
    }

    // ---- iterative helpers (#16) ----

    #[test]
    fn borrowed_structure_depth_scalar_is_one() {
        assert_eq!(calculate_borrowed_structure_depth(&BorrowedValue::Null), 1);
        assert_eq!(
            calculate_borrowed_structure_depth(&BorrowedValue::Int(7)),
            1
        );
        assert_eq!(
            calculate_borrowed_structure_depth(&BorrowedValue::owned_string("x".into())),
            1
        );
    }

    #[test]
    fn borrowed_structure_depth_empty_collections_is_one() {
        assert_eq!(
            calculate_borrowed_structure_depth(&BorrowedValue::Sequence(Vec::new())),
            1
        );
        assert_eq!(
            calculate_borrowed_structure_depth(&BorrowedValue::Mapping(IndexMap::new())),
            1
        );
    }

    #[test]
    fn borrowed_structure_depth_nested_sequence() {
        // [[[1]]] → depth 4 (outer seq + 2 nested seqs + scalar)
        let inner = BorrowedValue::Sequence(vec![BorrowedValue::Int(1)]);
        let mid = BorrowedValue::Sequence(vec![inner]);
        let outer = BorrowedValue::Sequence(vec![mid]);
        assert_eq!(calculate_borrowed_structure_depth(&outer), 4);
    }

    #[test]
    fn borrowed_structure_depth_nested_mapping() {
        // { k: { k: 1 } } → depth 3
        let mut inner = IndexMap::new();
        inner.insert(
            BorrowedValue::owned_string("k".into()),
            BorrowedValue::Int(1),
        );
        let mut outer = IndexMap::new();
        outer.insert(
            BorrowedValue::owned_string("k".into()),
            BorrowedValue::Mapping(inner),
        );
        assert_eq!(
            calculate_borrowed_structure_depth(&BorrowedValue::Mapping(outer)),
            3
        );
    }

    #[test]
    fn borrowed_complexity_scalar_is_one() {
        assert_eq!(calculate_borrowed_complexity(&BorrowedValue::Null), 1);
        assert_eq!(calculate_borrowed_complexity(&BorrowedValue::Bool(true)), 1);
        assert_eq!(calculate_borrowed_complexity(&BorrowedValue::Int(42)), 1);
        assert_eq!(
            calculate_borrowed_complexity(&BorrowedValue::owned_string("hi".into())),
            1
        );
    }

    #[test]
    fn borrowed_complexity_sequence_charges_len_plus_one_plus_children() {
        // Sequence with 3 scalar children: 1 (seq) + 3 (len) + 3*1 (children) = 7
        let seq = BorrowedValue::Sequence(vec![
            BorrowedValue::Int(1),
            BorrowedValue::Int(2),
            BorrowedValue::Int(3),
        ]);
        assert_eq!(calculate_borrowed_complexity(&seq), 7);
    }

    #[test]
    fn borrowed_complexity_mapping_charges_two_per_entry_plus_one_plus_children() {
        // Mapping with 2 entries (k1: v1, k2: v2): 1 + 2*2 + 4*1 = 9
        let mut map = IndexMap::new();
        map.insert(
            BorrowedValue::owned_string("k1".into()),
            BorrowedValue::Int(1),
        );
        map.insert(
            BorrowedValue::owned_string("k2".into()),
            BorrowedValue::Int(2),
        );
        assert_eq!(
            calculate_borrowed_complexity(&BorrowedValue::Mapping(map)),
            9
        );
    }

    #[test]
    fn borrowed_structure_depth_is_stack_safe_for_deep_input() {
        // 100k-deep sequence would blow the stack if the helper were recursive.
        let mut v = BorrowedValue::Int(0);
        for _ in 0..100_000 {
            v = BorrowedValue::Sequence(vec![v]);
        }
        // We don't care about the exact value, only that it doesn't overflow.
        let depth = calculate_borrowed_structure_depth(&v);
        assert!(depth >= 100_000);
        // Iterative drop to avoid the recursive Drop in Vec/IndexMap.
        drop_borrowed_iteratively(v);
    }

    #[test]
    fn borrowed_complexity_is_stack_safe_for_deep_input() {
        let mut v = BorrowedValue::Int(0);
        for _ in 0..100_000 {
            v = BorrowedValue::Sequence(vec![v]);
        }
        let _ = calculate_borrowed_complexity(&v);
        drop_borrowed_iteratively(v);
    }

    // Drops a deeply nested BorrowedValue without recursing through
    // Vec/IndexMap's own Drop. Used only by the stack-safety tests above.
    fn drop_borrowed_iteratively(root: BorrowedValue<'_>) {
        let mut stack: Vec<BorrowedValue<'_>> = vec![root];
        while let Some(node) = stack.pop() {
            match node {
                BorrowedValue::Sequence(seq) => stack.extend(seq),
                BorrowedValue::Mapping(map) => {
                    for (k, v) in map {
                        stack.push(k);
                        stack.push(v);
                    }
                }
                _ => {}
            }
        }
    }

    // ---- alias materialization cap (#15) on ZeroCopyComposer ----

    fn permissive_with_alias_cap(cap: usize) -> Limits {
        Limits {
            max_total_alias_nodes: cap,
            ..Limits::permissive()
        }
    }

    #[test]
    fn zero_copy_alias_materialization_cap_fires() {
        // Anchor expands to a 10-element sequence; 20 aliased siblings.
        // Each *a materialization charges ~11 nodes, so cap=50 must reject.
        let input = r#"
a: &a [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
b: [*a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a, *a]
"#;
        let mut composer = ZeroCopyComposer::with_limits(input, permissive_with_alias_cap(50));
        let err = composer.compose_document().expect_err("cap should reject");
        let msg = err.to_string();
        assert!(
            msg.contains("alias") || msg.contains("materializ") || msg.contains("limit"),
            "expected materialization-cap error, got: {msg}"
        );
    }

    #[test]
    fn zero_copy_alias_below_cap_succeeds() {
        // Same shape but a generous cap → must succeed.
        let input = r#"
a: &a [1, 2, 3]
b: [*a, *a]
"#;
        let mut composer = ZeroCopyComposer::with_limits(input, permissive_with_alias_cap(10_000));
        let result = composer.compose_document().unwrap().unwrap();
        // sanity: returns a mapping
        assert!(matches!(result, BorrowedValue::Mapping(_)));
    }
}
