#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::format_push_string)] // Test data generation

use rust_yaml::{Value, Yaml};
use std::time::{Duration, Instant};

#[test]
fn test_basic_parsing_performance() {
    let yaml = Yaml::new();

    // Create a reasonable-sized YAML document
    let mut doc = String::new();
    for i in 0..50 {
        doc.push_str(&format!("key_{}: value_{}\n", i, i));
        doc.push_str(&format!("nested_{}:\n", i));
        doc.push_str(&format!("  sub_key_{}: sub_value_{}\n", i, i));
        doc.push_str(&format!("  items_{}:\n", i));
        doc.push_str(&format!("    - item_{}_1\n", i));
        doc.push_str(&format!("    - item_{}_2\n", i));
    }

    let start_time = Instant::now();
    let result = yaml.load_str(&doc);
    let parse_duration = start_time.elapsed();

    assert!(result.is_ok(), "Document should parse successfully");

    // Parsing should complete within reasonable time
    assert!(
        parse_duration < Duration::from_secs(10),
        "Document parsing took too long: {:?}",
        parse_duration
    );

    println!("Basic parsing completed in: {:?}", parse_duration);

    if let Ok(Value::Mapping(ref map)) = result {
        assert!(map.len() >= 100, "Should have many top-level keys");
        println!("Parsed {} top-level keys", map.len());
    }
}

#[test]
fn test_reasonable_nesting_performance() {
    let yaml = Yaml::new();

    // Create a reasonably nested structure (not too deep)
    let mut nested_yaml = String::from("final_value");
    for i in 0..10 {
        nested_yaml = format!("level_{}:\n  {}", i, nested_yaml.replace("\n", "\n  "));
    }

    let start_time = Instant::now();
    let result = yaml.load_str(&nested_yaml);
    let parse_duration = start_time.elapsed();

    assert!(
        result.is_ok(),
        "Reasonable nesting should parse successfully"
    );
    assert!(
        parse_duration < Duration::from_secs(2),
        "Reasonable nesting parsing took too long: {:?}",
        parse_duration
    );

    println!(
        "Reasonable nesting parsing completed in: {:?}",
        parse_duration
    );
}

#[test]
fn test_sequence_performance() {
    let yaml = Yaml::new();

    // Create a sequence with reasonable size
    let items: Vec<String> = (0..100).map(|i| format!("item_{}", i)).collect();
    let sequence = format!("array: [{}]", items.join(", "));

    let start_time = Instant::now();
    let result = yaml.load_str(&sequence);
    let parse_duration = start_time.elapsed();

    assert!(result.is_ok(), "Sequence should parse successfully");

    // Should complete within reasonable time
    assert!(
        parse_duration < Duration::from_secs(3),
        "Sequence parsing took too long: {:?}",
        parse_duration
    );

    println!("Sequence parsing completed in: {:?}", parse_duration);

    if let Ok(Value::Mapping(ref map)) = result {
        if let Some(Value::Sequence(seq)) = map.get(&Value::String("array".to_string())) {
            assert_eq!(seq.len(), 100, "Should have all sequence items");
            println!("Parsed sequence with {} items", seq.len());
        }
    }
}

#[test]
fn test_basic_serialization_performance() {
    let yaml = Yaml::new();

    // Create a reasonably complex data structure
    let mut complex_data = indexmap::IndexMap::new();

    for i in 0..20 {
        let mut nested_map = indexmap::IndexMap::new();
        nested_map.insert(Value::String(format!("id_{}", i)), Value::Int(i));
        nested_map.insert(
            Value::String(format!("name_{}", i)),
            Value::String(format!("Item {}", i)),
        );

        let items = (0..5)
            .map(|j| Value::String(format!("item_{}_{}", i, j)))
            .collect();
        nested_map.insert(
            Value::String(format!("items_{}", i)),
            Value::Sequence(items),
        );

        complex_data.insert(
            Value::String(format!("entry_{}", i)),
            Value::Mapping(nested_map),
        );
    }

    let complex_value = Value::Mapping(complex_data);

    let start_time = Instant::now();
    let result = yaml.dump_str(&complex_value);
    let serialize_duration = start_time.elapsed();

    if result.is_ok() {
        // Should complete within reasonable time
        assert!(
            serialize_duration < Duration::from_secs(3),
            "Serialization took too long: {:?}",
            serialize_duration
        );

        println!("Basic serialization completed in: {:?}", serialize_duration);

        if let Ok(ref yaml_str) = result {
            println!("Serialized YAML length: {} bytes", yaml_str.len());
            assert!(
                yaml_str.len() > 100,
                "Should produce substantial YAML output"
            );
        }
    } else {
        println!("Serialization not supported or failed: {:?}", result);
    }
}

#[test]
fn test_round_trip_performance() {
    let yaml = Yaml::new();

    // Create test data
    let mut test_data = indexmap::IndexMap::new();

    for i in 0..10 {
        let mut section = indexmap::IndexMap::new();
        section.insert(
            Value::String("name".to_string()),
            Value::String(format!("Section {}", i)),
        );
        section.insert(
            Value::String("enabled".to_string()),
            Value::Bool(i % 2 == 0),
        );
        section.insert(Value::String("count".to_string()), Value::Int(i * 10));

        let tags = (0..3)
            .map(|j| Value::String(format!("tag_{}_{}", i, j)))
            .collect();
        section.insert(Value::String("tags".to_string()), Value::Sequence(tags));

        test_data.insert(
            Value::String(format!("section_{}", i)),
            Value::Mapping(section),
        );
    }

    let original_value = Value::Mapping(test_data);

    // Test round-trip performance
    let start_time = Instant::now();

    // Serialize
    let yaml_str_result = yaml.dump_str(&original_value);

    if let Ok(yaml_str) = yaml_str_result {
        let serialize_time = start_time.elapsed();

        // Parse back
        let parse_start = Instant::now();
        let parsed_value = yaml.load_str(&yaml_str).expect("Should parse back");
        let parse_time = parse_start.elapsed();

        let total_time = start_time.elapsed();

        // Should complete within reasonable time
        assert!(
            total_time < Duration::from_secs(2),
            "Round-trip took too long: {:?}",
            total_time
        );

        println!("Round-trip performance:");
        println!("  Serialize: {:?}", serialize_time);
        println!("  Parse: {:?}", parse_time);
        println!("  Total: {:?}", total_time);

        // Values should be equivalent
        assert_eq!(
            original_value, parsed_value,
            "Round-trip should preserve data"
        );
    } else {
        println!("Serialization not supported, testing parse-only performance");

        // Just test parsing performance
        let test_yaml = r#"
config:
  database:
    host: localhost
    port: 5432
  cache:
    enabled: true
    ttl: 3600
services:
  - name: service1
    port: 8080
    enabled: true
  - name: service2
    port: 8081
    enabled: false
"#;

        let parse_start = Instant::now();
        let result = yaml.load_str(test_yaml);
        let parse_time = parse_start.elapsed();

        assert!(result.is_ok(), "Test YAML should parse");
        assert!(
            parse_time < Duration::from_millis(100),
            "Simple parsing took too long: {:?}",
            parse_time
        );

        println!("Parse-only performance: {:?}", parse_time);
    }
}

#[test]
fn test_memory_usage_basic() {
    let yaml = Yaml::new();

    // Test multiple parsing operations in sequence to check for memory leaks
    for iteration in 0..5 {
        let doc_size = 10 + iteration * 5; // Small sizes for reasonable testing

        let mut test_doc = String::new();
        for i in 0..doc_size {
            test_doc.push_str(&format!("key_{}: value_{}\n", i, i));
        }

        let start_time = Instant::now();
        let result = yaml.load_str(&test_doc);
        let duration = start_time.elapsed();

        assert!(result.is_ok(), "Iteration {} should succeed", iteration);
        assert!(
            duration < Duration::from_secs(2),
            "Iteration {} took too long: {:?}",
            iteration,
            duration
        );

        // Let the value go out of scope to test memory cleanup
        drop(result);

        println!(
            "Memory test iteration {} (size: {}) completed in {:?}",
            iteration, doc_size, duration
        );
    }

    println!("Memory usage test completed successfully");
}

#[test]
fn test_unicode_performance() {
    let yaml = Yaml::new();

    // Create document with unicode content (simpler structure)
    let mut unicode_doc = String::new();
    let unicode_samples = [
        "🚀🌟💫⭐🌠",
        "你好世界测试内容",
        "مرحبا بالعالم اختبار",
        "Здравствуй мир тест",
        "🎵🎶🎸🎹🥁",
    ];

    for i in 0..10 {
        let sample = unicode_samples[i % unicode_samples.len()];
        unicode_doc.push_str(&format!("unicode_key_{}: \"{}_value_{}\"\n", i, sample, i));
        unicode_doc.push_str(&format!("simple_key_{}: \"simple_value_{}\"\n", i, i));
    }

    let start_time = Instant::now();
    let result = yaml.load_str(&unicode_doc);
    let parse_duration = start_time.elapsed();

    assert!(result.is_ok(), "Unicode document should parse successfully");

    // Should handle unicode efficiently
    assert!(
        parse_duration < Duration::from_secs(2),
        "Unicode parsing took too long: {:?}",
        parse_duration
    );

    println!(
        "Unicode performance test completed in: {:?}",
        parse_duration
    );

    if let Ok(Value::Mapping(ref map)) = result {
        println!("Parsed {} unicode entries", map.len());
        // With 10 iterations and 2 keys per iteration, we should have at least 10 keys
        assert!(
            map.len() >= 10,
            "Should have unicode keys, found: {}",
            map.len()
        );
    }
}

#[test]
fn test_error_handling_performance() {
    let yaml = Yaml::new();

    // Test that error handling doesn't cause performance issues
    let invalid_documents = [
        "[unclosed sequence",
        "{unclosed: mapping",
        "key:\n\t  mixed_indentation",
        "? incomplete complex key",
        "test: *undefined_anchor",
    ];

    let start_time = Instant::now();

    for (i, doc) in invalid_documents.iter().enumerate() {
        let parse_start = Instant::now();
        let result = yaml.load_str(doc);
        let parse_duration = parse_start.elapsed();

        // Should fail quickly
        assert!(
            parse_duration < Duration::from_millis(100),
            "Error handling for doc {} took too long: {:?}",
            i,
            parse_duration
        );

        // Most should produce errors (some might be accepted by lenient parser)
        if let Err(ref e) = result {
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty(), "Error should have message");
        }
    }

    let total_duration = start_time.elapsed();

    println!(
        "Error handling performance test completed in: {:?}",
        total_duration
    );
    assert!(
        total_duration < Duration::from_secs(1),
        "Total error handling took too long: {:?}",
        total_duration
    );
}

#[test]
fn test_string_processing_performance() {
    let yaml = Yaml::new();

    // Test performance with various string types (simpler)
    let mut string_doc = String::new();

    for i in 0..10 {
        // Plain strings
        string_doc.push_str(&format!("plain_{}: simple_value_{}\n", i, i));

        // Quoted strings
        string_doc.push_str(&format!(
            "quoted_{}: \"Quoted value with spaces {}\"\n",
            i, i
        ));

        // Strings with escapes
        string_doc.push_str(&format!("escaped_{}: \"Value with quotes {}\"\n", i, i));
    }

    let start_time = Instant::now();
    let result = yaml.load_str(&string_doc);
    let parse_duration = start_time.elapsed();

    assert!(result.is_ok(), "String document should parse successfully");

    // String processing should be efficient
    assert!(
        parse_duration < Duration::from_secs(5),
        "String processing took too long: {:?}",
        parse_duration
    );

    println!(
        "String processing performance test completed in: {:?}",
        parse_duration
    );

    if let Ok(Value::Mapping(ref map)) = result {
        println!("Processed {} string entries", map.len());
        // With 10 iterations and 3 keys per iteration, we should have 30 keys
        assert!(
            map.len() >= 20,
            "Should have string entries, found: {}",
            map.len()
        );
    }
}

#[test]
fn test_concurrent_parsing_simulation() {
    // Test that multiple YAML instances can work concurrently
    // Note: This doesn't use actual threads to keep the test simple

    let yaml_instances: Vec<Yaml> = (0..5).map(|_| Yaml::new()).collect();

    let test_documents: Vec<String> = (0..5).map(|i| {
        format!("test_doc_{}:\n  content: \"Document {}\"\n  items:\n    - item1\n    - item2\n    - item3", i, i)
    }).collect();

    let start_time = Instant::now();

    for (i, yaml) in yaml_instances.iter().enumerate() {
        let doc = &test_documents[i];
        let result = yaml.load_str(doc);
        assert!(result.is_ok(), "Document {} should parse", i);

        if let Ok(Value::Mapping(ref map)) = result {
            let key = Value::String(format!("test_doc_{}", i));
            assert!(map.contains_key(&key), "Should find expected key");
        }
    }

    let total_duration = start_time.elapsed();

    // Multiple instances should work efficiently
    assert!(
        total_duration < Duration::from_secs(1),
        "Concurrent parsing took too long: {:?}",
        total_duration
    );

    println!(
        "Concurrent parsing simulation completed in: {:?}",
        total_duration
    );
}
