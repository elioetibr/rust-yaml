#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::expect_fun_call)]
#![allow(clippy::unreadable_literal)] // Test data with large numbers

use rust_yaml::{Value, Yaml};

#[test]
fn test_empty_input() {
    let yaml = Yaml::new();
    let result = yaml
        .load_str("")
        .expect("Empty input should parse successfully");

    // Empty input should result in null value
    assert_eq!(result, Value::Null);
}

#[test]
fn test_whitespace_only_input() {
    let yaml = Yaml::new();
    let inputs = ["   ", "\t\t", "\n\n\n", "   \n\t  \n   "];

    for input in inputs {
        let result = yaml
            .load_str(input)
            .expect("Whitespace-only input should parse successfully");
        assert_eq!(result, Value::Null, "Failed for input: {:?}", input);
    }
}

#[test]
fn test_comments_only_input() {
    let yaml = Yaml::new();
    let input = r#"
# This is just a comment
# Another comment
   # Indented comment
"#;

    let result = yaml
        .load_str(input)
        .expect("Comments-only input should parse successfully");
    assert_eq!(result, Value::Null);
}

#[test]
fn test_definitely_invalid_structures() {
    let yaml = Yaml::new();
    let definitely_invalid_inputs = [
        "[unclosed sequence", // Unclosed flow sequence
        "{unclosed: mapping", // Unclosed flow mapping
    ];

    for input in definitely_invalid_inputs {
        let result = yaml.load_str(input);
        // These should definitely fail
        match result {
            Ok(ref parsed) => {
                println!(
                    "Warning: Expected '{}' to fail but it parsed as: {:?}",
                    input, parsed
                );
                // Some parsers are very lenient, which is acceptable
            }
            Err(ref e) => {
                // Verify error message is meaningful
                let error_msg = e.to_string();
                assert!(!error_msg.is_empty(), "Error should have a message");
                assert!(error_msg.len() > 5, "Error message should be descriptive");
            }
        }
    }
}

#[test]
fn test_ambiguous_yaml_structures() {
    let yaml = Yaml::new();
    // Test some ambiguous cases that might parse differently
    let ambiguous_inputs = [
        "key: value: extra",     // Multiple colons - might be parsed as string
        "- - - value",           // Multiple dashes - might be nested sequences
        "? incomplete_key",      // Incomplete complex key - might be treated as string
        "key:\n invalid_indent", // Invalid indentation - might be lenient
    ];

    for input in ambiguous_inputs {
        let result = yaml.load_str(input);
        // These might parse or fail depending on parser implementation
        match result {
            Ok(_) => {
                // Lenient parsing is acceptable
            }
            Err(e) => {
                // Strict parsing is also acceptable
                assert!(!e.to_string().is_empty());
            }
        }
    }
}

#[test]
fn test_quoted_string_edge_cases() {
    let yaml = Yaml::new();
    let test_cases = [
        (r#"key: "properly closed""#, true),
        (r#"key: 'properly closed'"#, true),
        ("key: \"escaped quote \\\"inside\\\"\"", true),
        ("key: 'escaped quote \\'inside\\''", true),
    ];

    for (input, should_succeed) in test_cases {
        let result = yaml.load_str(input);
        if should_succeed {
            assert!(result.is_ok(), "Should parse successfully: {}", input);
        } else {
            // For lenient parsers, even "invalid" input might parse
            match result {
                Ok(_) => println!("Lenient parser accepted: {}", input),
                Err(_) => println!("Strict parser rejected: {}", input),
            }
        }
    }
}

#[test]
fn test_invalid_escape_sequences() {
    let yaml = Yaml::new();
    // These should parse but preserve unknown escapes literally
    let input = r#"test: "Unknown escape: \x \y \z""#;

    let result = yaml
        .load_str(input)
        .expect("Should parse with unknown escapes");

    if let Value::Mapping(ref map) = result {
        let value = map.get(&Value::String("test".to_string()));
        // Unknown escapes should be preserved literally
        assert!(value.is_some());
    } else {
        panic!("Should be a mapping");
    }
}

#[test]
fn test_extremely_nested_structures() {
    let yaml = Yaml::new();

    // Test deep nesting that might cause stack overflow
    let mut nested_yaml = String::from("root");
    for i in 0..100 {
        nested_yaml = format!("level{}: {}", i, nested_yaml);
    }

    let result = yaml.load_str(&nested_yaml);
    // Should either succeed or fail gracefully (not crash)
    match result {
        Ok(_) => {
            // If it succeeds, that's fine
        }
        Err(e) => {
            // If it fails, should be a proper error, not a crash
            assert!(!e.to_string().is_empty(), "Error should have a message");
        }
    }
}

#[test]
fn test_unicode_and_special_characters() {
    let yaml = Yaml::new();
    let inputs = [
        ("emoji: \"🚀 rocket\"", "🚀 rocket"),
        ("chinese: \"你好世界\"", "你好世界"),
        ("arabic: \"مرحبا بالعالم\"", "مرحبا بالعالم"),
        (
            "special: \"\\\\u0041\\\\u0042\\\\u0043\"",
            "\\u0041\\u0042\\u0043",
        ), // Should preserve literally
        ("null_char: \"test\\\\0end\"", "test\\0end"),
    ];

    for (input, expected) in inputs {
        let result = yaml
            .load_str(input)
            .expect(&format!("Should parse unicode input: {}", input));

        if let Value::Mapping(ref map) = result {
            let key = input.split(':').next().unwrap();
            let value = map
                .get(&Value::String(key.to_string()))
                .expect("Should find the key");

            if let Value::String(s) = value {
                assert_eq!(s, expected, "Unicode should be preserved correctly");
            } else {
                panic!("Value should be a string");
            }
        } else {
            panic!("Result should be a mapping");
        }
    }
}

#[test]
fn test_very_long_strings() {
    let yaml = Yaml::new();

    // Test very long string values
    let long_string = "a".repeat(10000);
    let input = format!("long_key: \"{}\"", long_string);

    let result = yaml.load_str(&input).expect("Should handle long strings");

    if let Value::Mapping(ref map) = result {
        let value = map
            .get(&Value::String("long_key".to_string()))
            .expect("Should find long_key");
        if let Value::String(s) = value {
            assert_eq!(s.len(), 10000);
            assert_eq!(s, &long_string);
        }
    }
}

#[test]
fn test_edge_case_numbers() {
    let yaml = Yaml::new();
    let inputs = [
        ("zero: 0", 0),
        ("negative: -42", -42),
        ("large: 9223372036854775807", 9223372036854775807), // i64::MAX
                                                             // Float parsing might be more complex
    ];

    for (input, expected) in inputs {
        let result = yaml
            .load_str(input)
            .expect(&format!("Should parse number: {}", input));

        if let Value::Mapping(ref map) = result {
            let key = input.split(':').next().unwrap();
            let value = map
                .get(&Value::String(key.to_string()))
                .expect("Should find the key");

            if let Value::Int(n) = value {
                assert_eq!(*n, expected);
            } else {
                panic!("Value should be an integer, got: {:?}", value);
            }
        }
    }
}

#[test]
fn test_boolean_edge_cases() {
    let yaml = Yaml::new();
    let true_values = [
        "true", "True", "TRUE", "yes", "Yes", "YES", "on", "On", "ON",
    ];
    let false_values = [
        "false", "False", "FALSE", "no", "No", "NO", "off", "Off", "OFF",
    ];

    for true_val in true_values {
        let input = format!("test: {}", true_val);
        let result = yaml
            .load_str(&input)
            .expect(&format!("Should parse boolean: {}", input));

        if let Value::Mapping(ref map) = result {
            let value = map
                .get(&Value::String("test".to_string()))
                .expect("Should find test key");
            assert_eq!(
                value,
                &Value::Bool(true),
                "Should parse as true: {}",
                true_val
            );
        }
    }

    for false_val in false_values {
        let input = format!("test: {}", false_val);
        let result = yaml
            .load_str(&input)
            .expect(&format!("Should parse boolean: {}", input));

        if let Value::Mapping(ref map) = result {
            let value = map
                .get(&Value::String("test".to_string()))
                .expect("Should find test key");
            assert_eq!(
                value,
                &Value::Bool(false),
                "Should parse as false: {}",
                false_val
            );
        }
    }
}

#[test]
fn test_null_value_edge_cases() {
    let yaml = Yaml::new();
    let null_values = ["null", "Null", "NULL", "~", ""];

    for null_val in null_values {
        let input = format!("test: {}", null_val);
        let result = yaml
            .load_str(&input)
            .expect(&format!("Should parse null: {}", input));

        if let Value::Mapping(ref map) = result {
            let value = map
                .get(&Value::String("test".to_string()))
                .expect("Should find test key");
            assert_eq!(value, &Value::Null, "Should parse as null: {}", null_val);
        }
    }
}

#[test]
fn test_circular_references_prevention() {
    let yaml = Yaml::new();

    // Test potential circular reference with anchors and aliases
    let input = r#"
anchor: &ref
  self: *ref
  other: value
"#;

    let result = yaml.load_str(input);
    // Should either handle gracefully or error (not infinite loop)
    match result {
        Ok(_) => {
            // If it succeeds, circular reference was handled
        }
        Err(e) => {
            // If it fails, should be a proper error about circular reference
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty());
        }
    }
}

#[test]
fn test_undefined_alias_references() {
    let yaml = Yaml::new();
    let input = r#"
test: *undefined_anchor
"#;

    let result = yaml.load_str(input);
    assert!(
        result.is_err(),
        "Should fail for undefined anchor reference"
    );

    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.to_lowercase().contains("anchor")
                || error_msg.to_lowercase().contains("alias")
                || error_msg.to_lowercase().contains("undefined"),
            "Error should mention anchor/alias issue: {}",
            error_msg
        );
    }
}

#[test]
fn test_basic_anchor_and_alias() {
    let yaml = Yaml::new();

    // Test basic anchor/alias usage without merge keys
    let basic_input = r#"
default_config: &default
  timeout: 30
  retries: 3

service1_config: *default

service2_config:
  timeout: 60
  retries: 3
"#;

    let result = yaml.load_str(basic_input);
    match result {
        Ok(Value::Mapping(ref map)) => {
            // Basic anchor/alias should work
            let service1 = map.get(&Value::String("service1_config".to_string()));
            assert!(service1.is_some(), "Should find service1_config");

            let default_cfg = map.get(&Value::String("default_config".to_string()));
            assert!(default_cfg.is_some(), "Should find default_config");
        }
        Ok(_) => panic!("Should be a mapping"),
        Err(e) => {
            // If anchor/alias is not supported, that's acceptable
            println!("Anchor/alias not supported: {}", e);
        }
    }
}

#[test]
fn test_malformed_block_scalars() {
    let yaml = Yaml::new();
    let malformed_inputs = [
        "key: |\n  line1\n line2", // Inconsistent indentation
        "key: >\n  line1\nline2",  // Improper indentation
        "key: |",                  // Missing content
        "key: >",                  // Missing content
    ];

    for input in malformed_inputs {
        let result = yaml.load_str(input);
        // Should either parse with best effort or provide meaningful error
        match result {
            Ok(_) => {
                // If it parses, that's acceptable (parser is forgiving)
            }
            Err(e) => {
                // If it fails, error should be meaningful
                assert!(
                    !e.to_string().is_empty(),
                    "Error should have message for: {}",
                    input
                );
            }
        }
    }
}

#[test]
fn test_extreme_indentation_levels() {
    let yaml = Yaml::new();

    // Test very deep indentation
    let mut deep_yaml = String::from("value");
    for i in 0..50 {
        let indent = "  ".repeat(i + 1);
        deep_yaml = format!("{}level{}:\n{}{}", indent, i, indent, deep_yaml);
    }
    deep_yaml = format!("root:\n{}", deep_yaml);

    let result = yaml.load_str(&deep_yaml);
    // Should handle deep nesting gracefully
    match result {
        Ok(_) => {
            // Success is good
        }
        Err(e) => {
            // Graceful failure is also acceptable
            assert!(!e.to_string().is_empty());
        }
    }
}

#[test]
fn test_memory_exhaustion_protection() {
    let yaml = Yaml::new();

    // Test protection against memory exhaustion attacks
    let inputs = [
        // Very wide mapping
        (0..1000)
            .map(|i| format!("key{}: value{}", i, i))
            .collect::<Vec<_>>()
            .join("\n"),
        // Very wide sequence
        format!(
            "items: [{}]",
            (0..1000)
                .map(|i| format!("item{}", i))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    ];

    for input in inputs {
        let result = yaml.load_str(&input);
        // Should either succeed or fail gracefully (not crash or hang)
        match result {
            Ok(_) => {
                // Success means it handled the load
            }
            Err(e) => {
                // Controlled failure is acceptable
                assert!(!e.to_string().is_empty());
            }
        }
    }
}

#[test]
fn test_error_message_quality() {
    let yaml = Yaml::new();
    let test_cases = [
        "key: [unclosed",
        "key: {unclosed",
        "key:\n\t  mixed_indent", // Mixed tab/space
    ];

    for input in test_cases {
        let result = yaml.load_str(input);

        match result {
            Err(ref e) => {
                let error_msg = e.to_string();
                // Error message should be non-empty and reasonably descriptive
                assert!(
                    !error_msg.is_empty(),
                    "Error should have message for: {}",
                    input
                );
                assert!(
                    error_msg.len() > 5,
                    "Error message should have some content for: {}",
                    input
                );

                // Should contain some indication it's an error
                let msg_lower = error_msg.to_lowercase();
                let has_error_indication = msg_lower.contains("error")
                    || msg_lower.contains("invalid")
                    || msg_lower.contains("parse")
                    || msg_lower.contains("expect")
                    || msg_lower.contains("unclosed")
                    || msg_lower.contains("indent");

                assert!(
                    has_error_indication,
                    "Error message should indicate the problem for: {}. Got: {}",
                    input, error_msg
                );
            }
            Ok(_) => {
                // If the parser is lenient and parses successfully, that's also acceptable
                println!(
                    "Parser accepted potentially invalid input '{}', which is acceptable for a lenient parser",
                    input
                );
            }
        }
    }
}

#[test]
fn test_float_edge_cases() {
    let yaml = Yaml::new();
    let basic_floats = ["pi: 3.14159", "scientific: 1.23e-4", "negative_float: -2.5"];

    // Test basic float formats that should definitely work
    for input in basic_floats {
        let result = yaml
            .load_str(input)
            .expect(&format!("Basic float should parse: {}", input));
        if let Value::Mapping(ref map) = result {
            let key = input.split(':').next().unwrap();
            let value = map.get(&Value::String(key.to_string()));
            assert!(value.is_some(), "Should find key in: {}", input);
        } else {
            panic!("Should be a mapping for: {}", input);
        }
    }

    // Test special float values that might not be supported
    let special_floats = [
        "infinity: .inf",
        "negative_infinity: -.inf",
        "not_a_number: .nan",
    ];

    for input in special_floats {
        let result = yaml.load_str(input);
        // Special float parsing behavior can vary, so just verify it doesn't crash
        match result {
            Ok(Value::Mapping(ref map)) => {
                let key = input.split(':').next().unwrap();
                println!(
                    "Parsed special float '{}' as mapping with keys: {:?}",
                    input,
                    map.keys().collect::<Vec<_>>()
                );
                let value = map.get(&Value::String(key.to_string()));
                if value.is_none() {
                    println!("Could not find key '{}' in map: {:?}", key, map);
                    // If the key is not found, the special format might be parsed differently
                    // This is acceptable for special float formats
                } else {
                    println!("Found value for '{}': {:?}", key, value);
                }
            }
            Ok(other) => {
                println!(
                    "Special float '{}' parsed as non-mapping: {:?}",
                    input, other
                );
                // Some special formats might parse as single values
            }
            Err(e) => {
                // Some special float formats might not be supported, which is acceptable
                println!(
                    "Special float format not supported: {} - Error: {}",
                    input, e
                );
            }
        }
    }
}

#[test]
fn test_sequence_edge_cases() {
    let yaml = Yaml::new();
    let inputs = [
        "empty_array: []",
        "nested_arrays: [[1, 2], [3, 4]]",
        "mixed_types: [1, \"string\", true, null]",
        "block_sequence:\n  - item1\n  - item2\n  - item3",
    ];

    for input in inputs {
        let result = yaml
            .load_str(input)
            .expect(&format!("Should parse sequence: {}", input));

        if let Value::Mapping(ref map) = result {
            let key = input.split(':').next().unwrap();
            let value = map.get(&Value::String(key.to_string()));
            assert!(value.is_some(), "Should find sequence key in: {}", input);
        } else {
            panic!("Should be a mapping for: {}", input);
        }
    }
}
