//! Example demonstrating YAML writing and serialization capabilities

use indexmap::IndexMap;
use rust_yaml::{Value, Yaml};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let yaml = Yaml::new();

    println!("=== Example 1: Parsing YAML with anchors and merge keys ===");

    // Parse the YAML with anchors and merge keys
    let yaml_with_anchors = r#"
# Application configuration with anchors for shared settings
defaults: &defaults
  timeout: 30
  retry_count: 3
  log_level: info

# Database configuration
database:
  <<: *defaults
  host: localhost
  port: 5432

# API configuration  
api:
  <<: *defaults
  host: "0.0.0.0"
  port: 8080
  cors_enabled: true
"#;

    // Load and parse the YAML
    let parsed_value = yaml.load_str(yaml_with_anchors)?;

    // Write it back to YAML string
    let yaml_output = yaml.dump_str(&parsed_value)?;
    println!("Input YAML with anchors and merge keys:");
    println!("{}", yaml_with_anchors);
    println!("\nParsed and re-serialized YAML:");
    println!("{}", yaml_output);

    println!("\n=== Example 2: Creating YAML programmatically ===");

    // Create a new YAML structure programmatically
    let mut config = IndexMap::new();

    // Server configuration
    let mut server = IndexMap::new();
    server.insert(
        Value::String("host".to_string()),
        Value::String("127.0.0.1".to_string()),
    );
    server.insert(Value::String("port".to_string()), Value::Int(3000));
    server.insert(Value::String("ssl_enabled".to_string()), Value::Bool(true));

    // Database array
    let databases = vec![
        Value::String("primary".to_string()),
        Value::String("replica".to_string()),
        Value::String("cache".to_string()),
    ];

    // Feature flags
    let mut features = IndexMap::new();
    features.insert(
        Value::String("authentication".to_string()),
        Value::Bool(true),
    );
    features.insert(Value::String("logging".to_string()), Value::Bool(true));
    features.insert(Value::String("metrics".to_string()), Value::Bool(false));

    config.insert(Value::String("server".to_string()), Value::Mapping(server));
    config.insert(
        Value::String("databases".to_string()),
        Value::Sequence(databases),
    );
    config.insert(
        Value::String("features".to_string()),
        Value::Mapping(features),
    );
    config.insert(
        Value::String("version".to_string()),
        Value::String("1.0.0".to_string()),
    );
    config.insert(Value::String("debug".to_string()), Value::Bool(false));

    let new_yaml_value = Value::Mapping(config);

    // Convert to YAML string
    let new_yaml_output = yaml.dump_str(&new_yaml_value)?;
    println!("Programmatically created YAML:");
    println!("{}", new_yaml_output);

    println!("\n=== Example 3: Simple key-value pairs ===");

    let mut simple_config = IndexMap::new();
    simple_config.insert(
        Value::String("name".to_string()),
        Value::String("My Application".to_string()),
    );
    simple_config.insert(
        Value::String("environment".to_string()),
        Value::String("production".to_string()),
    );
    simple_config.insert(
        Value::String("max_connections".to_string()),
        Value::Int(100),
    );
    simple_config.insert(
        Value::String("timeout_seconds".to_string()),
        Value::Float(30.5),
    );
    simple_config.insert(Value::String("enabled".to_string()), Value::Bool(true));
    simple_config.insert(Value::String("null_value".to_string()), Value::Null);

    let simple_yaml = Value::Mapping(simple_config);
    let simple_output = yaml.dump_str(&simple_yaml)?;
    println!("Simple configuration YAML:");
    println!("{}", simple_output);

    Ok(())
}
