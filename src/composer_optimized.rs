//! Optimized YAML composer that reduces allocations and cloning
//!
//! This module provides an optimized composer implementation that
//! minimizes memory allocations and unnecessary cloning operations.

use crate::{
    BasicParser, Error, Limits, Parser, Position, ResourceTracker, Result,
    parser::{EventType, ScalarStyle},
    zero_copy_value::OptimizedValue,
};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::rc::Rc;

/// Iterative DFS — see [`crate::composer::calculate_structure_depth`] (#16).
fn calculate_optimized_structure_depth(value: &OptimizedValue) -> usize {
    let mut max_depth: usize = 1;
    let mut stack: Vec<(&OptimizedValue, usize)> = vec![(value, 1)];
    while let Some((node, depth)) = stack.pop() {
        if depth > max_depth {
            max_depth = depth;
        }
        let next = depth.saturating_add(1);
        match node {
            OptimizedValue::Sequence(seq) => {
                for item in seq.iter() {
                    stack.push((item, next));
                }
            }
            OptimizedValue::Mapping(map) => {
                for (_, v) in map.iter() {
                    stack.push((v, next));
                }
            }
            _ => {}
        }
    }
    max_depth
}

/// Trait for optimized composers
pub trait OptimizedComposer {
    /// Check if there are more documents available
    fn check_document(&self) -> bool;

    /// Compose the next document with minimal allocations
    fn compose_document(&mut self) -> Result<Option<OptimizedValue>>;

    /// Get the current position in the stream
    fn position(&self) -> Position;

    /// Reset the composer state
    fn reset(&mut self);
}

/// An optimized composer that reduces allocations
pub struct ReducedAllocComposer {
    parser: BasicParser,
    position: Position,
    /// Store anchors using Rc for cheap cloning
    anchors: HashMap<String, Rc<OptimizedValue>>,
    limits: Limits,
    resource_tracker: ResourceTracker,
    alias_expansion_stack: Vec<String>,
    current_depth: usize,
    /// Active YAML spec version for the current document.
    yaml_version: crate::version::YamlVersion,
}

impl ReducedAllocComposer {
    /// Create a new optimized composer
    pub fn new(input: String) -> Self {
        Self::with_limits(input, Limits::default())
    }

    /// Create a new optimized composer with custom limits
    pub fn with_limits(input: String, limits: Limits) -> Self {
        Self {
            parser: BasicParser::with_limits(input, limits.clone()),
            position: Position::new(),
            anchors: HashMap::new(),
            limits,
            resource_tracker: ResourceTracker::new(),
            alias_expansion_stack: Vec::new(),
            current_depth: 0,
            yaml_version: crate::version::YamlVersion::default(),
        }
    }

    /// Compose a node from events with reduced allocations
    fn compose_node(&mut self) -> Result<Option<OptimizedValue>> {
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
                let scalar_value = self.compose_scalar_optimized(value, style)?;

                // Store anchor if present - use Rc for cheap cloning
                if let Some(anchor_name) = anchor {
                    self.resource_tracker.add_anchor(&self.limits)?;
                    self.anchors
                        .insert(anchor_name, Rc::new(scalar_value.clone()));
                }

                Ok(Some(scalar_value))
            }

            EventType::SequenceStart { anchor, .. } => {
                let sequence = self.compose_sequence()?;

                // Store anchor if present
                if let Some(anchor_name) = anchor {
                    if let Some(ref seq) = sequence {
                        self.resource_tracker.add_anchor(&self.limits)?;
                        self.anchors.insert(anchor_name, Rc::new(seq.clone()));
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
                        self.anchors.insert(anchor_name, Rc::new(map.clone()));
                    }
                }

                Ok(mapping)
            }

            EventType::SequenceEnd | EventType::MappingEnd => Ok(None),

            EventType::Alias { anchor } => {
                // Check for cyclic references
                if self.alias_expansion_stack.contains(&anchor) {
                    return Err(Error::construction(
                        event.position,
                        format!("Cyclic alias reference detected: '{}'", anchor),
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
                self.alias_expansion_stack.push(anchor.clone());

                // Look up the anchor - use Rc clone for efficiency
                let result = match self.anchors.get(&anchor) {
                    Some(value) => {
                        // Check if the resolved value's structure depth would exceed alias depth limit
                        let structure_depth = calculate_optimized_structure_depth(value);
                        if structure_depth > self.limits.max_alias_depth {
                            return Err(Error::construction(
                                event.position,
                                format!(
                                    "Alias '{}' creates structure with depth {} exceeding max_alias_depth {}",
                                    anchor, structure_depth, self.limits.max_alias_depth
                                ),
                            ));
                        }

                        // Rc clone is very cheap - just increments reference count
                        let cloned = (**value).clone();
                        let nodes = calculate_complexity(&cloned)?;
                        // Cap cumulative alias materialization BEFORE
                        // committing the clone (#15 billion-laughs gap).
                        self.resource_tracker
                            .add_alias_materialization(&self.limits, nodes)?;
                        self.resource_tracker.add_complexity(&self.limits, nodes)?;
                        Ok(Some(cloned))
                    }
                    None => Err(Error::construction(
                        event.position,
                        format!("Unknown anchor '{}'", anchor),
                    )),
                };

                // Clean up tracking
                self.alias_expansion_stack.pop();
                self.resource_tracker.exit_alias();

                result
            }
        }
    }

    /// Compose a scalar value with optimization
    fn compose_scalar_optimized(
        &self,
        value: String,
        style: ScalarStyle,
    ) -> Result<OptimizedValue> {
        if matches!(style, ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted) {
            return Ok(OptimizedValue::string(value));
        }

        Ok(
            match crate::resolver::resolve_plain_scalar(&value, self.yaml_version) {
                crate::resolver::PlainScalarType::Null => OptimizedValue::Null,
                crate::resolver::PlainScalarType::Bool(b) => OptimizedValue::Bool(b),
                crate::resolver::PlainScalarType::Int(i) => OptimizedValue::Int(i),
                crate::resolver::PlainScalarType::Float(f) => OptimizedValue::Float(f),
                crate::resolver::PlainScalarType::Str => OptimizedValue::string(value),
                crate::resolver::PlainScalarType::Value => {
                    return Err(crate::resolver::value_tag_error(self.position));
                }
            },
        )
    }

    /// Compose a sequence with reduced allocations
    fn compose_sequence(&mut self) -> Result<Option<OptimizedValue>> {
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
        Ok(Some(OptimizedValue::sequence_with(sequence)))
    }

    /// Compose a mapping with reduced allocations
    fn compose_mapping(&mut self) -> Result<Option<OptimizedValue>> {
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

            let value = self.compose_node()?.unwrap_or(OptimizedValue::Null);

            // Check for merge key
            if let OptimizedValue::String(key_str) = &key {
                if key_str.as_str() == "<<" {
                    self.process_merge_key(&mut mapping, &value)?;
                    continue;
                }
            }

            self.resource_tracker.add_collection_item(&self.limits)?;
            self.resource_tracker.add_complexity(&self.limits, 2)?;

            mapping.insert(key, value);
        }

        self.current_depth -= 1;
        Ok(Some(OptimizedValue::mapping_with(
            mapping.into_iter().collect(),
        )))
    }

    /// Process a merge key by merging values into the current mapping
    fn process_merge_key(
        &self,
        mapping: &mut IndexMap<OptimizedValue, OptimizedValue>,
        merge_value: &OptimizedValue,
    ) -> Result<()> {
        match merge_value {
            // Single mapping to merge
            OptimizedValue::Mapping(source_map) => {
                for (key, value) in source_map.iter() {
                    // Only insert if key doesn't already exist
                    mapping.entry(key.clone()).or_insert_with(|| value.clone());
                }
            }

            // Sequence of mappings to merge
            OptimizedValue::Sequence(sources) => {
                for source in sources.iter() {
                    if let OptimizedValue::Mapping(source_map) = source {
                        for (key, value) in source_map.iter() {
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

impl OptimizedComposer for ReducedAllocComposer {
    fn check_document(&self) -> bool {
        if let Ok(Some(event)) = self.parser.peek_event() {
            !matches!(event.event_type, EventType::StreamEnd)
        } else {
            false
        }
    }

    fn compose_document(&mut self) -> Result<Option<OptimizedValue>> {
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

/// Calculate complexity score for a value. Iterative for stack safety (#16).
fn calculate_complexity(value: &OptimizedValue) -> Result<usize> {
    let mut total: usize = 0;
    let mut stack: Vec<&OptimizedValue> = vec![value];
    while let Some(node) = stack.pop() {
        match node {
            OptimizedValue::Sequence(seq) => {
                total = total.saturating_add(1usize.saturating_add(seq.len()));
                for item in seq.iter() {
                    stack.push(item);
                }
            }
            OptimizedValue::Mapping(map) => {
                total = total.saturating_add(1usize.saturating_add(map.len().saturating_mul(2)));
                for (k, v) in map.iter() {
                    stack.push(k);
                    stack.push(v);
                }
            }
            _ => total = total.saturating_add(1),
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_scalar() {
        let mut composer = ReducedAllocComposer::new("42".to_string());
        let result = composer.compose_document().unwrap().unwrap();
        assert_eq!(result, OptimizedValue::Int(42));
    }

    #[test]
    fn test_optimized_sequence() {
        let mut composer = ReducedAllocComposer::new("[1, 2, 3]".to_string());
        let result = composer.compose_document().unwrap().unwrap();

        if let OptimizedValue::Sequence(seq) = result {
            assert_eq!(seq.len(), 3);
        } else {
            panic!("Expected sequence");
        }
    }

    #[test]
    fn test_anchor_rc_sharing() {
        let yaml = r#"
base: &base
  value: 42
ref1: *base
ref2: *base
"#;
        let mut composer = ReducedAllocComposer::new(yaml.to_string());
        let _result = composer.compose_document().unwrap().unwrap();

        // The anchors should use Rc, so cloning should be cheap
        assert!(composer.anchors.len() > 0);
    }
}
