#![allow(clippy::approx_constant)]
#![allow(clippy::needless_raw_string_hashes)]

use rust_yaml::{LoaderType, Value, Yaml, YamlConfig};

#[test]
fn test_basic_scalar_parsing() {
    let yaml = Yaml::new();

    // Test null
    let result = yaml.load_str("null").unwrap();
    assert_eq!(result, Value::Null);

    // Test boolean
    let result = yaml.load_str("true").unwrap();
    assert_eq!(result, Value::Bool(true));

    let result = yaml.load_str("false").unwrap();
    assert_eq!(result, Value::Bool(false));

    // Test integer
    let result = yaml.load_str("42").unwrap();
    assert_eq!(result, Value::Int(42));

    // Test float
    let result = yaml.load_str("3.14").unwrap();
    assert_eq!(result, Value::Float(3.14));

    // Test string
    let result = yaml.load_str("hello world").unwrap();
    assert_eq!(result, Value::String("hello world".to_string()));
}

#[test]
fn test_flow_sequence_parsing() {
    let yaml = Yaml::new();

    let result = yaml.load_str("[1, 2, 3]").unwrap();
    let expected = Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
    assert_eq!(result, expected);
}

#[test]
fn test_flow_mapping_parsing() {
    let yaml = Yaml::new();

    let result = yaml.load_str(r#"{"key": "value", "number": 42}"#).unwrap();

    let mut expected_map = indexmap::IndexMap::new();
    expected_map.insert(
        Value::String("key".to_string()),
        Value::String("value".to_string()),
    );
    expected_map.insert(Value::String("number".to_string()), Value::Int(42));
    let expected = Value::Mapping(expected_map);

    assert_eq!(result, expected);
}

#[test]
fn test_block_sequence_parsing() {
    let yaml = Yaml::new();

    let yaml_content = r#"
- item1
- item2
- item3
"#;

    let result = yaml.load_str(yaml_content).unwrap();
    let expected = Value::Sequence(vec![
        Value::String("item1".to_string()),
        Value::String("item2".to_string()),
        Value::String("item3".to_string()),
    ]);
    assert_eq!(result, expected);
}

#[test]
fn test_block_mapping_parsing() {
    let yaml = Yaml::new();

    let yaml_content = r#"
key1: value1
key2: value2
key3: 42
"#;

    let result = yaml.load_str(yaml_content).unwrap();

    let mut expected_map = indexmap::IndexMap::new();
    expected_map.insert(
        Value::String("key1".to_string()),
        Value::String("value1".to_string()),
    );
    expected_map.insert(
        Value::String("key2".to_string()),
        Value::String("value2".to_string()),
    );
    expected_map.insert(Value::String("key3".to_string()), Value::Int(42));
    let expected = Value::Mapping(expected_map);

    assert_eq!(result, expected);
}

#[test]
fn test_nested_structure_parsing() {
    let yaml = Yaml::new();

    let yaml_content = r#"
users:
  - name: Alice
    age: 30
  - name: Bob
    age: 25
config:
  debug: true
  port: 8080
"#;

    let result = yaml.load_str(yaml_content).unwrap();

    // Build expected structure
    let mut user1 = indexmap::IndexMap::new();
    user1.insert(
        Value::String("name".to_string()),
        Value::String("Alice".to_string()),
    );
    user1.insert(Value::String("age".to_string()), Value::Int(30));

    let mut user2 = indexmap::IndexMap::new();
    user2.insert(
        Value::String("name".to_string()),
        Value::String("Bob".to_string()),
    );
    user2.insert(Value::String("age".to_string()), Value::Int(25));

    let users_sequence = Value::Sequence(vec![Value::Mapping(user1), Value::Mapping(user2)]);

    let mut config = indexmap::IndexMap::new();
    config.insert(Value::String("debug".to_string()), Value::Bool(true));
    config.insert(Value::String("port".to_string()), Value::Int(8080));

    let mut expected_map = indexmap::IndexMap::new();
    expected_map.insert(Value::String("users".to_string()), users_sequence);
    expected_map.insert(Value::String("config".to_string()), Value::Mapping(config));
    let expected = Value::Mapping(expected_map);

    assert_eq!(result, expected);
}

#[test]
fn test_multi_document_parsing() {
    let yaml = Yaml::new();

    let yaml_content = r#"
document: 1
data: [1, 2, 3]
---
document: 2
data: [4, 5, 6]
---
document: 3
data: [7, 8, 9]
"#;

    let documents = yaml.load_all_str(yaml_content).unwrap();
    assert_eq!(documents.len(), 3);

    // Check first document
    if let Value::Mapping(ref map) = documents[0] {
        assert_eq!(
            map.get(&Value::String("document".to_string())),
            Some(&Value::Int(1))
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_dump_basic_values() {
    let yaml = Yaml::new();

    // Test scalar dumping
    let output = yaml.dump_str(&Value::Int(42)).unwrap();
    assert_eq!(output.trim(), "42");

    let output = yaml.dump_str(&Value::String("hello".to_string())).unwrap();
    assert_eq!(output.trim(), "hello");

    let output = yaml.dump_str(&Value::Bool(true)).unwrap();
    assert_eq!(output.trim(), "true");
}

#[test]
fn test_roundtrip() {
    let yaml = Yaml::new();

    let original_yaml = r#"
name: rust-yaml
version: 0.1.0
features:
  - fast
  - safe
  - reliable
config:
  debug: true
  max_depth: 100
"#;

    // Parse and dump back
    let parsed = yaml.load_str(original_yaml).unwrap();
    let dumped = yaml.dump_str(&parsed).unwrap();

    // Parse the dumped result to ensure it's valid
    let reparsed = yaml.load_str(&dumped).unwrap();
    assert_eq!(parsed, reparsed);
}

#[test]
fn test_custom_config() {
    let config = YamlConfig {
        loader_type: LoaderType::Safe,
        allow_duplicate_keys: false,
        explicit_start: Some(true),
        width: Some(120),
        ..Default::default()
    };

    let yaml = Yaml::with_config(config);

    // Test that it can still parse basic content
    let result = yaml.load_str("key: value").unwrap();

    let mut expected_map = indexmap::IndexMap::new();
    expected_map.insert(
        Value::String("key".to_string()),
        Value::String("value".to_string()),
    );
    let expected = Value::Mapping(expected_map);

    assert_eq!(result, expected);
}

#[test]
fn test_error_handling() {
    let yaml = Yaml::new();

    // Test actually invalid YAML syntax - mixed indentation which should fail
    let result = yaml.load_str("key:\n  value1\n\tvalue2");
    assert!(result.is_err());

    if let Err(error) = result {
        // Should have position information and a meaningful error message
        let error_str = error.to_string();
        assert!(!error_str.is_empty());
        assert!(error_str.len() > 5); // Should be descriptive

        // The error should indicate it's a parsing/scanning issue
        let error_lower = error_str.to_lowercase();
        assert!(
            error_lower.contains("error")
                || error_lower.contains("invalid")
                || error_lower.contains("indentation")
        );
    } else {
        panic!("Expected error for invalid YAML");
    }
}

#[test]
fn test_empty_input() {
    let yaml = Yaml::new();

    let result = yaml.load_str("").unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn test_whitespace_only() {
    let yaml = Yaml::new();

    let result = yaml.load_str("   \n  \t  \n  ").unwrap();
    assert_eq!(result, Value::Null);
}

#[test]
fn test_comments_ignored() {
    let yaml = Yaml::new();

    let yaml_content = r#"
# This is a comment
key: value  # Another comment
# Final comment
"#;

    let result = yaml.load_str(yaml_content).unwrap();

    let mut expected_map = indexmap::IndexMap::new();
    expected_map.insert(
        Value::String("key".to_string()),
        Value::String("value".to_string()),
    );
    let expected = Value::Mapping(expected_map);

    assert_eq!(result, expected);
}

#[test]
fn test_quoted_strings() {
    let yaml = Yaml::new();

    let yaml_content = r#"
single_quoted: 'hello world'
double_quoted: "hello world"
with_escapes: "hello\nworld\ttab"
"#;

    let result = yaml.load_str(yaml_content).unwrap();

    if let Value::Mapping(ref map) = result {
        assert_eq!(
            map.get(&Value::String("single_quoted".to_string())),
            Some(&Value::String("hello world".to_string()))
        );
        assert_eq!(
            map.get(&Value::String("double_quoted".to_string())),
            Some(&Value::String("hello world".to_string()))
        );
        assert_eq!(
            map.get(&Value::String("with_escapes".to_string())),
            Some(&Value::String("hello\nworld\ttab".to_string()))
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_mixed_sequence_types() {
    let yaml = Yaml::new();

    let yaml_content = r#"
mixed_array:
  - 42
  - "string"
  - true
  - null
  - [1, 2, 3]
  - key: nested_value
"#;

    let result = yaml.load_str(yaml_content).unwrap();

    if let Value::Mapping(ref map) = result {
        if let Some(Value::Sequence(seq)) = map.get(&Value::String("mixed_array".to_string())) {
            assert_eq!(seq.len(), 6);
            assert_eq!(seq[0], Value::Int(42));
            assert_eq!(seq[1], Value::String("string".to_string()));
            assert_eq!(seq[2], Value::Bool(true));
            assert_eq!(seq[3], Value::Null);

            // Check nested sequence
            if let Value::Sequence(ref nested) = seq[4] {
                assert_eq!(nested.len(), 3);
                assert_eq!(nested[0], Value::Int(1));
            } else {
                panic!("Expected nested sequence");
            }

            // Check mapping as a sequence item (- key: value creates a mapping entry)
            if let Value::Mapping(ref nested_map) = seq[5] {
                assert_eq!(
                    nested_map.get(&Value::String("key".to_string())),
                    Some(&Value::String("nested_value".to_string()))
                );
            } else {
                panic!("Expected mapping as sequence item, got: {:?}", seq[5]);
            }
        } else {
            panic!("Expected sequence");
        }
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_version_like_strings_not_parsed_as_float() {
    let yaml = Yaml::new();

    // Version strings like "0.5.8" must be parsed as strings, not floats.
    // Regression: the scanner treated "0.5.8" as the number 0.5 leaving ".8"
    // as a stray token, which corrupted the rest of the mapping.
    let input = r#"
metadata:
  app_version: 0.5.8
  category: core
  description: A package
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse YAML");

    if let Value::Mapping(root) = &parsed {
        let metadata = root
            .get(&Value::String("metadata".to_string()))
            .expect("missing 'metadata' key");
        if let Value::Mapping(meta) = metadata {
            // app_version must be a string "0.5.8", not a float 0.5
            let app_version = meta
                .get(&Value::String("app_version".to_string()))
                .expect("missing 'app_version' key");
            assert_eq!(
                *app_version,
                Value::String("0.5.8".to_string()),
                "0.5.8 should be parsed as a string, got {:?}",
                app_version
            );

            // category must still be correct (not shifted)
            let category = meta
                .get(&Value::String("category".to_string()))
                .expect("missing 'category' key — mapping was corrupted");
            assert_eq!(*category, Value::String("core".to_string()),);

            // description must still be correct
            let description = meta
                .get(&Value::String("description".to_string()))
                .expect("missing 'description' key — mapping was corrupted");
            assert_eq!(*description, Value::String("A package".to_string()),);
        } else {
            panic!("Expected metadata to be a mapping");
        }
    } else {
        panic!("Expected root to be a mapping");
    }
}

#[test]
fn test_version_string_round_trip() {
    let yaml = Yaml::new();

    let input = "version: 0.5.8\n";
    let parsed = yaml.load_str(input).expect("Failed to parse");
    let serialized = yaml.dump_str(&parsed).expect("Failed to serialize");
    let reparsed = yaml.load_str(&serialized).expect("Failed to re-parse");
    assert_eq!(parsed, reparsed, "Round-trip failed for version string");
}

/// Compact notation: the `-` of a block sequence sits at the same indent
/// as the parent mapping keys. Keys that follow the sequence (without `-`)
/// must be recognised as mapping siblings, NOT extra sequence items.
#[test]
fn test_compact_sequence_does_not_swallow_sibling_keys() {
    let yaml = Yaml::new();

    let input = r#"metadata:
  features:
  - upgrade
  - auto_config
  name: vynil
  type: system
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse YAML");

    if let Value::Mapping(root) = &parsed {
        let meta = root
            .get(&Value::String("metadata".to_string()))
            .expect("missing 'metadata'");
        if let Value::Mapping(meta) = meta {
            // features must be a 2-element sequence
            let features = meta
                .get(&Value::String("features".to_string()))
                .expect("missing 'features'");
            if let Value::Sequence(seq) = features {
                assert_eq!(seq.len(), 2, "features should have 2 items, got {:?}", seq);
                assert_eq!(seq[0], Value::String("upgrade".to_string()));
                assert_eq!(seq[1], Value::String("auto_config".to_string()));
            } else {
                panic!("features should be a sequence, got {:?}", features);
            }

            // name and type must be sibling keys of metadata
            assert_eq!(
                meta.get(&Value::String("name".to_string())),
                Some(&Value::String("vynil".to_string())),
                "name should be a sibling key of metadata"
            );
            assert_eq!(
                meta.get(&Value::String("type".to_string())),
                Some(&Value::String("system".to_string())),
                "type should be a sibling key of metadata"
            );
        } else {
            panic!("metadata should be a mapping");
        }
    } else {
        panic!("root should be a mapping");
    }
}

/// Top-level keys after a mapping that contains a compact sequence must
/// be parsed as top-level, not swallowed into the inner sequence.
#[test]
fn test_compact_sequence_does_not_swallow_top_level_keys() {
    let yaml = Yaml::new();

    let input = r#"metadata:
  features:
  - upgrade
  name: vynil
images:
  agent:
    registry: docker.io
requirements: []
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse YAML");

    if let Value::Mapping(root) = &parsed {
        assert!(
            root.get(&Value::String("metadata".to_string())).is_some(),
            "missing top-level 'metadata'"
        );
        assert!(
            root.get(&Value::String("images".to_string())).is_some(),
            "missing top-level 'images' — it was swallowed into the compact sequence"
        );
        assert!(
            root.get(&Value::String("requirements".to_string()))
                .is_some(),
            "missing top-level 'requirements'"
        );
    } else {
        panic!("root should be a mapping");
    }
}

/// The full package.yaml round-trip: parse → serialize → re-parse must be
/// structurally identical.
#[test]
fn test_package_yaml_round_trip() {
    let yaml = Yaml::new();

    let input = r#"apiVersion: vinyl.solidite.fr/v1beta1
kind: Package
metadata:
  app_version: 0.5.8
  category: core
  description: Vynil controller to manage vynil packages installations
  features:
  - upgrade
  - auto_config
  name: vynil
  type: system
images:
  agent:
    registry: docker.io
    repository: sebt3/vynil-agent
  controller:
    registry: docker.io
    repository: sebt3/vynil-operator
resources:
  controller:
    requests:
      cpu: 50m
      memory: 256Mi
    limits:
      cpu: 1000m
      memory: 512Mi
requirements: []
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse");
    let serialized = yaml.dump_str(&parsed).expect("Failed to serialize");
    let reparsed = yaml.load_str(&serialized).expect("Failed to re-parse");
    assert_eq!(
        parsed, reparsed,
        "Round-trip mismatch.\nSerialized:\n{}",
        serialized
    );
}

/// Strings that happen to contain dots+digits (like domain names) or
/// hyphens (like docker image names) should not be gratuitously quoted.
#[test]
fn test_no_unnecessary_quoting() {
    let yaml = Yaml::new();

    let input = r#"apiVersion: vinyl.solidite.fr/v1beta1
repository: sebt3/vynil-agent
name: my-app
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse");
    let serialized = yaml.dump_str(&parsed).expect("Failed to serialize");

    // These strings should NOT be quoted in the output
    assert!(
        !serialized.contains("\"vinyl.solidite.fr/v1beta1\""),
        "Domain-like string should not be quoted. Got:\n{}",
        serialized
    );
    assert!(
        !serialized.contains("\"sebt3/vynil-agent\""),
        "Path-like string should not be quoted. Got:\n{}",
        serialized
    );
    assert!(
        !serialized.contains("\"my-app\""),
        "Hyphenated string should not be quoted. Got:\n{}",
        serialized
    );
}

/// Nested compact sequences: the Kubernetes CRD pattern has multiple levels
/// of compact-notation sequences (versions, additionalPrinterColumns,
/// required, etc.). Each must close correctly when its sibling keys appear.
#[test]
fn test_nested_compact_sequences() {
    let yaml = Yaml::new();

    let input = r#"spec:
  versions:
  - additionalPrinterColumns:
    - description: Update schedule
      name: schedule
      type: string
    - description: Last update
      name: last_updated
      type: date
    name: v1
    schema:
      openAPIV3Schema:
        properties:
          spec:
            properties:
              schedule:
                type: string
            required:
            - schedule
            type: object
          status:
            description: The status object
            type: object
        required:
        - spec
        type: object
    served: true
    storage: true
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse");

    // Check top-level structure
    let root = match &parsed {
        Value::Mapping(m) => m,
        _ => panic!("root should be a mapping"),
    };
    let spec = match root.get(&Value::String("spec".into())) {
        Some(Value::Mapping(m)) => m,
        other => panic!("spec should be a mapping, got {:?}", other),
    };

    // versions must be a 1-element sequence
    let versions = match spec.get(&Value::String("versions".into())) {
        Some(Value::Sequence(s)) => s,
        other => panic!("versions should be a sequence, got {:?}", other),
    };
    assert_eq!(versions.len(), 1, "versions should have 1 item");

    // The single version item must be a mapping with expected keys
    let v1 = match &versions[0] {
        Value::Mapping(m) => m,
        other => panic!("versions[0] should be a mapping, got {:?}", other),
    };

    // additionalPrinterColumns: 2-element sequence (NOT swallowing name, schema, etc.)
    let cols = match v1.get(&Value::String("additionalPrinterColumns".into())) {
        Some(Value::Sequence(s)) => s,
        other => panic!(
            "additionalPrinterColumns should be a sequence, got {:?}",
            other
        ),
    };
    assert_eq!(
        cols.len(),
        2,
        "additionalPrinterColumns should have 2 items, got {:?}",
        cols
    );

    // name, schema, served, storage must be sibling keys
    assert_eq!(
        v1.get(&Value::String("name".into())),
        Some(&Value::String("v1".into())),
        "name must be a sibling key"
    );
    assert!(
        v1.get(&Value::String("schema".into())).is_some(),
        "schema must be a sibling key"
    );
    assert_eq!(
        v1.get(&Value::String("served".into())),
        Some(&Value::Bool(true)),
        "served must be a sibling key"
    );
    assert_eq!(
        v1.get(&Value::String("storage".into())),
        Some(&Value::Bool(true)),
        "storage must be a sibling key"
    );

    // Deeper: spec.properties.required must be [schedule], type must be "object"
    let oas = match v1.get(&Value::String("schema".into())) {
        Some(Value::Mapping(m)) => match m.get(&Value::String("openAPIV3Schema".into())) {
            Some(Value::Mapping(m)) => m,
            _ => panic!("openAPIV3Schema missing"),
        },
        _ => panic!("schema missing"),
    };
    let props = match oas.get(&Value::String("properties".into())) {
        Some(Value::Mapping(m)) => m,
        _ => panic!("properties missing"),
    };
    let spec_inner = match props.get(&Value::String("spec".into())) {
        Some(Value::Mapping(m)) => m,
        _ => panic!("spec inner missing"),
    };

    // required must be a compact sequence [schedule]
    let required = match spec_inner.get(&Value::String("required".into())) {
        Some(Value::Sequence(s)) => s,
        other => panic!("required should be a sequence, got {:?}", other),
    };
    assert_eq!(required.len(), 1);
    assert_eq!(required[0], Value::String("schedule".into()));

    // type must be a sibling key "object" (not swallowed by required sequence)
    assert_eq!(
        spec_inner.get(&Value::String("type".into())),
        Some(&Value::String("object".into())),
        "type must be a sibling of required"
    );

    // status must be a sibling of spec under properties
    assert!(
        props.get(&Value::String("status".into())).is_some(),
        "status must be a sibling of spec under properties"
    );
}

/// Round-trip of nested compact sequences must be structurally stable.
#[test]
fn test_nested_compact_sequences_round_trip() {
    let yaml = Yaml::new();

    let input = r#"spec:
  versions:
  - additionalPrinterColumns:
    - description: Update schedule
      name: schedule
    name: v1
    schema:
      openAPIV3Schema:
        properties:
          spec:
            required:
            - schedule
            type: object
          status:
            type: object
        required:
        - spec
        type: object
    served: true
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse");
    let serialized = yaml.dump_str(&parsed).expect("Failed to serialize");
    let reparsed = yaml.load_str(&serialized).expect("Failed to re-parse");
    assert_eq!(
        parsed, reparsed,
        "Nested compact sequence round-trip failed.\nSerialized:\n{}",
        serialized
    );
}

/// Diagnose CRD round-trip bug: nested anyOf/oneOf must not swallow sibling keys.
#[test]
fn test_crd_nested_anyof_round_trip() {
    let yaml = Yaml::new();

    let input = r#"spec:
  versions:
  - name: v1
    schema:
      openAPIV3Schema:
        properties:
          spec:
            properties:
              source:
                anyOf:
                - oneOf:
                  - required:
                    - list
                  - required:
                    - harbor
                - enum:
                  - null
                  nullable: true
                description: Source type
                type: object
            required:
            - schedule
            type: object
          status:
            description: Status
            type: object
        required:
        - spec
        type: object
    served: true
    storage: true
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse");
    let serialized = yaml.dump_str(&parsed).expect("Failed to serialize");

    println!("=== SERIALIZED OUTPUT (Yaml::new) ===");
    println!("{}", serialized);
    println!("=== END ===");

    // status: must appear as a mapping key, NOT as a sequence item "- status:"
    assert!(
        serialized.contains("status:"),
        "Serialized output must contain 'status:' as a mapping key"
    );
    assert!(
        !serialized.contains("- status:"),
        "Serialized output must NOT contain '- status:' (status swallowed into sequence)\nSerialized:\n{}",
        serialized
    );

    // Also verify structural stability via re-parse
    let reparsed = yaml.load_str(&serialized).expect("Failed to re-parse");
    assert_eq!(
        parsed, reparsed,
        "CRD anyOf round-trip failed.\nSerialized:\n{}",
        serialized
    );
}

/// Same CRD test but with round_trip.rs config (sequence_indent: Some(0), indent: 2, emit_anchors: false).
#[test]
fn test_crd_nested_anyof_round_trip_with_config() {
    use rust_yaml::IndentConfig;

    let yaml = Yaml::with_config(YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        emit_anchors: false,
        indent: IndentConfig {
            indent: 2,
            sequence_indent: Some(0),
            ..Default::default()
        },
        ..Default::default()
    });

    let input = r#"spec:
  versions:
  - name: v1
    schema:
      openAPIV3Schema:
        properties:
          spec:
            properties:
              source:
                anyOf:
                - oneOf:
                  - required:
                    - list
                  - required:
                    - harbor
                - enum:
                  - null
                  nullable: true
                description: Source type
                type: object
            required:
            - schedule
            type: object
          status:
            description: Status
            type: object
        required:
        - spec
        type: object
    served: true
    storage: true
"#;

    let docs = yaml.load_all_str(input).expect("Failed to parse");
    let serialized = yaml.dump_all_str(&docs).expect("Failed to serialize");

    println!("=== SERIALIZED OUTPUT (with_config, sequence_indent=0) ===");
    println!("{}", serialized);
    println!("=== END ===");

    // status: must appear as a mapping key, NOT as a sequence item "- status:"
    assert!(
        serialized.contains("status:"),
        "Serialized output must contain 'status:' as a mapping key"
    );
    assert!(
        !serialized.contains("- status:"),
        "Serialized output must NOT contain '- status:' (status swallowed into sequence)\nSerialized:\n{}",
        serialized
    );

    // Structural stability via re-parse
    let reparsed = yaml.load_all_str(&serialized).expect("Failed to re-parse");
    assert_eq!(
        docs, reparsed,
        "CRD anyOf round-trip with config failed.\nSerialized:\n{}",
        serialized
    );
}

/// Regression test: mapping keys after deeply-nested compact sequences must stay at the
/// correct indent level.  The bug was in check_active_mapping_at_level which failed to
/// count BlockSequenceStart tokens, causing it to walk past the real mapping start and
/// return false — triggering a spurious BlockMappingStart that split one mapping into two.
#[test]
fn test_mapping_keys_after_nested_compact_sequences() {
    let yaml = Yaml::new();

    let input = r#"spec:
  versions:
  - name: v1
    schema:
      openAPIV3Schema:
        properties:
          spec:
            properties:
              source:
                anyOf:
                - oneOf:
                  - required:
                    - list
                  - required:
                    - harbor
                - enum:
                  - null
                  nullable: true
                description: Source type
                type: object
            required:
            - schedule
            type: object
          status:
            description: Status
            type: object
        required:
        - spec
        type: object
    served: true
    storage: true
"#;

    let parsed = yaml.load_str(input).expect("Failed to parse");

    let versions = parsed
        .as_mapping()
        .and_then(|m| m.get(&Value::String("spec".into())))
        .and_then(|s| s.as_mapping())
        .and_then(|m| m.get(&Value::String("versions".into())))
        .and_then(|v| v.as_sequence())
        .expect("spec.versions must be a sequence");

    assert_eq!(versions.len(), 1);

    let item = versions[0]
        .as_mapping()
        .expect("versions[0] must be a mapping");
    assert!(
        item.contains_key(&Value::String("served".into())),
        "served must be inside the sequence item"
    );
    assert!(
        item.contains_key(&Value::String("storage".into())),
        "storage must be inside the sequence item"
    );

    let top = parsed.as_mapping().expect("top level must be a mapping");
    assert!(
        !top.contains_key(&Value::String("served".into())),
        "served must NOT be at top level"
    );
    assert!(
        !top.contains_key(&Value::String("storage".into())),
        "storage must NOT be at top level"
    );

    let serialized = yaml.dump_str(&parsed).expect("Failed to serialize");
    let reparsed = yaml.load_str(&serialized).expect("Failed to re-parse");
    assert_eq!(
        parsed, reparsed,
        "Round-trip must be stable.\nSerialized:\n{}",
        serialized
    );
}
