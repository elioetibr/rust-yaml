#![allow(clippy::needless_raw_string_hashes)]

use rust_yaml::{Value, Yaml};

#[test]
fn test_merge_keys_basic() {
    let yaml_content = r#"
base: &base
  name: test
  count: 42

derived:
  <<: *base
  active: true
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        let derived = map
            .get(&Value::String("derived".to_string()))
            .expect("derived should exist");

        if let Value::Mapping(derived_map) = derived {
            // Verify inherited values
            assert_eq!(
                derived_map.get(&Value::String("name".to_string())),
                Some(&Value::String("test".to_string()))
            );
            assert_eq!(
                derived_map.get(&Value::String("count".to_string())),
                Some(&Value::Int(42))
            );
            // Verify explicit value
            assert_eq!(
                derived_map.get(&Value::String("active".to_string())),
                Some(&Value::Bool(true))
            );
        } else {
            panic!("derived should be a mapping");
        }
    } else {
        panic!("result should be a mapping");
    }
}

#[test]
fn test_merge_keys_override() {
    let yaml_content = r#"
base: &base
  name: base_name
  count: 1
  active: false

derived:
  <<: *base
  name: derived_name  # Should override base
  new_field: extra
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        let derived = map
            .get(&Value::String("derived".to_string()))
            .expect("derived should exist");

        if let Value::Mapping(derived_map) = derived {
            // Verify overridden value
            assert_eq!(
                derived_map.get(&Value::String("name".to_string())),
                Some(&Value::String("derived_name".to_string()))
            );
            // Verify inherited values
            assert_eq!(
                derived_map.get(&Value::String("count".to_string())),
                Some(&Value::Int(1))
            );
            assert_eq!(
                derived_map.get(&Value::String("active".to_string())),
                Some(&Value::Bool(false))
            );
            // Verify new field
            assert_eq!(
                derived_map.get(&Value::String("new_field".to_string())),
                Some(&Value::String("extra".to_string()))
            );
        } else {
            panic!("derived should be a mapping");
        }
    } else {
        panic!("result should be a mapping");
    }
}

#[test]
fn test_merge_keys_multiple_sources() {
    let yaml_content = r#"
base1: &base1
  name: from_base1
  field1: value1

base2: &base2
  field2: value2
  field3: value3

derived:
  <<: [*base1, *base2]
  name: overridden
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        let derived = map
            .get(&Value::String("derived".to_string()))
            .expect("derived should exist");

        if let Value::Mapping(derived_map) = derived {
            // Verify overridden value takes precedence
            assert_eq!(
                derived_map.get(&Value::String("name".to_string())),
                Some(&Value::String("overridden".to_string()))
            );
            // Verify values from both bases
            assert_eq!(
                derived_map.get(&Value::String("field1".to_string())),
                Some(&Value::String("value1".to_string()))
            );
            assert_eq!(
                derived_map.get(&Value::String("field2".to_string())),
                Some(&Value::String("value2".to_string()))
            );
            assert_eq!(
                derived_map.get(&Value::String("field3".to_string())),
                Some(&Value::String("value3".to_string()))
            );
        } else {
            panic!("derived should be a mapping");
        }
    } else {
        panic!("result should be a mapping");
    }
}

#[test]
fn test_merge_keys_nested() {
    let yaml_content = r#"
base: &base
  timeout: 30
  retries: 3

service:
  name: test_service
  settings:
    <<: *base
    port: 8080
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        let service = map
            .get(&Value::String("service".to_string()))
            .expect("service should exist");

        if let Value::Mapping(service_map) = service {
            let settings = service_map
                .get(&Value::String("settings".to_string()))
                .expect("settings should exist");

            if let Value::Mapping(settings_map) = settings {
                // Should inherit the base values directly
                assert_eq!(
                    settings_map.get(&Value::String("timeout".to_string())),
                    Some(&Value::Int(30))
                );
                assert_eq!(
                    settings_map.get(&Value::String("retries".to_string())),
                    Some(&Value::Int(3))
                );
                // Should also have explicit value
                assert_eq!(
                    settings_map.get(&Value::String("port".to_string())),
                    Some(&Value::Int(8080))
                );
            } else {
                panic!("settings should be a mapping");
            }
        } else {
            panic!("service should be a mapping");
        }
    } else {
        panic!("result should be a mapping");
    }
}

#[test]
fn test_merge_keys_with_sequences() {
    let yaml_content = r#"
defaults: &defaults
  timeout: 30

config:
  <<: *defaults
  items:  # Explicit sequence value
    - custom1
    - custom2
    - custom3
"#;

    let yaml = Yaml::new();
    let result = yaml
        .load_str(yaml_content)
        .expect("Should parse successfully");

    if let Value::Mapping(ref map) = result {
        let config = map
            .get(&Value::String("config".to_string()))
            .expect("config should exist");

        if let Value::Mapping(config_map) = config {
            // Should inherit timeout
            assert_eq!(
                config_map.get(&Value::String("timeout".to_string())),
                Some(&Value::Int(30))
            );

            // Should have overridden sequence
            if let Some(Value::Sequence(items)) =
                config_map.get(&Value::String("items".to_string()))
            {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Value::String("custom1".to_string()));
                assert_eq!(items[1], Value::String("custom2".to_string()));
                assert_eq!(items[2], Value::String("custom3".to_string()));
            } else {
                panic!("items should be a sequence");
            }
        } else {
            panic!("config should be a mapping");
        }
    } else {
        panic!("result should be a mapping");
    }
}

#[test]
fn test_merge_keys_error_cases() {
    // Test merge with non-mapping value (should error)
    let yaml_content = r#"
not_mapping: &ref "just a string"

test:
  <<: *ref
"#;

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_content);

    assert!(
        result.is_err(),
        "Should fail when merging non-mapping value"
    );
    if let Err(e) = result {
        assert!(
            e.to_string().contains("must be a mapping"),
            "Error should mention mapping requirement"
        );
    }
}

#[test]
fn test_merge_keys_round_trip() {
    let yaml_content = r#"base: &base
  key: value
  number: 42

derived:
  <<: *base
  extra: field
"#;

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

    // Should be equivalent (though structure might differ due to merge resolution)
    if let (Value::Mapping(orig), Value::Mapping(rt)) = (&parsed, &round_trip) {
        // Check that derived mapping has the same effective values
        let orig_derived = orig
            .get(&Value::String("derived".to_string()))
            .expect("derived should exist in original");
        let rt_derived = rt
            .get(&Value::String("derived".to_string()))
            .expect("derived should exist in round-trip");

        if let (Value::Mapping(orig_map), Value::Mapping(rt_map)) = (orig_derived, rt_derived) {
            // All merged values should be present in both
            assert_eq!(
                orig_map.get(&Value::String("key".to_string())),
                rt_map.get(&Value::String("key".to_string()))
            );
            assert_eq!(
                orig_map.get(&Value::String("number".to_string())),
                rt_map.get(&Value::String("number".to_string()))
            );
            assert_eq!(
                orig_map.get(&Value::String("extra".to_string())),
                rt_map.get(&Value::String("extra".to_string()))
            );
        }
    }
}
