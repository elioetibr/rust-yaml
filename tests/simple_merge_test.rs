#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_raw_string_hashes)]

use rust_yaml::Yaml;

#[test]
fn test_alias_resolution() {
    let yaml_content = r#"
base: &base
  key: value

ref: *base
"#;

    let yaml = Yaml::new();
    match yaml.load_str(yaml_content) {
        Ok(result) => {
            println!("Alias resolution works: {:?}", result);
        }
        Err(e) => {
            panic!("Alias resolution failed: {}", e);
        }
    }
}

#[test]
fn test_merge_keys_basic() {
    let yaml_content = r#"
base: &base
  key: value

test:
  <<: *base
"#;

    let yaml = Yaml::new();
    match yaml.load_str(yaml_content) {
        Ok(result) => {
            println!("✅ Merge keys work: {:?}", result);

            // Verify the structure is correct
            if let rust_yaml::Value::Mapping(ref map) = result {
                let base_value = map
                    .get(&rust_yaml::Value::String("base".to_string()))
                    .expect("base should exist");
                let test_value = map
                    .get(&rust_yaml::Value::String("test".to_string()))
                    .expect("test should exist");

                // Both should be mappings with the same content
                if let (rust_yaml::Value::Mapping(base_map), rust_yaml::Value::Mapping(test_map)) =
                    (base_value, test_value)
                {
                    assert_eq!(
                        base_map.get(&rust_yaml::Value::String("key".to_string())),
                        Some(&rust_yaml::Value::String("value".to_string()))
                    );
                    assert_eq!(
                        test_map.get(&rust_yaml::Value::String("key".to_string())),
                        Some(&rust_yaml::Value::String("value".to_string()))
                    );
                    println!("✅ Merge keys properly inherited values from base");
                } else {
                    panic!("Both base and test should be mappings");
                }
            } else {
                panic!("Result should be a mapping");
            }
        }
        Err(e) => {
            panic!("Merge keys should work now, but got error: {}", e);
        }
    }
}
