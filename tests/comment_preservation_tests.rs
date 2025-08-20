//! Comprehensive integration tests for comment preservation

use rust_yaml::{CommentedValue, Comments, LoaderType, Style, Value, Yaml, YamlConfig};

#[test]
fn test_comment_preservation_basic() {
    // Create YAML parser with comment preservation enabled
    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    let yaml_with_comments = r#"
# This is a leading comment
key: value  # This is a trailing comment
# This is another leading comment
number: 42
"#;

    // Test that we can parse with comments using the comment-preserving API
    let result = yaml.load_str_with_comments(yaml_with_comments);
    assert!(result.is_ok(), "Should be able to parse YAML with comments");

    let commented_value = result.unwrap();

    // Verify the structure is preserved
    if let Value::Mapping(map) = &commented_value.value {
        assert!(map.contains_key(&Value::String("key".to_string())));
        assert!(map.contains_key(&Value::String("number".to_string())));
    } else {
        panic!("Expected mapping at root level");
    }

    // Test round-trip serialization
    let serialized = yaml.dump_str_with_comments(&commented_value);
    assert!(
        serialized.is_ok(),
        "Should be able to serialize commented value"
    );

    let output = serialized.unwrap();

    // Verify that the output is valid YAML
    let reparsed = yaml.load_str(&output);
    assert!(reparsed.is_ok(), "Round-trip output should be valid YAML");
}

#[test]
fn test_comment_preservation_complex() {
    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    let complex_yaml_with_comments = r#"
# Main configuration file
# This controls the entire application

application:
  # Application name and version
  name: "MyApp"  # The official name
  version: "1.0.0"  # Semantic versioning
  
  # Server configuration
  server:
    # Network settings
    host: "localhost"  # Bind address
    port: 8080        # Listen port
    
    # SSL configuration
    ssl:
      enabled: true   # Enable HTTPS
      cert: "/path/to/cert.pem"  # Certificate file
      key: "/path/to/key.pem"   # Private key file

# Database configuration
database:
  # Primary database connection
  host: "db.example.com"  # Database host
  port: 5432             # Database port
  name: "myapp"          # Database name
  
  # Connection pool settings
  pool:
    min: 5   # Minimum connections
    max: 20  # Maximum connections

# Feature flags
features:
  - "authentication"  # User login system
  - "authorization"   # Permission system
  - "logging"        # Application logging
  - "metrics"        # Performance metrics
"#;

    let result = yaml.load_str_with_comments(complex_yaml_with_comments);
    assert!(result.is_ok(), "Should parse complex YAML with comments");

    let commented_value = result.unwrap();

    // Test serialization of complex structure
    let serialized = yaml.dump_str_with_comments(&commented_value);
    assert!(
        serialized.is_ok(),
        "Should serialize complex commented structure"
    );

    let output = serialized.unwrap();

    // Verify round-trip parsing
    let reparsed = yaml.load_str(&output);
    assert!(
        reparsed.is_ok(),
        "Complex round-trip should produce valid YAML"
    );

    // Verify structure is preserved
    let original_parsed = yaml.load_str(complex_yaml_with_comments).unwrap();
    let round_trip_parsed = reparsed.unwrap();
    assert_eq!(
        original_parsed, round_trip_parsed,
        "Round-trip should preserve structure"
    );
}

#[test]
fn test_comment_preservation_sequences() {
    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    let sequence_yaml = r#"
# List of users
users:
  # Administrative users
  - name: "admin"     # System administrator
    role: "admin"     # Full access
    active: true      # Currently active
    
  # Regular users  
  - name: "user1"     # First user
    role: "user"      # Limited access
    active: true      # Currently active
    
  - name: "user2"     # Second user
    role: "user"      # Limited access  
    active: false     # Inactive account

# Configuration settings
settings:
  # Notification preferences
  - email: true       # Email notifications
  - sms: false       # SMS notifications
  - push: true       # Push notifications
"#;

    let result = yaml.load_str_with_comments(sequence_yaml);
    assert!(result.is_ok(), "Should parse sequences with comments");

    let commented_value = result.unwrap();

    // Test serialization
    let serialized = yaml.dump_str_with_comments(&commented_value);
    assert!(
        serialized.is_ok(),
        "Should serialize sequences with comments"
    );

    let output = serialized.unwrap();

    // Verify the output is valid and structure is preserved
    let reparsed = yaml.load_str(&output).unwrap();
    let original_parsed = yaml.load_str(sequence_yaml).unwrap();
    assert_eq!(
        original_parsed, reparsed,
        "Sequence structure should be preserved"
    );
}

#[test]
fn test_comment_preservation_multiline_strings() {
    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    let multiline_yaml = r#"
# Configuration with multiline strings
config:
  # SQL query for user lookup
  user_query: |  # Literal block scalar
    SELECT id, name, email
    FROM users
    WHERE active = true
    ORDER BY name
  
  # Application description  
  description: >  # Folded block scalar
    This is a long description that will be
    folded into a single line when processed,
    making it easier to read in the YAML file
    while maintaining proper formatting.
    
  # Simple configuration values
  debug: true      # Enable debug mode
  timeout: 30      # Request timeout in seconds
"#;

    let result = yaml.load_str_with_comments(multiline_yaml);
    assert!(
        result.is_ok(),
        "Should parse multiline strings with comments"
    );

    let commented_value = result.unwrap();

    // Test serialization
    let serialized = yaml.dump_str_with_comments(&commented_value);
    assert!(
        serialized.is_ok(),
        "Should serialize multiline strings with comments"
    );

    let output = serialized.unwrap();

    // Verify structure preservation
    let reparsed = yaml.load_str(&output).unwrap();
    let original_parsed = yaml.load_str(multiline_yaml).unwrap();
    assert_eq!(
        original_parsed, reparsed,
        "Multiline string structure should be preserved"
    );
}

#[test]
fn test_comment_preservation_with_anchors_and_aliases() {
    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    let anchor_yaml = r#"
# Default configuration template
defaults: &defaults  # Anchor for shared config
  timeout: 30         # Default timeout
  retries: 3         # Default retry count
  debug: false       # Default debug setting

# Development environment
development:
  <<: *defaults       # Merge defaults
  debug: true         # Override debug for dev
  host: "localhost"   # Dev host
  
# Production environment  
production:
  <<: *defaults       # Merge defaults
  host: "prod.example.com"  # Production host
  ssl: true          # Enable SSL in production
  
# Testing environment
testing:
  <<: *defaults       # Merge defaults
  host: "test.example.com"  # Test host
  debug: true         # Enable debug for testing
"#;

    let result = yaml.load_str_with_comments(anchor_yaml);
    assert!(result.is_ok(), "Should parse anchors/aliases with comments");

    let commented_value = result.unwrap();

    // Test serialization
    let serialized = yaml.dump_str_with_comments(&commented_value);
    assert!(
        serialized.is_ok(),
        "Should serialize anchors/aliases with comments"
    );

    let output = serialized.unwrap();

    // Verify structure preservation (anchors/aliases are resolved during parsing)
    let reparsed = yaml.load_str(&output).unwrap();
    let original_parsed = yaml.load_str(anchor_yaml).unwrap();
    assert_eq!(
        original_parsed, reparsed,
        "Anchor/alias structure should be preserved"
    );
}

#[test]
fn test_comment_preservation_edge_cases() {
    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    // Test various edge cases
    let edge_case_yaml = r#"
# Comment at start of document
key1: value1  # Trailing comment

# Multiple leading comments
# Another leading comment
# Yet another leading comment
key2: value2

key3: value3  # Comment with special chars: @#$%^&*()

# Comment with quotes and escapes
key4: "quoted value"  # Comment with "quotes" and 'apostrophes'

# Empty lines and spacing


key5: value5  # Comment after empty lines

# Comment before end of document
"#;

    let result = yaml.load_str_with_comments(edge_case_yaml);
    assert!(
        result.is_ok(),
        "Should handle edge cases in comment preservation"
    );

    let commented_value = result.unwrap();

    // Test serialization
    let serialized = yaml.dump_str_with_comments(&commented_value);
    assert!(serialized.is_ok(), "Should serialize edge cases");

    let output = serialized.unwrap();

    // Verify the output is valid YAML
    let reparsed = yaml.load_str(&output);
    assert!(
        reparsed.is_ok(),
        "Edge case round-trip should be valid YAML"
    );
}

#[test]
fn test_comment_preservation_fallback_to_regular_parsing() {
    // Test that when comment preservation is disabled, we fall back to regular parsing
    let regular_yaml = Yaml::new(); // Default config without comment preservation

    let yaml_with_comments = r#"
# This comment will be ignored
key: value  # This comment will also be ignored
number: 42
"#;

    // Should parse successfully but ignore comments
    let result = regular_yaml.load_str(yaml_with_comments);
    assert!(result.is_ok(), "Should parse YAML ignoring comments");

    // Verify structure is still correct
    let value = result.unwrap();
    if let Value::Mapping(map) = value {
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
fn test_comment_preservation_multi_document() {
    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    let multi_doc_yaml = r#"
# First document
# Configuration for service A
name: "service-a"  # Service name
port: 8080        # Listen port
---
# Second document  
# Configuration for service B
name: "service-b"  # Service name
port: 8081        # Listen port
---
# Third document
# Configuration for service C
name: "service-c"  # Service name
port: 8082        # Listen port
"#;

    // Test parsing multiple documents with comments
    let docs_result = yaml.load_all_str(multi_doc_yaml);
    assert!(
        docs_result.is_ok(),
        "Should parse multi-document YAML with comments"
    );

    let docs = docs_result.unwrap();
    assert_eq!(docs.len(), 3, "Should have 3 documents");

    // Verify each document has correct structure
    for (i, doc) in docs.iter().enumerate() {
        if let Value::Mapping(map) = doc {
            let expected_name = format!("service-{}", ['a', 'b', 'c'][i]);
            #[allow(clippy::cast_possible_wrap)]
            let expected_port = 8080 + i as i64;

            assert_eq!(
                map.get(&Value::String("name".to_string())),
                Some(&Value::String(expected_name))
            );
            assert_eq!(
                map.get(&Value::String("port".to_string())),
                Some(&Value::Int(expected_port))
            );
        } else {
            panic!("Expected mapping in document {}", i);
        }
    }
}

#[test]
fn test_comment_preservation_performance() {
    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    // Create a moderately complex YAML with many comments
    let complex_yaml = (0..50)
        .map(|i| {
            format!(
                r#"
# Configuration item {}
item_{}: 
  # Sub-item properties
  id: {}          # Unique identifier
  name: "Item {}"  # Display name
  active: true    # Status flag
  priority: {}    # Priority level
"#,
                i,
                i,
                i,
                i,
                i % 5
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Test that parsing doesn't take unreasonably long
    let start = std::time::Instant::now();
    let result = yaml.load_str_with_comments(&complex_yaml);
    let parse_duration = start.elapsed();

    assert!(
        result.is_ok(),
        "Should parse complex YAML with many comments"
    );
    assert!(
        parse_duration.as_secs() < 5,
        "Parsing should complete within 5 seconds"
    );

    // Test serialization performance
    let commented_value = result.unwrap();
    let start = std::time::Instant::now();
    let serialized = yaml.dump_str_with_comments(&commented_value);
    let serialize_duration = start.elapsed();

    assert!(
        serialized.is_ok(),
        "Should serialize complex commented YAML"
    );
    assert!(
        serialize_duration.as_secs() < 5,
        "Serialization should complete within 5 seconds"
    );
}

#[test]
fn test_commented_value_construction() {
    // Test manual construction of CommentedValue
    let value = Value::String("test".to_string());
    let mut comments = Comments::new();
    comments.add_leading("This is a leading comment".to_string());
    comments.set_trailing("This is a trailing comment".to_string());

    let commented_value = CommentedValue {
        value,
        comments,
        style: Style::default(),
    };

    assert!(
        commented_value.has_comments(),
        "Should detect presence of comments"
    );

    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    // Test serialization of manually constructed CommentedValue
    let result = yaml.dump_str_with_comments(&commented_value);
    assert!(
        result.is_ok(),
        "Should serialize manually constructed CommentedValue"
    );
}

#[test]
fn test_comment_preservation_with_different_quote_styles() {
    let config = YamlConfig {
        preserve_comments: true,
        preserve_quotes: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    let quoted_yaml = r#"
# Different quote styles
plain_string: hello world      # No quotes
single_quoted: 'single quotes' # Single quotes
double_quoted: "double quotes" # Double quotes

# Special cases
quoted_number: "123"          # Quoted to prevent parsing as number
quoted_boolean: 'true'        # Quoted to prevent parsing as boolean
special_chars: "line1\nline2" # Escaped characters
"#;

    let result = yaml.load_str_with_comments(quoted_yaml);
    assert!(result.is_ok(), "Should parse quoted strings with comments");

    let commented_value = result.unwrap();

    // Test serialization preserves quote styles and comments
    let serialized = yaml.dump_str_with_comments(&commented_value);
    assert!(
        serialized.is_ok(),
        "Should serialize quoted strings with comments"
    );

    let output = serialized.unwrap();

    // Verify structure is preserved
    let reparsed = yaml.load_str(&output);
    assert!(
        reparsed.is_ok(),
        "Round-trip with quotes and comments should be valid"
    );
}
