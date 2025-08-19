#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_raw_string_hashes)]

use indexmap::IndexMap;
use rust_yaml::{BasicEmitter, Emitter, Value, Yaml};

#[test]
fn test_emit_with_yaml_version() {
    let mut emitter = BasicEmitter::new();
    emitter.set_yaml_version(1, 2);

    let mut map = IndexMap::new();
    map.insert(
        Value::String("key".to_string()),
        Value::String("value".to_string()),
    );
    let value = Value::Mapping(map);

    let mut output = Vec::new();
    emitter.emit(&value, &mut output).unwrap();

    let result = String::from_utf8(output).unwrap();
    println!("Emitted:\n{}", result);

    assert!(result.contains("%YAML 1.2"));
    assert!(result.contains("---"));
    assert!(result.contains("key: value"));
}

#[test]
fn test_emit_with_tag_directives() {
    let mut emitter = BasicEmitter::new();
    emitter.add_tag_directive("!".to_string(), "tag:example.com,2024:".to_string());
    emitter.add_tag_directive("!!".to_string(), "tag:yaml.org,2002:".to_string());

    let mut map = IndexMap::new();
    map.insert(
        Value::String("key".to_string()),
        Value::String("value".to_string()),
    );
    let value = Value::Mapping(map);

    let mut output = Vec::new();
    emitter.emit(&value, &mut output).unwrap();

    let result = String::from_utf8(output).unwrap();
    println!("Emitted:\n{}", result);

    assert!(result.contains("%TAG ! tag:example.com,2024:"));
    assert!(result.contains("%TAG !! tag:yaml.org,2002:"));
    assert!(result.contains("---"));
    assert!(result.contains("key: value"));
}

#[test]
fn test_emit_with_both_directives() {
    let mut emitter = BasicEmitter::new();
    emitter.set_yaml_version(1, 2);
    emitter.add_tag_directive("!e!".to_string(), "tag:example.com,2024:".to_string());

    let mut map = IndexMap::new();
    map.insert(
        Value::String("name".to_string()),
        Value::String("Alice".to_string()),
    );
    map.insert(Value::String("age".to_string()), Value::Int(30));
    let value = Value::Mapping(map);

    let mut output = Vec::new();
    emitter.emit(&value, &mut output).unwrap();

    let result = String::from_utf8(output).unwrap();
    println!("Emitted:\n{}", result);

    assert!(result.contains("%YAML 1.2"));
    assert!(result.contains("%TAG !e! tag:example.com,2024:"));
    assert!(result.contains("---"));
    assert!(result.contains("name: Alice"));
    assert!(result.contains("age: 30"));
}

#[test]
fn test_directives_roundtrip() {
    let yaml_content = r#"%YAML 1.2
%TAG ! tag:example.com,2024:
---
key: value
nested:
  item1: 10
  item2: 20
"#;

    // Parse the YAML
    let yaml = Yaml::new();
    let parsed = yaml.load_str(yaml_content).unwrap();

    // Emit it back with directives
    let mut emitter = BasicEmitter::new();
    emitter.set_yaml_version(1, 2);
    emitter.add_tag_directive("!".to_string(), "tag:example.com,2024:".to_string());

    let mut output = Vec::new();
    emitter.emit(&parsed, &mut output).unwrap();

    let result = String::from_utf8(output).unwrap();
    println!("Original:\n{}", yaml_content);
    println!("Roundtrip:\n{}", result);

    // Parse the emitted YAML to verify it's valid
    let reparsed = yaml.load_str(&result).unwrap();

    // Check that the values match
    assert_eq!(parsed, reparsed);

    // Check that directives are present
    assert!(result.contains("%YAML 1.2"));
    assert!(result.contains("%TAG ! tag:example.com,2024:"));
    assert!(result.contains("---"));
}

#[test]
fn test_clear_directives() {
    let mut emitter = BasicEmitter::new();

    // Set directives
    emitter.set_yaml_version(1, 2);
    emitter.add_tag_directive("!".to_string(), "tag:example.com,2024:".to_string());

    // Clear them
    emitter.clear_directives();

    let value = Value::String("test".to_string());
    let mut output = Vec::new();
    emitter.emit(&value, &mut output).unwrap();

    let result = String::from_utf8(output).unwrap();
    println!("Emitted after clear:\n{}", result);

    // Should not contain directives
    assert!(!result.contains("%YAML"));
    assert!(!result.contains("%TAG"));
    assert!(!result.contains("---"));
    assert_eq!(result.trim(), "test");
}

#[test]
fn test_multiple_documents_with_directives() {
    let mut emitter = BasicEmitter::new();

    // First document with directives
    emitter.set_yaml_version(1, 2);
    let value1 = Value::String("doc1".to_string());
    let mut output1 = Vec::new();
    emitter.emit(&value1, &mut output1).unwrap();

    // Second document without changing directives
    let value2 = Value::String("doc2".to_string());
    let mut output2 = Vec::new();
    emitter.emit(&value2, &mut output2).unwrap();

    let result1 = String::from_utf8(output1).unwrap();
    let result2 = String::from_utf8(output2).unwrap();

    println!("Doc1:\n{}", result1);
    println!("Doc2:\n{}", result2);

    // Both should have the YAML version directive
    assert!(result1.contains("%YAML 1.2"));
    assert!(result2.contains("%YAML 1.2"));
}
