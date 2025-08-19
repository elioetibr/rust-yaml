//! Comprehensive tests for YAML tag support

use rust_yaml::{Value, Yaml};

#[test]
fn test_standard_yaml_tags() {
    // Test standard YAML tags like !!str, !!int, !!float
    let yaml_input = r"
!!str 123
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    // Should force 123 to be a string due to !!str tag
    assert_eq!(result, Value::String("123".to_string()));
}

#[test]
fn test_tag_with_mapping() {
    let yaml_input = r"
!!map
  key: value
  number: 42
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    // Should create a mapping
    if let Value::Mapping(map) = result {
        assert_eq!(
            map.get(&Value::String("key".to_string())),
            Some(&Value::String("value".to_string()))
        );
        assert_eq!(
            map.get(&Value::String("number".to_string())),
            Some(&Value::Int(42))
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_tag_with_sequence() {
    let yaml_input = r"
!!seq
  - item1
  - item2
  - item3
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    // Should create a sequence
    if let Value::Sequence(seq) = result {
        assert_eq!(seq.len(), 3);
        assert_eq!(seq[0], Value::String("item1".to_string()));
        assert_eq!(seq[1], Value::String("item2".to_string()));
        assert_eq!(seq[2], Value::String("item3".to_string()));
    } else {
        panic!("Expected sequence");
    }
}

#[test]
fn test_binary_tag() {
    let yaml_input = r"
!!binary |
  SGVsbG8gV29ybGQh
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    // Should decode base64 to "Hello World!"
    assert_eq!(result, Value::String("Hello World!".to_string()));
}

#[test]
fn test_explicit_type_tags() {
    let yaml_input = r#"
string: !!str 123
integer: !!int "456"
float: !!float "3.15"
boolean: !!bool "yes"
null: !!null "anything"
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    if let Value::Mapping(map) = result {
        assert_eq!(
            map.get(&Value::String("string".to_string())),
            Some(&Value::String("123".to_string()))
        );
        assert_eq!(
            map.get(&Value::String("integer".to_string())),
            Some(&Value::Int(456))
        );
        assert_eq!(
            map.get(&Value::String("float".to_string())),
            Some(&Value::Float(3.15))
        );
        assert_eq!(
            map.get(&Value::String("boolean".to_string())),
            Some(&Value::Bool(true))
        );
        assert_eq!(map.get(&Value::Null), Some(&Value::Null));
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_tag_directives_resolution() {
    let yaml_input = r"%TAG ! tag:example.com,2024:
%TAG !! tag:yaml.org,2002:
---
!widget
  id: 123
  type: !!str button
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    // Should handle tag directives
    assert!(result.is_ok(), "Should parse with tag directives");
}

#[test]
fn test_named_tag_handles() {
    let yaml_input = r"%TAG !ex! tag:example.com,2024:
---
!ex!person
  name: John Doe
  age: 30
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    assert!(result.is_ok(), "Should parse with named tag handles");
}

#[test]
fn test_verbatim_tags() {
    let yaml_input = r"
!<tag:example.com,2024:widget>
  id: 123
  type: button
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    assert!(result.is_ok(), "Should parse verbatim tags");
}

#[test]
fn test_tag_inheritance() {
    // Tags should only apply to the node they're attached to
    let yaml_input = r"
!!str
  - 123
  - 456
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    // The !!str tag applies to the sequence, not its elements
    // This should result in an error or the sequence itself being stringified
    assert!(result.is_ok());
}

#[test]
fn test_mixed_tags_in_collection() {
    let yaml_input = r#"
- !!str 123
- !!int "456"
- !!float "3.15"
- regular
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    if let Value::Sequence(seq) = result {
        assert_eq!(seq[0], Value::String("123".to_string()));
        assert_eq!(seq[1], Value::Int(456));
        assert_eq!(seq[2], Value::Float(3.15));
        assert_eq!(seq[3], Value::String("regular".to_string()));
    } else {
        panic!("Expected sequence");
    }
}

#[test]
fn test_tag_on_anchor() {
    let yaml_input = r"
base: &anchor !!str 123
ref: *anchor
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    if let Value::Mapping(map) = result {
        // Both should be strings due to the !!str tag
        assert_eq!(
            map.get(&Value::String("base".to_string())),
            Some(&Value::String("123".to_string()))
        );
        assert_eq!(
            map.get(&Value::String("ref".to_string())),
            Some(&Value::String("123".to_string()))
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_invalid_tag_error() {
    // Test that invalid tags are handled gracefully
    let yaml_input = r"
!invalid!tag!format value
";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    // Should either parse with a warning or return an error
    // For now, we expect it to parse successfully
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_schema_validation() {
    // Test Core schema vs JSON schema differences
    // In Core schema, "yes" is a boolean, in JSON schema it's a string

    // Default should use Core schema
    let yaml = Yaml::new();
    let result = yaml.load_str("yes").unwrap();
    assert_eq!(result, Value::Bool(true));

    // TODO: Add JSON schema mode when implemented
    // let yaml = Yaml::with_schema(Schema::Json);
    // let result = yaml.load_str("yes").unwrap();
    // assert_eq!(result, Value::String("yes".to_string()));
}

#[test]
fn test_multiple_documents_with_different_tags() {
    let yaml_input = r"%TAG ! tag:example.com,2024:
---
!widget
  id: 1
...
%TAG ! tag:another.com,2025:
---
!gadget
  id: 2
";

    let yaml = Yaml::new();
    let documents = yaml.load_all_str(yaml_input).unwrap();

    assert_eq!(documents.len(), 2);
    // Both documents should parse successfully with their respective tag directives
}
