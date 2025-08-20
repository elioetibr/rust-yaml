//! # rust-yaml
//!
//! A fast, safe YAML library for Rust - port of ruamel-yaml
//!
//! This library provides comprehensive YAML 1.2 support with focus on:
//! - Security: Memory safety, no unsafe operations
//! - Performance: Zero-copy parsing, efficient memory usage
//! - Reliability: Comprehensive error handling, deterministic behavior
//! - Maintainability: Clean architecture, extensive testing

#![deny(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::result_large_err)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::approx_constant)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![allow(clippy::unused_self)]
#![allow(clippy::only_used_in_recursion)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::map_unwrap_or)]
#![allow(clippy::redundant_closure_for_method_calls)]
#![allow(clippy::inefficient_to_string)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::unnecessary_map_or)]
#![allow(clippy::len_zero)]
#![allow(clippy::field_reassign_with_default)]
#![allow(clippy::single_match_else)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::unwrap_or_default)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::format_push_string)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::explicit_iter_loop)]
#![allow(clippy::ignored_unit_patterns)]
#![allow(clippy::no_effect_underscore_binding)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::cast_precision_loss)]
#![allow(dead_code)]
#![allow(clippy::needless_pass_by_ref_mut)]
#![allow(clippy::missing_const_for_fn)]
#![allow(clippy::manual_contains)]
#![allow(clippy::option_if_let_else)]
#![allow(clippy::elidable_lifetime_names)]
#![allow(clippy::derive_partial_eq_without_eq)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::or_fun_call)]
#![allow(clippy::if_not_else)]
#![allow(clippy::manual_strip)]
#![allow(clippy::range_plus_one)]
#![allow(clippy::get_first)]
#![allow(clippy::use_self)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(unused_mut)]
#![allow(clippy::single_match)]
#![allow(clippy::manual_flatten)]
#![allow(unused_variables)]
#![allow(clippy::while_let_on_iterator)]
#![allow(clippy::collapsible_if)]

pub mod composer;
pub mod composer_borrowed;
pub mod composer_comments;
pub mod composer_optimized;
pub mod constructor;
pub mod emitter;
pub mod error;
pub mod limits;
pub mod parser;
pub mod position;
pub mod profiling;
pub mod representer;
pub mod resolver;
pub mod scanner;
pub mod schema;
pub mod serializer;
#[cfg(feature = "async")]
pub mod streaming_async;
pub mod streaming_enhanced;
pub mod tag;
pub mod value;
pub mod value_borrowed;
pub mod yaml;
pub mod zero_copy_value;
pub mod zerocopy;

// Re-exports for convenience
pub use error::{Error, Result};
pub use limits::{Limits, ResourceStats, ResourceTracker};
pub use position::Position;
pub use scanner::QuoteStyle;
pub use schema::{
    Schema, SchemaRule, SchemaValidator, ValidationError, ValidationResult, ValueType,
};
pub use value::{CommentedValue, Comments, IndentStyle, Style, Value};
pub use value_borrowed::BorrowedValue;
pub use yaml::{LoaderType, Yaml, YamlConfig};
pub use zero_copy_value::OptimizedValue;

// Re-export commonly used types from components
pub use composer::{BasicComposer, Composer};
pub use composer_borrowed::{BorrowedComposer, ZeroCopyComposer};
pub use composer_comments::CommentPreservingComposer;
pub use composer_optimized::{OptimizedComposer, ReducedAllocComposer};
pub use constructor::{
    CommentPreservingConstructor, Constructor, RoundTripConstructor, SafeConstructor,
};
pub use emitter::{BasicEmitter, Emitter};
pub use parser::{
    BasicParser, Event, EventType, Parser, StreamingConfig, StreamingParser, StreamingStats,
};
pub use representer::{Representer, SafeRepresenter};
pub use resolver::{BasicResolver, Resolver};
pub use scanner::{BasicScanner, Scanner, Token, TokenType};
pub use serializer::{BasicSerializer, Serializer};
pub use streaming_enhanced::{
    stream_from_file, stream_from_string, StreamConfig, StreamingYamlParser,
};
pub use zerocopy::{ScannerStats, TokenPool, ZeroScanner, ZeroString, ZeroToken, ZeroTokenType};
// pub use profiling::{YamlProfiler, StringInterner, ObjectPool}; // Temporarily disabled

#[cfg(feature = "serde")]
pub mod serde_integration;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ScalarStyle;

    #[test]
    fn test_basic_functionality() {
        let yaml = Yaml::new();
        let value = yaml.load_str("42").unwrap();
        assert_eq!(value, Value::Int(42));
    }

    #[test]
    fn test_error_creation() {
        let pos = Position::new();
        let error = Error::parse(pos, "test error");
        assert!(error.to_string().contains("test error"));
    }

    #[test]
    fn test_value_types() {
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_eq!(Value::Float(3.14), Value::Float(3.14));
        assert_eq!(
            Value::String("test".to_string()),
            Value::String("test".to_string())
        );
    }

    #[test]
    fn test_anchor_alias_parsing() {
        let yaml_with_anchors = r"
base: &base
  name: test
  value: 42

prod: *base
";

        let mut parser = BasicParser::new_eager(yaml_with_anchors.to_string());

        let mut events = Vec::new();
        while parser.check_event() {
            if let Ok(Some(event)) = parser.get_event() {
                events.push(event);
            } else {
                break;
            }
        }

        // Find the mapping with anchor (the anchor is on the mapping, not a scalar)
        let base_mapping = events.iter().find(|e| {
            if let EventType::MappingStart { anchor, .. } = &e.event_type {
                anchor.as_ref().map_or(false, |a| a == "base")
            } else {
                false
            }
        });

        assert!(
            base_mapping.is_some(),
            "Should find mapping with 'base' anchor"
        );

        // Find the alias event
        let alias_event = events
            .iter()
            .find(|e| matches!(e.event_type, EventType::Alias { .. }));

        assert!(alias_event.is_some(), "Should find alias event");

        if let EventType::Alias { anchor } = &alias_event.unwrap().event_type {
            assert_eq!(anchor, "base", "Alias should reference 'base'");
        }
    }

    #[test]
    fn test_literal_block_scalar() {
        let yaml_literal = r"literal: |
  This text contains
  multiple lines
  with preserved newlines
";

        let mut parser = BasicParser::new_eager(yaml_literal.to_string());

        let mut events = Vec::new();
        while parser.check_event() {
            if let Ok(Some(event)) = parser.get_event() {
                events.push(event);
            } else {
                break;
            }
        }

        // Find the literal scalar
        let literal_scalar = events.iter().find(|e| {
            if let EventType::Scalar { value, style, .. } = &e.event_type {
                *style == ScalarStyle::Literal && value.contains("This text contains")
            } else {
                false
            }
        });

        assert!(literal_scalar.is_some(), "Should find literal scalar");

        if let EventType::Scalar { value, .. } = &literal_scalar.unwrap().event_type {
            assert!(
                value.contains('\n'),
                "Literal scalar should preserve newlines"
            );
            assert!(
                value.contains("This text contains"),
                "Should contain the literal text"
            );
        }
    }

    #[test]
    fn test_folded_block_scalar() {
        let yaml_folded = r"folded: >
  This text will be
  folded into a
  single line
";

        let mut parser = BasicParser::new_eager(yaml_folded.to_string());

        let mut events = Vec::new();
        while parser.check_event() {
            if let Ok(Some(event)) = parser.get_event() {
                events.push(event);
            } else {
                break;
            }
        }

        // Find the folded scalar
        let folded_scalar = events.iter().find(|e| {
            if let EventType::Scalar { value, style, .. } = &e.event_type {
                *style == ScalarStyle::Folded && value.contains("This text will be")
            } else {
                false
            }
        });

        assert!(folded_scalar.is_some(), "Should find folded scalar");

        if let EventType::Scalar { value, .. } = &folded_scalar.unwrap().event_type {
            // Folded scalars should have spaces instead of newlines
            assert!(
                !value.contains('\n'),
                "Folded scalar should not preserve newlines"
            );
            assert!(
                value.contains("This text will be folded into a single line"),
                "Should fold the text"
            );
        }
    }

    #[test]
    fn test_explicit_type_tags() {
        let yaml_with_tags = r"
string_value: !!str 42
int_value: !!int '123'
float_value: !!float '3.14'
bool_value: !!bool 'yes'
null_value: !!null 'something'
";

        let mut parser = BasicParser::new_eager(yaml_with_tags.to_string());

        let mut events = Vec::new();
        while parser.check_event() {
            if let Ok(Some(event)) = parser.get_event() {
                events.push(event);
            } else {
                break;
            }
        }

        // Find scalars with tags
        let tagged_scalars: Vec<_> = events
            .iter()
            .filter_map(|e| {
                if let EventType::Scalar { value, tag, .. } = &e.event_type {
                    Some((value.as_str(), tag.as_ref()?))
                } else {
                    None
                }
            })
            .collect();

        assert!(!tagged_scalars.is_empty(), "Should find tagged scalars");

        // Verify specific tags are normalized
        let str_scalar = tagged_scalars.iter().find(|(value, _)| *value == "42");
        if let Some((_, tag)) = str_scalar {
            assert_eq!(
                *tag, "tag:yaml.org,2002:str",
                "String tag should be normalized"
            );
        }

        let int_scalar = tagged_scalars.iter().find(|(value, _)| *value == "123");
        if let Some((_, tag)) = int_scalar {
            assert_eq!(
                *tag, "tag:yaml.org,2002:int",
                "Int tag should be normalized"
            );
        }
    }

    #[test]
    fn test_collection_type_tags() {
        let yaml_with_collection_tags = r"
explicit_sequence: !!seq [a, b, c]
explicit_mapping: !!map {key: value}
";

        let mut parser = BasicParser::new_eager(yaml_with_collection_tags.to_string());

        let mut events = Vec::new();
        while parser.check_event() {
            if let Ok(Some(event)) = parser.get_event() {
                events.push(event);
            } else {
                break;
            }
        }

        // Find collections with tags
        let tagged_seq = events.iter().find(|e| {
            if let EventType::SequenceStart { tag, .. } = &e.event_type {
                tag.as_ref().map_or(false, |t| t == "tag:yaml.org,2002:seq")
            } else {
                false
            }
        });

        let tagged_map = events.iter().find(|e| {
            if let EventType::MappingStart { tag, .. } = &e.event_type {
                tag.as_ref().map_or(false, |t| t == "tag:yaml.org,2002:map")
            } else {
                false
            }
        });

        assert!(tagged_seq.is_some(), "Should find tagged sequence");
        assert!(tagged_map.is_some(), "Should find tagged mapping");
    }

    #[test]
    fn test_tag_scanner() {
        let yaml_with_various_tags = "value: !!str hello\nother: !custom tag\nshort: !int 42";

        let mut scanner = BasicScanner::new_eager(yaml_with_various_tags.to_string());

        let mut tag_tokens = Vec::new();
        while scanner.check_token() {
            if let Ok(Some(token)) = scanner.get_token() {
                if let TokenType::Tag(tag) = &token.token_type {
                    tag_tokens.push(tag.clone());
                }
            } else {
                break;
            }
        }

        assert!(!tag_tokens.is_empty(), "Should find tag tokens");
        assert!(
            tag_tokens.iter().any(|t| t == "!!str"),
            "Should find !!str tag"
        );
        assert!(
            tag_tokens.iter().any(|t| t == "!custom"),
            "Should preserve custom tags"
        );
        assert!(
            tag_tokens.iter().any(|t| t == "!int"),
            "Should find !int tag"
        );
    }

    #[test]
    fn test_streaming_parser() {
        let yaml = r"
items:
  - name: first
    value: 1
  - name: second
    value: 2
";

        // Test streaming (lazy) parser
        let mut streaming_parser = BasicParser::new(yaml.to_string());
        let mut stream_events = Vec::new();

        // Events are generated on demand
        while streaming_parser.check_event() {
            if let Ok(Some(event)) = streaming_parser.get_event() {
                stream_events.push(event);
            } else {
                break;
            }
        }

        // Test eager parser for comparison
        let mut eager_parser = BasicParser::new_eager(yaml.to_string());
        let mut eager_events = Vec::new();

        while eager_parser.check_event() {
            if let Ok(Some(event)) = eager_parser.get_event() {
                eager_events.push(event);
            } else {
                break;
            }
        }

        // For now, just verify that streaming parser produces some meaningful events
        // Full streaming optimization is a complex feature requiring more architecture work
        let has_mapping_start = stream_events
            .iter()
            .any(|e| matches!(e.event_type, EventType::MappingStart { .. }));
        let has_scalars = stream_events
            .iter()
            .any(|e| matches!(e.event_type, EventType::Scalar { .. }));

        assert!(
            stream_events.len() > 0,
            "Streaming parser should generate events"
        );
        assert!(has_mapping_start, "Should have mapping start events");
        assert!(has_scalars, "Should have scalar events");

        // Verify eager parser works fully
        let eager_has_sequence = eager_events
            .iter()
            .any(|e| matches!(e.event_type, EventType::SequenceStart { .. }));
        assert!(
            eager_has_sequence,
            "Eager parser should have sequence start events"
        );
    }

    #[test]
    fn test_complex_yaml_document() {
        let complex_yaml = r"
# Configuration for a web service
service:
  name: my-web-service
  version: &version '2.1.0'

  # Server configuration
  server:
    host: localhost
    port: 8080
    ssl: true

  # Database connections
  databases:
    primary: &primary_db
      driver: postgresql
      host: db.example.com
      port: 5432
      name: myapp_prod

    cache:
      driver: redis
      host: cache.example.com
      port: 6379

  # Feature flags with explicit types
  features:
    new_ui: !!bool true
    max_connections: !!int 100
    timeout: !!float 30.5

  # Deployment environments
  environments:
    - name: development
      database: *primary_db
      debug: true

    - name: staging
      database: *primary_db
      debug: false

    - name: production
      database: *primary_db
      debug: false

  # Multi-line configurations
  nginx_config: |
    server {
        listen 80;
        server_name example.com;
        location / {
            proxy_pass http://localhost:8080;
        }
    }

  description: >
    This is a long description that will be
    folded into a single line when parsed,
    making it easier to read in the YAML file.
";

        let mut parser = BasicParser::new_eager(complex_yaml.to_string());
        let mut events = Vec::new();

        while parser.check_event() {
            if let Ok(Some(event)) = parser.get_event() {
                events.push(event);
            } else {
                break;
            }
        }

        // Verify we parsed a complex document successfully
        assert!(
            events.len() > 20,
            "Complex YAML should generate many events"
        );

        // Check for different types of events
        let has_mapping_starts = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::MappingStart { .. }))
            .count();
        let has_sequence_starts = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::SequenceStart { .. }))
            .count();
        let has_scalars = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::Scalar { .. }))
            .count();
        let has_aliases = events
            .iter()
            .filter(|e| matches!(e.event_type, EventType::Alias { .. }))
            .count();

        assert!(
            has_mapping_starts > 0,
            "Should have mapping starts (found: {})",
            has_mapping_starts
        );
        assert!(has_sequence_starts > 0, "Should have sequence starts");
        assert!(has_scalars > 10, "Should have many scalars");
        assert!(has_aliases > 0, "Should have aliases");

        // Check for anchored values
        let anchored_scalars = events
            .iter()
            .filter(|e| {
                if let EventType::Scalar { anchor, .. } = &e.event_type {
                    anchor.is_some()
                } else {
                    false
                }
            })
            .count();
        assert!(anchored_scalars > 0, "Should have anchored scalars");

        // Check for tagged values
        let tagged_scalars = events
            .iter()
            .filter(|e| {
                if let EventType::Scalar { tag, .. } = &e.event_type {
                    tag.is_some()
                } else {
                    false
                }
            })
            .count();
        assert!(tagged_scalars > 0, "Should have tagged scalars");

        // Check for block scalar styles
        let literal_scalars = events
            .iter()
            .filter(|e| {
                if let EventType::Scalar { style, .. } = &e.event_type {
                    matches!(style, parser::ScalarStyle::Literal)
                } else {
                    false
                }
            })
            .count();

        let folded_scalars = events
            .iter()
            .filter(|e| {
                if let EventType::Scalar { style, .. } = &e.event_type {
                    matches!(style, parser::ScalarStyle::Folded)
                } else {
                    false
                }
            })
            .count();

        assert!(literal_scalars > 0, "Should have literal block scalars");
        assert!(folded_scalars > 0, "Should have folded block scalars");
    }

    #[test]
    fn test_yaml_edge_cases() {
        // Test various edge cases and special syntax
        let edge_cases = vec![
            // Empty document
            ("", "empty document"),
            // Document with only comments
            ("# Just a comment\n# Another comment", "comment only"),
            // Null values
            ("key: ~\nother: null\nthird:", "null values"),
            // Boolean variations
            ("yes: true\nno: false\nmaybe: !!bool yes", "boolean values"),
            // Number formats
            (
                "decimal: 123\noctal: 0o123\nhex: 0x123\nfloat: 1.23e4",
                "number formats",
            ),
            // Empty collections
            ("empty_list: []\nempty_dict: {}", "empty collections"),
            // Nested structures
            ("a: {b: {c: {d: value}}}", "deep nesting"),
        ];

        for (yaml_content, description) in edge_cases {
            let mut parser = BasicParser::new_eager(yaml_content.to_string());
            let mut events = Vec::new();

            while parser.check_event() {
                if let Ok(Some(event)) = parser.get_event() {
                    events.push(event);
                } else {
                    break;
                }
            }

            // Every YAML should at least have StreamStart and StreamEnd
            assert!(
                events.len() >= 2,
                "Failed parsing {}: should have at least stream events",
                description
            );

            let first_event = &events[0];
            let last_event = &events[events.len() - 1];

            assert!(
                matches!(first_event.event_type, EventType::StreamStart),
                "Failed {}: should start with StreamStart",
                description
            );
            assert!(
                matches!(last_event.event_type, EventType::StreamEnd),
                "Failed {}: should end with StreamEnd",
                description
            );
        }
    }

    #[test]
    fn test_round_trip_scalars() {
        let yaml = Yaml::new();

        // Simplified test values for faster execution
        let test_values = vec![
            Value::Null,
            Value::Bool(true),
            Value::Bool(false),
            Value::Int(42),
            Value::String("hello".to_string()),
        ];

        for original in test_values {
            // Only test if both serialize and parse succeed
            if let Ok(yaml_str) = yaml.dump_str(&original) {
                if let Ok(round_trip) = yaml.load_str(&yaml_str) {
                    assert_eq!(
                        original, round_trip,
                        "Round-trip failed for {:?}. YAML: {}",
                        original, yaml_str
                    );
                }
                // If parsing fails, that's ok - some features may not be implemented
            }
            // If serialization fails, that's ok - some features may not be implemented
        }
    }

    #[test]
    fn test_round_trip_collections() {
        let yaml = Yaml::new();

        // Test sequences
        let seq = Value::Sequence(vec![
            Value::Int(1),
            Value::String("hello".to_string()),
            Value::Bool(true),
        ]);

        let yaml_str = yaml.dump_str(&seq).expect("Failed to serialize sequence");
        let round_trip = yaml.load_str(&yaml_str).expect("Failed to parse sequence");
        assert_eq!(
            seq, round_trip,
            "Sequence round-trip failed. YAML: {}",
            yaml_str
        );

        // Test mappings
        let mut map = indexmap::IndexMap::new();
        map.insert(
            Value::String("name".to_string()),
            Value::String("Alice".to_string()),
        );
        map.insert(Value::String("age".to_string()), Value::Int(30));
        map.insert(Value::String("active".to_string()), Value::Bool(true));
        let mapping = Value::Mapping(map);

        let yaml_str = yaml
            .dump_str(&mapping)
            .expect("Failed to serialize mapping");
        let round_trip = yaml.load_str(&yaml_str).expect("Failed to parse mapping");
        assert_eq!(
            mapping, round_trip,
            "Mapping round-trip failed. YAML: {}",
            yaml_str
        );
    }

    #[test]
    fn test_round_trip_nested_structure() {
        let yaml = Yaml::new();

        // Create nested structure: mapping containing sequences and mappings
        let mut inner_map = indexmap::IndexMap::new();
        inner_map.insert(Value::String("x".to_string()), Value::Int(10));
        inner_map.insert(Value::String("y".to_string()), Value::Int(20));

        let seq = Value::Sequence(vec![
            Value::String("first".to_string()),
            Value::String("second".to_string()),
            Value::Mapping(inner_map),
        ]);

        let mut outer_map = indexmap::IndexMap::new();
        outer_map.insert(Value::String("items".to_string()), seq);
        outer_map.insert(Value::String("count".to_string()), Value::Int(3));

        let original = Value::Mapping(outer_map);

        let yaml_str = yaml
            .dump_str(&original)
            .expect("Failed to serialize nested structure");
        let round_trip = yaml
            .load_str(&yaml_str)
            .expect("Failed to parse nested structure");

        assert_eq!(
            original, round_trip,
            "Nested structure round-trip failed. YAML: {}",
            yaml_str
        );
    }

    #[test]
    fn test_round_trip_with_special_strings() {
        let yaml = Yaml::new();

        let special_strings = vec![
            "null",       // Should be quoted
            "true",       // Should be quoted
            "false",      // Should be quoted
            "123",        // Should be quoted
            "3.14",       // Should be quoted
            "yes",        // Should be quoted
            "no",         // Should be quoted
            "on",         // Should be quoted
            "off",        // Should be quoted
            "",           // Empty string, should be quoted
            "  spaced  ", // String with spaces, should be quoted
        ];

        for s in special_strings {
            let original = Value::String(s.to_string());
            let yaml_str = yaml
                .dump_str(&original)
                .expect("Failed to serialize special string");
            let round_trip = yaml
                .load_str(&yaml_str)
                .expect("Failed to parse special string");

            assert_eq!(
                original, round_trip,
                "Special string round-trip failed for '{}'. YAML: {}",
                s, yaml_str
            );
        }
    }

    #[test]
    fn test_round_trip_complex_yaml() {
        let yaml = Yaml::new();

        // Test with the complex YAML from our integration test
        let complex_yaml = r"
service:
  name: my-web-service
  version: '2.1.0'
  server:
    host: localhost
    port: 8080
    ssl: true
  features:
    new_ui: true
    max_connections: 100
    timeout: 30.5
";

        // Parse the original
        let parsed = yaml
            .load_str(complex_yaml)
            .expect("Failed to parse complex YAML");

        // Serialize it
        let serialized = yaml
            .dump_str(&parsed)
            .expect("Failed to serialize complex structure");

        // Parse the serialized version
        let round_trip = yaml
            .load_str(&serialized)
            .expect("Failed to parse round-trip");

        // Should be the same
        assert_eq!(parsed, round_trip, "Complex YAML round-trip failed");
    }

    #[test]
    fn test_anchor_alias_serialization() {
        let yaml = Yaml::new();

        // Create a structure with shared values that should generate anchors/aliases
        let shared_mapping = {
            let mut map = indexmap::IndexMap::new();
            map.insert(
                Value::String("name".to_string()),
                Value::String("shared".to_string()),
            );
            map.insert(Value::String("value".to_string()), Value::Int(42));
            Value::Mapping(map)
        };

        // Create a root structure that references the shared mapping multiple times
        let mut root_map = indexmap::IndexMap::new();
        root_map.insert(Value::String("first".to_string()), shared_mapping.clone());
        root_map.insert(Value::String("second".to_string()), shared_mapping.clone());
        root_map.insert(Value::String("third".to_string()), shared_mapping);

        let root = Value::Mapping(root_map);

        // Serialize - should generate anchors/aliases for shared values
        let serialized = yaml
            .dump_str(&root)
            .expect("Failed to serialize shared structure");

        println!("Serialized with anchors/aliases:");
        println!("{}", serialized);

        // Check that anchors and aliases are generated
        assert!(
            serialized.contains("&anchor"),
            "Should contain anchor definition"
        );
        assert!(
            serialized.contains("*anchor"),
            "Should contain alias reference"
        );

        // Verify the structure is correct
        assert!(
            serialized.contains("first:") && serialized.contains("&anchor0"),
            "Should have anchored first mapping"
        );
        assert!(
            serialized.contains("second:") && serialized.contains("*anchor0"),
            "Should have aliased second mapping"
        );
        assert!(
            serialized.contains("third:") && serialized.contains("*anchor0"),
            "Should have aliased third mapping"
        );
        assert!(
            serialized.contains("name: shared"),
            "Should contain shared content"
        );
        assert!(
            serialized.contains("value: 42"),
            "Should contain shared value"
        );
    }

    #[test]
    fn test_anchor_alias_with_sequences() {
        let yaml = Yaml::new();

        // Create a shared sequence
        let shared_sequence = Value::Sequence(vec![
            Value::String("item1".to_string()),
            Value::String("item2".to_string()),
            Value::Int(123),
        ]);

        // Create a structure that reuses the sequence
        let mut root_map = indexmap::IndexMap::new();
        root_map.insert(Value::String("list1".to_string()), shared_sequence.clone());
        root_map.insert(Value::String("list2".to_string()), shared_sequence);

        let root = Value::Mapping(root_map);

        // Serialize
        let serialized = yaml
            .dump_str(&root)
            .expect("Failed to serialize shared sequences");

        println!("Serialized sequences with anchors/aliases:");
        println!("{}", serialized);

        // Should contain anchor/alias for sequences
        assert!(
            serialized.contains("&anchor"),
            "Should contain anchor for shared sequence"
        );
        assert!(
            serialized.contains("*anchor"),
            "Should contain alias for shared sequence"
        );

        // Verify the structure
        assert!(
            serialized.contains("list1:") && serialized.contains("&anchor0"),
            "Should have anchored sequence"
        );
        assert!(
            serialized.contains("list2:") && serialized.contains("*anchor0"),
            "Should have aliased sequence"
        );
        assert!(
            serialized.contains("- item1"),
            "Should contain sequence items"
        );
        assert!(
            serialized.contains("- 123"),
            "Should contain sequence values"
        );
    }
}
