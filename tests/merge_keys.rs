//! Tests for YAML merge key support

use rust_yaml::{Value, Yaml};

#[test]
fn test_basic_merge_key() {
    let yaml_str = r#"
defaults: &defaults
  adapter: postgres
  host: localhost
  port: 5432

development:
  <<: *defaults
  database: dev_db
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str).expect("Failed to parse YAML");

    if let Value::Mapping(root) = result {
        let dev = root
            .get(&Value::String("development".to_string()))
            .expect("Missing 'development' key");

        if let Value::Mapping(dev_map) = dev {
            // Check that all keys from defaults are present
            assert_eq!(
                dev_map.get(&Value::String("adapter".to_string())),
                Some(&Value::String("postgres".to_string()))
            );
            assert_eq!(
                dev_map.get(&Value::String("host".to_string())),
                Some(&Value::String("localhost".to_string()))
            );
            assert_eq!(
                dev_map.get(&Value::String("port".to_string())),
                Some(&Value::Int(5432))
            );
            assert_eq!(
                dev_map.get(&Value::String("database".to_string())),
                Some(&Value::String("dev_db".to_string()))
            );
        } else {
            panic!("'development' is not a mapping");
        }
    } else {
        panic!("Root is not a mapping");
    }
}

#[test]
fn test_merge_key_override() {
    let yaml_str = r#"
defaults: &defaults
  host: localhost
  port: 5432

custom:
  <<: *defaults
  port: 3306  # Should override the merged value
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str).expect("Failed to parse YAML");

    if let Value::Mapping(root) = result {
        let custom = root
            .get(&Value::String("custom".to_string()))
            .expect("Missing 'custom' key");

        if let Value::Mapping(custom_map) = custom {
            assert_eq!(
                custom_map.get(&Value::String("host".to_string())),
                Some(&Value::String("localhost".to_string())),
                "Host should be merged"
            );
            assert_eq!(
                custom_map.get(&Value::String("port".to_string())),
                Some(&Value::Int(3306)),
                "Port should be overridden"
            );
        }
    }
}

#[test]
fn test_multiple_merge_keys() {
    let yaml_str = r#"
base1: &base1
  a: 1
  b: 2

base2: &base2
  c: 3
  d: 4

merged:
  <<: [*base1, *base2]
  e: 5
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str).expect("Failed to parse YAML");

    if let Value::Mapping(root) = result {
        let merged = root
            .get(&Value::String("merged".to_string()))
            .expect("Missing 'merged' key");

        if let Value::Mapping(merged_map) = merged {
            assert_eq!(merged_map.len(), 5, "Should have all 5 keys");
            assert_eq!(
                merged_map.get(&Value::String("a".to_string())),
                Some(&Value::Int(1))
            );
            assert_eq!(
                merged_map.get(&Value::String("b".to_string())),
                Some(&Value::Int(2))
            );
            assert_eq!(
                merged_map.get(&Value::String("c".to_string())),
                Some(&Value::Int(3))
            );
            assert_eq!(
                merged_map.get(&Value::String("d".to_string())),
                Some(&Value::Int(4))
            );
            assert_eq!(
                merged_map.get(&Value::String("e".to_string())),
                Some(&Value::Int(5))
            );
        }
    }
}

#[test]
fn test_nested_merge_keys() {
    let yaml_str = r#"
base: &base
  name: base
  value: 1

extended: &extended
  <<: *base
  extra: 2

final:
  <<: *extended
  override: 3
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str).expect("Failed to parse YAML");

    if let Value::Mapping(root) = result {
        // Check extended first
        let extended = root
            .get(&Value::String("extended".to_string()))
            .expect("Missing 'extended' key");

        if let Value::Mapping(extended_map) = extended {
            assert_eq!(extended_map.len(), 3, "Extended should have 3 keys");
            assert_eq!(
                extended_map.get(&Value::String("name".to_string())),
                Some(&Value::String("base".to_string()))
            );
        }

        // Check final
        let final_node = root
            .get(&Value::String("final".to_string()))
            .expect("Missing 'final' key");

        if let Value::Mapping(final_map) = final_node {
            assert_eq!(final_map.len(), 4, "Final should have 4 keys");
            assert_eq!(
                final_map.get(&Value::String("name".to_string())),
                Some(&Value::String("base".to_string())),
                "Should inherit from base through extended"
            );
            assert_eq!(
                final_map.get(&Value::String("value".to_string())),
                Some(&Value::Int(1))
            );
            assert_eq!(
                final_map.get(&Value::String("extra".to_string())),
                Some(&Value::Int(2))
            );
            assert_eq!(
                final_map.get(&Value::String("override".to_string())),
                Some(&Value::Int(3))
            );
        }
    }
}

#[test]
fn test_merge_key_with_null_values() {
    let yaml_str = r#"
defaults: &defaults
  a: 1
  b: ~

merged:
  <<: *defaults
  c: 3
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str).expect("Failed to parse YAML");

    if let Value::Mapping(root) = result {
        let merged = root
            .get(&Value::String("merged".to_string()))
            .expect("Missing 'merged' key");

        if let Value::Mapping(merged_map) = merged {
            assert_eq!(
                merged_map.get(&Value::String("a".to_string())),
                Some(&Value::Int(1))
            );
            assert_eq!(
                merged_map.get(&Value::String("b".to_string())),
                Some(&Value::Null)
            );
            assert_eq!(
                merged_map.get(&Value::String("c".to_string())),
                Some(&Value::Int(3))
            );
        }
    }
}

#[test]
fn test_merge_key_precedence() {
    // Test that explicit keys override merged keys
    let yaml_str = r#"
defaults: &defaults
  key1: default1
  key2: default2

override:
  key1: explicit1  # Before merge
  <<: *defaults
  key2: explicit2  # After merge
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str).expect("Failed to parse YAML");

    if let Value::Mapping(root) = result {
        let override_node = root
            .get(&Value::String("override".to_string()))
            .expect("Missing 'override' key");

        if let Value::Mapping(override_map) = override_node {
            assert_eq!(
                override_map.get(&Value::String("key1".to_string())),
                Some(&Value::String("explicit1".to_string())),
                "Explicit key before merge should take precedence"
            );
            assert_eq!(
                override_map.get(&Value::String("key2".to_string())),
                Some(&Value::String("explicit2".to_string())),
                "Explicit key after merge should take precedence"
            );
        }
    }
}

#[test]
fn test_merge_key_with_complex_values() {
    let yaml_str = r#"
defaults: &defaults
  simple: value
  list:
    - item1
    - item2
  nested:
    inner: data

merged:
  <<: *defaults
  extra: added
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str).expect("Failed to parse YAML");

    if let Value::Mapping(root) = result {
        let merged = root
            .get(&Value::String("merged".to_string()))
            .expect("Missing 'merged' key");

        if let Value::Mapping(merged_map) = merged {
            // Check simple value
            assert_eq!(
                merged_map.get(&Value::String("simple".to_string())),
                Some(&Value::String("value".to_string()))
            );

            // Check list
            if let Some(Value::Sequence(list)) = merged_map.get(&Value::String("list".to_string()))
            {
                assert_eq!(list.len(), 2);
                assert_eq!(list[0], Value::String("item1".to_string()));
                assert_eq!(list[1], Value::String("item2".to_string()));
            } else {
                panic!("List not merged correctly");
            }

            // Check nested mapping
            if let Some(Value::Mapping(nested)) =
                merged_map.get(&Value::String("nested".to_string()))
            {
                assert_eq!(
                    nested.get(&Value::String("inner".to_string())),
                    Some(&Value::String("data".to_string()))
                );
            } else {
                panic!("Nested mapping not merged correctly");
            }

            // Check extra key
            assert_eq!(
                merged_map.get(&Value::String("extra".to_string())),
                Some(&Value::String("added".to_string()))
            );
        }
    }
}

#[test]
fn test_invalid_merge_key_value() {
    // Merge key with non-mapping value should fail
    let yaml_str = r#"
invalid:
  <<: "not a mapping"
  key: value
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str);

    assert!(
        result.is_err(),
        "Should fail when merge key value is not a mapping"
    );
}

#[test]
fn test_merge_key_in_sequence() {
    // Merge keys should only work in mappings, not sequences
    let yaml_str = r#"
defaults: &defaults
  a: 1

list:
  - <<: *defaults  # This should create a mapping with << as a key
  - b: 2
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_str).expect("Failed to parse YAML");

    if let Value::Mapping(root) = result
        && let Some(Value::Sequence(list)) = root.get(&Value::String("list".to_string()))
    {
        // First item should be a mapping with the merge
        if let Value::Mapping(first) = &list[0] {
            // Since we're in a sequence, the merge should work
            assert_eq!(
                first.get(&Value::String("a".to_string())),
                Some(&Value::Int(1)),
                "Merge should work in mapping within sequence"
            );
        }
    }
}
