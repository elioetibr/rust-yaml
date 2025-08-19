#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::uninlined_format_args)]

use indexmap::IndexMap;
use rust_yaml::{Value, Yaml};

#[test]
fn test_simple_complex_key_object() {
    let yaml_content = r#"
? {name: John, age: 30}
: person_data
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        // Should have one entry with a complex key
        assert_eq!(map.len(), 1);

        // Find the complex key
        let mut found_complex_key = false;
        for (key, value) in map {
            if let Value::Mapping(key_map) = key {
                // The complex key should be a mapping with name and age
                assert_eq!(key_map.len(), 2);
                assert_eq!(
                    key_map.get(&Value::String("name".to_string())),
                    Some(&Value::String("John".to_string()))
                );
                assert_eq!(
                    key_map.get(&Value::String("age".to_string())),
                    Some(&Value::Int(30))
                );
                assert_eq!(value, &Value::String("person_data".to_string()));
                found_complex_key = true;
            }
        }
        assert!(found_complex_key, "Should find the complex key");
    } else {
        panic!("Result should be a mapping");
    }
}

#[test]
fn test_complex_key_sequence() {
    let yaml_content = r#"
? [apple, banana, cherry]
: fruit_list
? [1, 2, 3]
: number_list
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        assert_eq!(map.len(), 2);

        // Check fruit list key
        let fruit_key = Value::Sequence(vec![
            Value::String("apple".to_string()),
            Value::String("banana".to_string()),
            Value::String("cherry".to_string()),
        ]);
        assert_eq!(
            map.get(&fruit_key),
            Some(&Value::String("fruit_list".to_string()))
        );

        // Check number list key
        let number_key = Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)]);
        assert_eq!(
            map.get(&number_key),
            Some(&Value::String("number_list".to_string()))
        );
    } else {
        panic!("Result should be a mapping");
    }
}

#[test]
fn test_mixed_simple_and_complex_keys() {
    let yaml_content = r#"
simple_key: simple_value
? {compound: key}
: compound_value
another_simple: another_value
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        assert_eq!(map.len(), 3);

        // Check simple keys
        assert_eq!(
            map.get(&Value::String("simple_key".to_string())),
            Some(&Value::String("simple_value".to_string()))
        );
        assert_eq!(
            map.get(&Value::String("another_simple".to_string())),
            Some(&Value::String("another_value".to_string()))
        );

        // Check complex key
        let complex_key = Value::Mapping({
            let mut key_map = IndexMap::new();
            key_map.insert(
                Value::String("compound".to_string()),
                Value::String("key".to_string()),
            );
            key_map
        });
        assert_eq!(
            map.get(&complex_key),
            Some(&Value::String("compound_value".to_string()))
        );
    } else {
        panic!("Result should be a mapping");
    }
}

#[test]
fn test_nested_complex_keys() {
    let yaml_content = r#"
?
  level1:
    level2: nested_key
: nested_value
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        assert_eq!(map.len(), 1);

        // Build the expected complex key structure
        let level2_map = {
            let mut inner = IndexMap::new();
            inner.insert(
                Value::String("level2".to_string()),
                Value::String("nested_key".to_string()),
            );
            inner
        };
        let level1_map = {
            let mut outer = IndexMap::new();
            outer.insert(
                Value::String("level1".to_string()),
                Value::Mapping(level2_map),
            );
            outer
        };
        let complex_key = Value::Mapping(level1_map);

        assert_eq!(
            map.get(&complex_key),
            Some(&Value::String("nested_value".to_string()))
        );
    } else {
        panic!("Result should be a mapping");
    }
}

#[test]
fn test_complex_key_with_null() {
    let yaml_content = r#"
? {key1: null, key2: ~}
: null_value
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        assert_eq!(map.len(), 1);

        let complex_key = Value::Mapping({
            let mut key_map = IndexMap::new();
            key_map.insert(Value::String("key1".to_string()), Value::Null);
            key_map.insert(Value::String("key2".to_string()), Value::Null);
            key_map
        });
        assert_eq!(
            map.get(&complex_key),
            Some(&Value::String("null_value".to_string()))
        );
    } else {
        panic!("Result should be a mapping");
    }
}

#[test]
fn test_complex_key_with_boolean() {
    let yaml_content = r#"
? {enabled: true, disabled: false}
: boolean_config
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        assert_eq!(map.len(), 1);

        let complex_key = Value::Mapping({
            let mut key_map = IndexMap::new();
            key_map.insert(Value::String("enabled".to_string()), Value::Bool(true));
            key_map.insert(Value::String("disabled".to_string()), Value::Bool(false));
            key_map
        });
        assert_eq!(
            map.get(&complex_key),
            Some(&Value::String("boolean_config".to_string()))
        );
    } else {
        panic!("Result should be a mapping");
    }
}

#[test]
fn test_complex_key_multiline() {
    let yaml_content = r#"
?
  name: "John Doe"
  age: 30
  address:
    street: "123 Main St"
    city: "Anytown"
:
  full_person_record
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        assert_eq!(map.len(), 1);

        // Should find exactly one key-value pair
        let mut found = false;
        for (key, value) in map {
            if let Value::Mapping(key_map) = key {
                assert_eq!(key_map.len(), 3); // name, age, address
                assert_eq!(value, &Value::String("full_person_record".to_string()));
                found = true;
            }
        }
        assert!(found, "Should find the complex multiline key");
    } else {
        panic!("Result should be a mapping");
    }
}

#[test]
fn test_complex_key_round_trip() {
    let yaml_content = r#"? {type: user, id: 123}
: user_data
simple: value"#;

    let yaml = Yaml::new();

    // Parse original
    let parsed = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    // Serialize back to YAML
    let serialized = yaml
        .dump_str(&parsed)
        .expect("Should serialize successfully");

    // Parse the serialized version
    let round_trip = yaml
        .load_str(&serialized)
        .expect("Should parse round-trip successfully");

    // Both should be equivalent
    if let (Value::Mapping(orig), Value::Mapping(rt)) = (&parsed, &round_trip) {
        assert_eq!(orig.len(), rt.len());

        // Complex keys should be preserved
        let complex_key = Value::Mapping({
            let mut key_map = IndexMap::new();
            key_map.insert(
                Value::String("type".to_string()),
                Value::String("user".to_string()),
            );
            key_map.insert(Value::String("id".to_string()), Value::Int(123));
            key_map
        });

        assert_eq!(orig.get(&complex_key), rt.get(&complex_key));
        assert_eq!(
            orig.get(&Value::String("simple".to_string())),
            rt.get(&Value::String("simple".to_string()))
        );
    } else {
        panic!("Both should be mappings");
    }
}

#[test]
fn test_empty_complex_key() {
    let yaml_content = r#"
? {}
: empty_object_key
? []
: empty_array_key
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        assert_eq!(map.len(), 2);

        // Check empty object key
        let empty_obj_key = Value::Mapping(IndexMap::new());
        assert_eq!(
            map.get(&empty_obj_key),
            Some(&Value::String("empty_object_key".to_string()))
        );

        // Check empty array key
        let empty_array_key = Value::Sequence(Vec::new());
        assert_eq!(
            map.get(&empty_array_key),
            Some(&Value::String("empty_array_key".to_string()))
        );
    } else {
        panic!("Result should be a mapping");
    }
}

#[test]
fn test_duplicate_complex_keys_error() {
    let yaml_content = r#"
? {name: John}
: first_value
? {name: John}
: second_value
"#;

    let yaml = Yaml::new();
    // This should either succeed with the second value overriding the first,
    // or fail with a duplicate key error, depending on implementation
    let result = yaml.load_str(yaml_content);

    if let Ok(Value::Mapping(map)) = result {
        // If parsing succeeds, should have only one entry (last one wins)
        assert_eq!(map.len(), 1);

        let complex_key = Value::Mapping({
            let mut key_map = IndexMap::new();
            key_map.insert(
                Value::String("name".to_string()),
                Value::String("John".to_string()),
            );
            key_map
        });
        // The value should be the second (last) one
        assert_eq!(
            map.get(&complex_key),
            Some(&Value::String("second_value".to_string()))
        );
    } else {
        // If it fails, that's also acceptable behavior for duplicate keys
        assert!(
            result.is_err(),
            "Should either succeed or fail, but got: {:?}",
            result
        );
    }
}
