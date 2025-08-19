#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::uninlined_format_args)]

use rust_yaml::Yaml;
use rust_yaml::scanner::BasicScanner;
use rust_yaml::value::IndentStyle;

#[test]
fn test_spaces_indentation_detection_2_spaces() {
    let yaml_content = r#"root:
  level1:
    level2: value
  back: to_root"#;

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    match scanner.detected_indent_style() {
        Some(IndentStyle::Spaces(2)) => {
            // Expected: 2-space indentation detected
        }
        other => panic!("Expected IndentStyle::Spaces(2), got {:?}", other),
    }
}

#[test]
fn test_spaces_indentation_detection_4_spaces() {
    let yaml_content = r#"root:
    level1:
        level2: value
    back: to_root"#;

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    match scanner.detected_indent_style() {
        Some(IndentStyle::Spaces(4)) => {
            // Expected: 4-space indentation detected
        }
        other => panic!("Expected IndentStyle::Spaces(4), got {:?}", other),
    }
}

#[test]
fn test_tabs_indentation_detection() {
    let yaml_content = "root:\n\tlevel1:\n\t\tlevel2: value\n\tback: to_root";

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    match scanner.detected_indent_style() {
        Some(IndentStyle::Tabs) => {
            // Expected: Tab indentation detected
        }
        other => panic!("Expected IndentStyle::Tabs, got {:?}", other),
    }
}

#[test]
fn test_mixed_indentation_error() {
    let yaml_content = "root:\n  level1: spaces\n\tlevel2: tab";

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_content);

    assert!(result.is_err(), "Should fail with mixed indentation");
    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("mixed indentation"),
        "Error should mention mixed indentation: {}",
        error_msg
    );
}

#[test]
fn test_no_indentation_style_flat_structure() {
    let yaml_content = r#"key1: value1
key2: value2
key3: value3"#;

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    assert!(
        scanner.detected_indent_style().is_none(),
        "No indentation style should be detected for flat structure"
    );
}

#[test]
fn test_indentation_style_with_comments() {
    let yaml_content = r#"# Top level comment
root:
  # Nested comment
  level1:
    level2: value  # End of line comment
  back: to_root"#;

    let scanner = BasicScanner::new_eager_with_comments(yaml_content.to_string());

    match scanner.detected_indent_style() {
        Some(IndentStyle::Spaces(2)) => {
            // Expected: 2-space indentation detected even with comments
        }
        other => panic!("Expected IndentStyle::Spaces(2), got {:?}", other),
    }
}

#[test]
fn test_complex_nested_indentation() {
    let yaml_content = r#"config:
  database:
    host: localhost
    port: 5432
    credentials:
      username: user
      password: pass
  cache:
    enabled: true
    settings:
      ttl: 3600
      max_size: 1000
server:
  host: 0.0.0.0
  port: 8080"#;

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    match scanner.detected_indent_style() {
        Some(IndentStyle::Spaces(2)) => {
            // Expected: 2-space indentation detected in complex structure
        }
        other => panic!("Expected IndentStyle::Spaces(2), got {:?}", other),
    }
}

#[test]
fn test_indentation_style_round_trip_preservation() {
    let original_yaml = r#"config:
    database:
        host: localhost
        credentials:
            user: admin
    server:
        port: 8080"#;

    let yaml = Yaml::new();

    // Parse original
    let value = yaml
        .load_str(original_yaml)
        .expect("Should parse successfully");

    // Serialize back
    let serialized = yaml
        .dump_str(&value)
        .expect("Should serialize successfully");

    // Parse serialized version to check indentation detection
    let scanner = BasicScanner::new_eager(serialized.to_string());

    // The serialized version should maintain consistent indentation
    match scanner.detected_indent_style() {
        Some(IndentStyle::Spaces(width)) => {
            assert!(
                *width == 2 || *width == 4,
                "Should use consistent space indentation, got {}",
                width
            );
        }
        other => {
            println!("Serialized YAML:\n{}", serialized);
            panic!("Should have detected space indentation, got {:?}", other);
        }
    }
}

#[test]
fn test_single_level_indentation() {
    let yaml_content = r#"root:
   child: value"#; // 3 spaces

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    // Should detect the indentation even with just one level
    match scanner.detected_indent_style() {
        Some(IndentStyle::Spaces(3)) => {
            // Expected: 3-space indentation detected
        }
        other => panic!("Expected IndentStyle::Spaces(3), got {:?}", other),
    }
}

#[test]
fn test_sequences_with_indentation() {
    let yaml_content = r#"items:
  - name: item1
    value: 100
  - name: item2
    value: 200"#;

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    match scanner.detected_indent_style() {
        Some(IndentStyle::Spaces(2)) => {
            // Expected: 2-space indentation detected with sequences
        }
        other => panic!("Expected IndentStyle::Spaces(2), got {:?}", other),
    }
}

#[test]
fn test_flow_style_no_indentation_detection() {
    let yaml_content = r#"{key1: value1, nested: {key2: value2}}"#;

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    assert!(
        scanner.detected_indent_style().is_none(),
        "Flow style should not affect indentation detection"
    );
}

#[test]
fn test_mixed_block_and_flow_indentation() {
    let yaml_content = r#"config:
  database: {host: localhost, port: 5432}
  cache:
    enabled: true
    settings: [ttl, max_size]"#;

    let scanner = BasicScanner::new_eager(yaml_content.to_string());

    match scanner.detected_indent_style() {
        Some(IndentStyle::Spaces(2)) => {
            // Expected: 2-space indentation detected from block portions
        }
        other => panic!("Expected IndentStyle::Spaces(2), got {:?}", other),
    }
}
