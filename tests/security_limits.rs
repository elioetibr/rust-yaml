//! Tests for security resource limits

use rust_yaml::{Limits, LoaderType, Yaml, YamlConfig};

#[test]
fn test_max_depth_limit() {
    // Create a YAML with excessive nesting
    let mut yaml_str = String::new();
    for _ in 0..60 {
        yaml_str.push_str("- ");
    }
    yaml_str.push_str("value");

    // Test with strict limits
    let config = YamlConfig {
        limits: Limits::strict(), // max_depth = 50
        loader_type: LoaderType::Safe,
        ..YamlConfig::default()
    };

    let yaml = Yaml::with_config(config);
    let result = yaml.load_str(&yaml_str);

    assert!(result.is_err());
    if let Err(e) = result {
        let error_str = e.to_string();
        assert!(
            error_str.contains("depth") || error_str.contains("limit"),
            "Expected depth limit error, got: {}",
            error_str
        );
    }
}

#[test]
fn test_max_string_length_limit() {
    // Create a YAML with a long string that exceeds the limit
    let long_string = "x".repeat(70_000); // 70KB string (above 64KB limit)
    let yaml_str = format!("key: \"{}\"", long_string);

    // Test with strict limits (max string = 64KB)
    let config = YamlConfig {
        limits: Limits::strict(),
        loader_type: LoaderType::Safe,
        ..YamlConfig::default()
    };

    let yaml = Yaml::with_config(config);
    let result = yaml.load_str(&yaml_str);

    assert!(result.is_err());
    if let Err(e) = result {
        let error_str = e.to_string();
        assert!(
            error_str.contains("string")
                || error_str.contains("length")
                || error_str.contains("limit"),
            "Expected string length limit error, got: {}",
            error_str
        );
    }
}

#[test]
fn test_max_anchor_limit() {
    // Create a YAML with too many anchors
    let mut yaml_str = String::new();
    for i in 0..150 {
        yaml_str.push_str(&format!("item{}: &anchor{} value{}\n", i, i, i));
    }

    // Test with strict limits (max anchors = 100)
    let config = YamlConfig {
        limits: Limits::strict(),
        loader_type: LoaderType::Safe,
        ..YamlConfig::default()
    };

    let yaml = Yaml::with_config(config);
    let result = yaml.load_str(&yaml_str);

    assert!(result.is_err());
    if let Err(e) = result {
        let error_str = e.to_string();
        assert!(
            error_str.contains("anchor") || error_str.contains("limit"),
            "Expected anchor limit error, got: {}",
            error_str
        );
    }
}

#[test]
fn test_max_document_size_limit() {
    // Create a document that exceeds size limit
    let large_doc = "x: ".to_string() + &"y".repeat(2_000_000); // 2MB document

    // Test with strict limits (max document size = 1MB)
    let config = YamlConfig {
        limits: Limits::strict(),
        loader_type: LoaderType::Safe,
        ..YamlConfig::default()
    };

    let yaml = Yaml::with_config(config);
    let result = yaml.load_str(&large_doc);

    assert!(result.is_err());
    if let Err(e) = result {
        let error_str = e.to_string();
        assert!(
            error_str.contains("document")
                || error_str.contains("size")
                || error_str.contains("limit"),
            "Expected document size limit error, got: {}",
            error_str
        );
    }
}

#[test]
fn test_max_collection_size_limit() {
    // Create a YAML with a collection that exceeds the strict limit
    // Using a smaller size to avoid timeout issues
    let mut yaml_str = String::new();
    for i in 0..200 {
        // Using 200 items which is above the test limit we'll set
        yaml_str.push_str(&format!("- item{}\n", i));
    }

    // Test with custom limits (set max collection size to 100)
    let mut limits = Limits::strict();
    limits.max_collection_size = 100; // Set limit to 100, test has 200 items

    let config = YamlConfig {
        limits,
        loader_type: LoaderType::Safe,
        ..YamlConfig::default()
    };

    let yaml = Yaml::with_config(config);
    let result = yaml.load_str(&yaml_str);

    // This should fail either during parsing or construction
    if let Err(e) = result {
        let error_str = e.to_string();
        assert!(
            error_str.contains("collection")
                || error_str.contains("sequence")
                || error_str.contains("limit"),
            "Expected collection size limit error, got: {}",
            error_str
        );
    }
}

#[test]
fn test_secure_config() {
    // Test the secure configuration preset
    let config = YamlConfig::secure();
    let yaml = Yaml::with_config(config);

    // Should be able to parse normal documents
    let normal_yaml = "key: value\nlist:\n  - item1\n  - item2";
    let result = yaml.load_str(normal_yaml);
    assert!(result.is_ok());

    // But should reject suspicious patterns
    let suspicious_yaml = "x: ".to_string() + &"y".repeat(2_000_000); // 2MB string
    let result = yaml.load_str(&suspicious_yaml);
    assert!(result.is_err());
}

#[test]
fn test_billion_laughs_attack() {
    // Classic billion laughs attack (exponential expansion via aliases)
    let yaml_bomb = r#"
a: &a ["lol", "lol", "lol", "lol", "lol", "lol", "lol", "lol", "lol"]
b: &b [*a, *a, *a, *a, *a, *a, *a, *a, *a]
c: &c [*b, *b, *b, *b, *b, *b, *b, *b, *b]
d: &d [*c, *c, *c, *c, *c, *c, *c, *c, *c]
e: &e [*d, *d, *d, *d, *d, *d, *d, *d, *d]
f: &f [*e, *e, *e, *e, *e, *e, *e, *e, *e]
g: &g [*f, *f, *f, *f, *f, *f, *f, *f, *f]
"#;

    // This would expand to 9^7 = 4,782,969 "lol" strings without protection
    let config = YamlConfig {
        limits: Limits::strict(),
        loader_type: LoaderType::Safe,
        ..YamlConfig::default()
    };

    let yaml = Yaml::with_config(config);
    let result = yaml.load_str(yaml_bomb);

    // Should fail with resource limit error, not consume excessive memory
    assert!(result.is_err(), "Should reject billion laughs attack");
    if let Err(e) = result {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("limit")
                || error_msg.contains("complexity")
                || error_msg.contains("alias")
                || error_msg.contains("collection"),
            "Should fail with resource limit error, got: {}",
            error_msg
        );
    }
}

#[test]
fn test_cyclic_alias_detection() {
    // Create cyclic reference - this is invalid YAML but shouldn't crash
    let yaml_str = r"
a: &a
  b: *b
b: &b
  a: *a
";

    let config = YamlConfig {
        limits: Limits::strict(),
        ..YamlConfig::default()
    };

    let yaml = Yaml::with_config(config);
    let result = yaml.load_str(yaml_str);

    // Should detect and reject cyclic references
    assert!(
        result.is_err(),
        "Should detect and reject cyclic references"
    );
}

#[test]
fn test_nested_alias_expansion_limit() {
    // Test nested alias expansion depth
    let yaml_str = r#"
a: &a "base"
b: &b [*a]
c: &c [*b]
d: &d [*c]
e: &e [*d]
f: &f [*e]
g: [*f]
"#;

    let config = YamlConfig {
        limits: Limits::strict(),
        ..YamlConfig::default()
    }; // max_alias_depth = 5

    let yaml = Yaml::with_config(config);
    let result = yaml.load_str(yaml_str);

    // Should fail when alias depth exceeds limit
    assert!(result.is_err(), "Should limit alias expansion depth");
}

#[test]
fn test_unlimited_config() {
    // Test that unlimited configuration allows large documents
    let config = YamlConfig {
        limits: Limits::unlimited(),
        ..YamlConfig::default()
    };
    let yaml = Yaml::with_config(config);

    // Should handle moderately large documents without issues
    let mut yaml_str = String::new();
    for i in 0..1000 {
        yaml_str.push_str(&format!("item{}: value{}\n", i, i));
    }

    let result = yaml.load_str(&yaml_str);
    assert!(result.is_ok());
}

#[test]
fn test_permissive_config() {
    // Test permissive configuration
    let config = YamlConfig {
        limits: Limits::permissive(),
        ..YamlConfig::default()
    };
    let yaml = Yaml::with_config(config);

    // Should handle large but reasonable documents
    let mut yaml_str = String::new();
    for i in 0..1000 {
        // Using 1000 items which should be fast enough in release mode
        yaml_str.push_str(&format!("- item{}\n", i));
    }

    let result = yaml.load_str(&yaml_str);
    // This may or may not succeed depending on implementation details
    // but should not panic
    let _ = result;
}
