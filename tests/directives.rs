//! Tests for YAML directives (%YAML and %TAG)

use rust_yaml::{Value, Yaml};

#[test]
fn test_yaml_version_directive() {
    let yaml_input = r#"%YAML 1.2
---
foo: bar
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    // Should parse the document content correctly
    if let Value::Mapping(map) = result {
        assert_eq!(
            map.get(&Value::String("foo".to_string())),
            Some(&Value::String("bar".to_string()))
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_tag_directive_basic() {
    let yaml_input = r#"%TAG ! tag:example.com,2024:
---
!person
name: John Doe
age: 30
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    // Should handle tag directives (even if not fully resolved yet)
    assert!(result.is_ok(), "Should parse document with tag directive");
}

#[test]
fn test_multiple_tag_directives() {
    let yaml_input = r#"%TAG ! tag:example.com,2024:
%TAG !! tag:yaml.org,2002:
---
!!str "Hello"
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    assert!(
        result.is_ok(),
        "Should parse document with multiple tag directives"
    );
}

#[test]
fn test_directives_with_multiple_documents() {
    let yaml_input = r#"%YAML 1.2
---
doc1: value1
...
%YAML 1.2
---
doc2: value2
"#;

    let yaml = Yaml::new();

    // Load all documents
    let documents = yaml.load_all_str(yaml_input).unwrap();

    assert_eq!(documents.len(), 2, "Should parse two documents");

    // Check first document
    if let Value::Mapping(map) = &documents[0] {
        assert_eq!(
            map.get(&Value::String("doc1".to_string())),
            Some(&Value::String("value1".to_string()))
        );
    } else {
        panic!("Expected mapping for first document");
    }

    // Check second document
    if let Value::Mapping(map) = &documents[1] {
        assert_eq!(
            map.get(&Value::String("doc2".to_string())),
            Some(&Value::String("value2".to_string()))
        );
    } else {
        panic!("Expected mapping for second document");
    }
}

#[test]
fn test_implicit_document_with_directives() {
    // Document without explicit --- should still work with directives
    let yaml_input = r#"%YAML 1.2
%TAG ! tag:example.com,2024:
key: value
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input).unwrap();

    if let Value::Mapping(map) = result {
        assert_eq!(
            map.get(&Value::String("key".to_string())),
            Some(&Value::String("value".to_string()))
        );
    } else {
        panic!("Expected mapping");
    }
}

#[test]
fn test_tag_directive_with_handle() {
    let yaml_input = r#"%TAG !ex! tag:example.com,2024:
---
!ex!widget
id: 123
type: button
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    assert!(
        result.is_ok(),
        "Should parse document with named tag handle"
    );
}

#[test]
fn test_directives_only_apply_to_next_document() {
    let yaml_input = r#"%YAML 1.2
%TAG ! tag:example.com,2024:
---
doc1: with_directives
...
---
doc2: without_directives
"#;

    let yaml = Yaml::new();
    let documents = yaml.load_all_str(yaml_input).unwrap();

    assert_eq!(documents.len(), 2, "Should parse both documents");

    // Both documents should parse correctly
    // The directives only apply to the first document
    for (i, doc) in documents.iter().enumerate() {
        if let Value::Mapping(_) = doc {
            // Good - parsed as mapping
        } else {
            panic!("Document {} should be a mapping", i + 1);
        }
    }
}

#[test]
fn test_yaml_version_1_1_compatibility() {
    // Test that we can at least parse YAML 1.1 directive
    let yaml_input = r#"%YAML 1.1
---
# YAML 1.1 had different boolean representations
yes: true
no: false
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    assert!(result.is_ok(), "Should parse YAML 1.1 document");
}

#[test]
fn test_directive_scanner_integration() {
    // Test that directives are properly scanned and passed through the pipeline
    let yaml_input = r#"%YAML 1.2
%TAG !foo! tag:example.com,2024/foo:
%TAG !bar! tag:example.com,2024/bar:
---
regular: value
!foo!widget: component
!bar!config: settings
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input);

    // Should not error even with complex directives
    assert!(
        result.is_ok(),
        "Should handle multiple custom tag directives"
    );
}
