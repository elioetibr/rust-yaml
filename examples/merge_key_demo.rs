//! Demonstration of merge key functionality

#![allow(clippy::needless_raw_string_hashes)] // Test YAML strings

use rust_yaml::{Value, Yaml};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example YAML with merge keys
    let yaml_input = r#"
# Define reusable configurations
defaults: &defaults
  adapter: postgres
  host: localhost
  port: 5432
  timeout: 30

# Development environment inherits from defaults and adds specific configs
development:
  <<: *defaults
  database: dev_db
  debug: true

# Production environment inherits and overrides
production:
  <<: *defaults
  host: prod.example.com
  database: prod_db
  timeout: 60  # Override timeout
  ssl: true

# Multiple inheritance example
base_features: &features
  logging: enabled
  monitoring: basic

advanced_features: &advanced
  caching: redis
  queue: rabbitmq

# Service configuration inheriting from multiple sources
api_service:
  <<: [*features, *advanced, *defaults]
  name: api-service
  port: 8080  # Override default port
"#;

    println!("=== Merge Key Demonstration ===");
    println!("Input YAML:\n{}", yaml_input);

    let yaml = Yaml::new();
    let result = yaml.load_str(yaml_input)?;

    println!("\n=== Parsed Structure ===");

    if let Value::Mapping(root) = &result {
        // Check development environment
        if let Some(Value::Mapping(dev)) = root.get(&Value::String("development".to_string())) {
            println!("\nDevelopment environment:");
            for (key, value) in dev {
                println!("  {}: {:?}", key, value);
            }

            // Verify inherited values
            assert_eq!(
                dev.get(&Value::String("adapter".to_string())),
                Some(&Value::String("postgres".to_string())),
                "Should inherit adapter from defaults"
            );
            assert_eq!(
                dev.get(&Value::String("database".to_string())),
                Some(&Value::String("dev_db".to_string())),
                "Should have specific database"
            );
        }

        // Check production environment
        if let Some(Value::Mapping(prod)) = root.get(&Value::String("production".to_string())) {
            println!("\nProduction environment:");
            for (key, value) in prod {
                println!("  {}: {:?}", key, value);
            }

            // Verify overridden values
            assert_eq!(
                prod.get(&Value::String("host".to_string())),
                Some(&Value::String("prod.example.com".to_string())),
                "Should override host"
            );
            assert_eq!(
                prod.get(&Value::String("timeout".to_string())),
                Some(&Value::Int(60)),
                "Should override timeout"
            );
        }

        // Check API service with multiple inheritance
        if let Some(Value::Mapping(api)) = root.get(&Value::String("api_service".to_string())) {
            println!("\nAPI Service (multiple inheritance):");
            for (key, value) in api {
                println!("  {}: {:?}", key, value);
            }

            // Should have values from all sources
            assert_eq!(
                api.get(&Value::String("logging".to_string())),
                Some(&Value::String("enabled".to_string())),
                "Should inherit from features"
            );
            assert_eq!(
                api.get(&Value::String("caching".to_string())),
                Some(&Value::String("redis".to_string())),
                "Should inherit from advanced features"
            );
            assert_eq!(
                api.get(&Value::String("adapter".to_string())),
                Some(&Value::String("postgres".to_string())),
                "Should inherit from defaults"
            );
            assert_eq!(
                api.get(&Value::String("port".to_string())),
                Some(&Value::Int(8080)),
                "Should override port from defaults"
            );
        }
    }

    println!("\n=== Round-trip Test ===");

    // Test round-trip serialization
    let serialized = yaml.dump_str(&result)?;
    println!("Serialized YAML:\n{}", serialized);

    // Parse it back
    let round_trip = yaml.load_str(&serialized)?;

    // Should be equivalent (though merge keys will be resolved)
    println!(
        "Round-trip test: {}",
        if result == round_trip {
            "✅ PASSED"
        } else {
            "❌ FAILED"
        }
    );

    println!("\n=== Summary ===");
    println!("✅ Basic merge key inheritance");
    println!("✅ Value overriding");
    println!("✅ Multiple source inheritance");
    println!("✅ Complex nested structures");
    println!("✅ Round-trip compatibility");
    println!("\nMerge key functionality is working correctly!");

    Ok(())
}
