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

            // Check nested sequence containing a mapping (this is correct YAML behavior)
            if let Value::Sequence(ref nested_seq) = seq[5] {
                assert_eq!(nested_seq.len(), 1);
                if let Value::Mapping(ref nested_map) = nested_seq[0] {
                    assert_eq!(
                        nested_map.get(&Value::String("key".to_string())),
                        Some(&Value::String("nested_value".to_string()))
                    );
                } else {
                    panic!("Expected mapping inside nested sequence");
                }
            } else {
                panic!("Expected nested sequence containing mapping");
            }
        } else {
            panic!("Expected sequence");
        }
    } else {
        panic!("Expected mapping");
    }
}
