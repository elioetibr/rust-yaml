//! Comprehensive demonstration of comment preservation features

use rust_yaml::{CommentedValue, Comments, LoaderType, Style, Value, Yaml, YamlConfig};

fn main() {
    println!("🔄 Comment Preservation Demo - Testing Round-Trip Functionality");

    // 1. Basic Comment Preservation
    println!("\n1. Testing Basic Comment Preservation");

    let config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let yaml = Yaml::with_config(config);

    let basic_yaml = r#"
# This is a leading comment
name: "rust-yaml"  # This is a trailing comment
# Another leading comment
version: "0.0.4"
"#;

    println!("Original YAML:");
    println!("{}", basic_yaml);

    match yaml.load_str_with_comments(basic_yaml) {
        Ok(commented_value) => {
            println!("✅ Successfully parsed YAML with comments");

            match yaml.dump_str_with_comments(&commented_value) {
                Ok(output) => {
                    println!("✅ Successfully serialized with comments preserved");
                    println!("Round-trip output:");
                    println!("{}", output);
                }
                Err(e) => println!("❌ Failed to serialize: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to parse: {}", e),
    }

    // 2. Complex Configuration with Comments
    println!("\n2. Testing Complex Configuration with Comments");

    let complex_yaml = r#"
# Application Configuration
# This file controls all aspects of the application

# Server Configuration
server:
  # Network settings
  host: "localhost"    # Bind to localhost for development
  port: 8080          # Standard HTTP port
  
  # Security settings
  ssl:
    enabled: true     # Enable HTTPS in production
    cert: "/etc/ssl/cert.pem"  # Certificate file path
    key: "/etc/ssl/key.pem"    # Private key file path

# Database Configuration  
database:
  # Connection details
  host: "db.example.com"  # Database server hostname
  port: 5432             # PostgreSQL default port
  name: "myapp_prod"     # Production database name
  
  # Connection pool settings
  pool:
    min_connections: 5   # Minimum pool size
    max_connections: 20  # Maximum pool size
    timeout: 30         # Connection timeout in seconds

# Feature Flags
features:
  # Core features
  - "authentication"  # User login and session management
  - "authorization"   # Role-based access control
  - "audit_logging"   # Security audit trail
  
  # Optional features
  - "metrics"        # Performance monitoring
  - "caching"        # Redis-based caching layer

# Monitoring Configuration
monitoring:
  # Logging configuration
  logging:
    level: "INFO"      # Log level (DEBUG, INFO, WARN, ERROR)
    format: "json"     # Log format (json, text)
    output: "stdout"   # Output destination
    
  # Metrics collection  
  metrics:
    enabled: true      # Enable metrics collection
    interval: 60       # Collection interval in seconds
    endpoint: "/metrics"  # Prometheus metrics endpoint
"#;

    println!("Complex configuration:");

    match yaml.load_str_with_comments(complex_yaml) {
        Ok(commented_value) => {
            println!("✅ Successfully parsed complex YAML with comments");

            // Verify the structure is preserved
            if let Value::Mapping(map) = &commented_value.value {
                let sections = ["server", "database", "features", "monitoring"];
                for section in &sections {
                    if map.contains_key(&Value::String(section.to_string())) {
                        println!("  ✓ Found section: {}", section);
                    }
                }
            }

            match yaml.dump_str_with_comments(&commented_value) {
                Ok(output) => {
                    println!("✅ Successfully serialized complex configuration");
                    println!("Output length: {} characters", output.len());

                    // Verify round-trip parsing
                    match yaml.load_str(&output) {
                        Ok(_) => println!("✅ Round-trip output is valid YAML"),
                        Err(e) => println!("❌ Round-trip output is invalid: {}", e),
                    }
                }
                Err(e) => println!("❌ Failed to serialize complex config: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to parse complex config: {}", e),
    }

    // 3. Comments with Arrays and Objects
    println!("\n3. Testing Comments with Arrays and Objects");

    let array_yaml = r#"
# User Management System
users:
  # Administrative users
  - name: "admin"        # System administrator
    role: "superuser"    # Full system access
    active: true         # Account status
    permissions:
      - "read"          # Read access
      - "write"         # Write access
      - "delete"        # Delete access
      - "admin"         # Administrative access
    
  # Regular users
  - name: "john_doe"     # Standard user account
    role: "user"         # Limited access
    active: true         # Currently active
    permissions:
      - "read"          # Read-only access
      - "write"         # Can modify own data
      
  - name: "jane_smith"   # Another user account
    role: "moderator"    # Elevated permissions
    active: false        # Temporarily disabled
    permissions:
      - "read"          # Read access
      - "write"         # Write access
      - "moderate"      # Moderation capabilities

# System Settings
settings:
  # Security policies
  password_policy:
    min_length: 8        # Minimum password length
    require_uppercase: true   # Must contain uppercase
    require_numbers: true     # Must contain numbers
    require_symbols: false    # Symbols are optional
    
  # Session management
  session:
    timeout: 3600        # Session timeout in seconds
    renewable: true      # Allow session renewal
    max_concurrent: 3    # Max concurrent sessions per user
"#;

    match yaml.load_str_with_comments(array_yaml) {
        Ok(commented_value) => {
            println!("✅ Successfully parsed arrays and objects with comments");

            match yaml.dump_str_with_comments(&commented_value) {
                Ok(output) => {
                    println!("✅ Successfully serialized arrays/objects with comments");

                    // Check that critical structure is preserved
                    let lines: Vec<&str> = output.lines().collect();
                    let has_users = lines.iter().any(|line| line.contains("users:"));
                    let has_settings = lines.iter().any(|line| line.contains("settings:"));

                    if has_users && has_settings {
                        println!("✅ Key sections preserved in output");
                    } else {
                        println!("⚠️  Some sections may not be preserved");
                    }
                }
                Err(e) => println!("❌ Failed to serialize arrays/objects: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to parse arrays/objects: {}", e),
    }

    // 4. Multi-line Strings with Comments
    println!("\n4. Testing Multi-line Strings with Comments");

    let multiline_yaml = r#"
# Database Configuration
database:
  # Connection string template
  connection_string: |  # Literal block scalar
    host=localhost
    port=5432
    dbname=myapp
    user=dbuser
    password=secret
    sslmode=require
    
  # SQL query templates
  user_query: |         # User lookup query
    SELECT u.id, u.name, u.email, u.created_at
    FROM users u
    WHERE u.active = true
      AND u.email_verified = true
    ORDER BY u.created_at DESC
    LIMIT 100;
    
  # Application description
  description: >        # Folded block scalar
    This is the main database configuration for the application.
    It includes connection parameters, query templates, and
    other database-related settings that are used throughout
    the application lifecycle.
    
# Script Configuration
scripts:
  # Startup script
  startup: |           # Multi-line startup commands
    echo "Starting application..."
    export NODE_ENV=production
    export LOG_LEVEL=info
    node server.js
    
  # Backup script  
  backup: |            # Database backup commands
    echo "Starting backup..."
    pg_dump -h localhost -U dbuser myapp > backup_$(date +%Y%m%d).sql
    echo "Backup completed"
"#;

    match yaml.load_str_with_comments(multiline_yaml) {
        Ok(commented_value) => {
            println!("✅ Successfully parsed multi-line strings with comments");

            match yaml.dump_str_with_comments(&commented_value) {
                Ok(output) => {
                    println!("✅ Successfully serialized multi-line strings");

                    // Verify that multi-line content is preserved
                    if output.contains("SELECT") && output.contains("pg_dump") {
                        println!("✅ Multi-line content preserved");
                    } else {
                        println!("⚠️  Multi-line content may be modified");
                    }
                }
                Err(e) => println!("❌ Failed to serialize multi-line: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to parse multi-line: {}", e),
    }

    // 5. Comments with Anchors and Aliases
    println!("\n5. Testing Comments with Anchors and Aliases");

    let anchor_yaml = r#"
# Default configuration template
defaults: &defaults    # Anchor for shared configuration
  timeout: 30         # Request timeout in seconds
  retries: 3          # Number of retry attempts
  log_level: "INFO"   # Default logging level
  
# Environment-specific configurations
environments:
  # Development environment
  development:
    <<: *defaults      # Merge default configuration
    debug: true        # Enable debug mode
    host: "localhost"  # Local development host
    port: 3000        # Development port
    
  # Staging environment  
  staging:
    <<: *defaults      # Merge default configuration
    debug: false       # Disable debug in staging
    host: "staging.example.com"  # Staging server
    port: 80          # Standard HTTP port
    
  # Production environment
  production:
    <<: *defaults      # Merge default configuration
    debug: false       # Never debug in production
    host: "prod.example.com"     # Production server
    port: 443         # HTTPS port
    ssl: true         # Enable SSL
"#;

    match yaml.load_str_with_comments(anchor_yaml) {
        Ok(commented_value) => {
            println!("✅ Successfully parsed anchors/aliases with comments");

            match yaml.dump_str_with_comments(&commented_value) {
                Ok(output) => {
                    println!("✅ Successfully serialized anchors/aliases");

                    // Note: Anchors/aliases are typically resolved during parsing
                    // so the output may not contain the original anchor syntax
                    if output.contains("development") && output.contains("production") {
                        println!("✅ Environment configurations preserved");
                    }
                }
                Err(e) => println!("❌ Failed to serialize anchors/aliases: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to parse anchors/aliases: {}", e),
    }

    // 6. Manual Comment Construction
    println!("\n6. Testing Manual Comment Construction");

    let mut manual_comments = Comments::new();
    manual_comments.add_leading("This is a manually added leading comment".to_string());
    manual_comments.add_leading("This is another leading comment".to_string());
    manual_comments.set_trailing("This is a manually added trailing comment".to_string());
    manual_comments.add_inner("This is an inner comment".to_string());

    let manual_value = Value::String("manually created value".to_string());
    let manual_commented = CommentedValue {
        value: manual_value,
        comments: manual_comments,
        style: Style::default(),
    };

    println!(
        "Manual CommentedValue created with {} leading comments",
        manual_commented.comments.leading.len()
    );
    println!(
        "Has trailing comment: {}",
        manual_commented.comments.trailing.is_some()
    );
    println!(
        "Has inner comments: {}",
        !manual_commented.comments.inner.is_empty()
    );

    match yaml.dump_str_with_comments(&manual_commented) {
        Ok(output) => {
            println!("✅ Successfully serialized manually constructed CommentedValue");
            println!("Output: {}", output.trim());
        }
        Err(e) => println!("❌ Failed to serialize manual comments: {}", e),
    }

    // 7. Performance Test
    println!("\n7. Testing Comment Preservation Performance");

    let start_time = std::time::Instant::now();

    // Create a larger YAML document with many comments
    let large_yaml = (0..100)
        .map(|i| {
            format!(
                r#"
# Configuration section {}
section_{}:
  # Property definitions for section {}
  id: {}              # Unique identifier
  name: "Section {}"   # Display name
  active: true        # Enable this section
  priority: {}        # Processing priority
"#,
                i,
                i,
                i,
                i,
                i,
                i % 10
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    match yaml.load_str_with_comments(&large_yaml) {
        Ok(commented_value) => {
            let parse_time = start_time.elapsed();
            println!("✅ Parsed large document in {:?}", parse_time);

            let serialize_start = std::time::Instant::now();
            match yaml.dump_str_with_comments(&commented_value) {
                Ok(output) => {
                    let serialize_time = serialize_start.elapsed();
                    println!("✅ Serialized large document in {:?}", serialize_time);
                    println!("Output size: {} characters", output.len());
                }
                Err(e) => println!("❌ Failed to serialize large document: {}", e),
            }
        }
        Err(e) => println!("❌ Failed to parse large document: {}", e),
    }

    // 8. Comparison with Regular Parsing
    println!("\n8. Comparing with Regular YAML Parsing");

    let regular_yaml = Yaml::new(); // Default configuration without comment preservation

    let test_yaml = r#"
# This comment will be ignored
key: value  # This comment will also be ignored
number: 42
"#;

    // Regular parsing (ignores comments)
    match regular_yaml.load_str(test_yaml) {
        Ok(regular_value) => {
            println!("✅ Regular parsing successful (comments ignored)");

            // Comment-preserving parsing
            match yaml.load_str_with_comments(test_yaml) {
                Ok(commented_value) => {
                    println!("✅ Comment-preserving parsing successful");

                    // Compare the actual values (should be the same)
                    if regular_value == commented_value.value {
                        println!("✅ Both parsing methods produce identical value structures");
                    } else {
                        println!("⚠️  Parsing methods produce different structures");
                    }
                }
                Err(e) => println!("❌ Comment-preserving parsing failed: {}", e),
            }
        }
        Err(e) => println!("❌ Regular parsing failed: {}", e),
    }

    println!("\n🎉 Comment Preservation Demo Complete!");
    println!("✅ Features demonstrated:");
    println!("   - Basic comment preservation (leading and trailing)");
    println!("   - Complex configuration with nested comments");
    println!("   - Comments with arrays and objects");
    println!("   - Multi-line strings with comments");
    println!("   - Comments with anchors and aliases");
    println!("   - Manual comment construction");
    println!("   - Performance with large documents");
    println!("   - Comparison with regular parsing");
}
