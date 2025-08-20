//! Comprehensive benchmarks for comment preservation features

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rust_yaml::{CommentedValue, Comments, LoaderType, Style, Value, Yaml, YamlConfig};

fn bench_comment_parsing_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("comment_parsing");

    // Regular YAML parser (baseline)
    let regular_yaml = Yaml::new();

    // Comment-preserving YAML parser
    let comment_config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let comment_yaml = Yaml::with_config(comment_config);

    // Test YAML with various comment densities
    let light_comments = r#"
# Light comments
name: "test"  # Simple comment
version: "1.0"
features: [fast, safe]
"#;

    let medium_comments = r#"
# Application Configuration
# Version 1.0 - Production Ready

# Basic Information
name: "rust-yaml"      # Project name
version: "1.0.0"       # Current version
author: "Developer"    # Main author

# Feature Configuration  
features:
  # Core features
  - "parsing"          # YAML parsing capability
  - "serialization"    # YAML output generation
  - "validation"       # Schema validation
  
  # Advanced features
  - "comments"         # Comment preservation
  - "anchors"          # Anchor/alias support
  - "multiline"        # Multi-line string handling

# Database Settings
database:
  # Connection details
  host: "localhost"    # Database host
  port: 5432          # PostgreSQL port
  name: "appdb"       # Database name
  
  # Pool configuration
  pool:
    min: 5            # Minimum connections
    max: 20           # Maximum connections
"#;

    let heavy_comments = (0..50)
        .map(|i| {
            format!(
                r#"
# Section {} - Configuration Item
# This section contains configuration for item {}
# Updated: 2024-01-{:02}
item_{}:
  # Basic properties for item {}
  id: {}              # Unique identifier for item {}
  name: "Item {}"      # Display name for item {}
  active: true        # Status flag for item {}
  priority: {}        # Priority level (0-9) for item {}
  
  # Advanced properties
  metadata:
    # Creation information
    created: "2024-01-{:02}"  # Creation date for item {}
    creator: "system"    # Creator of item {}
    
    # Processing flags
    processed: false    # Has item {} been processed?
    retries: 0         # Number of retries for item {}
"#,
                i,
                i,
                (i % 31) + 1,
                i,
                i,
                i,
                i,
                i,
                i,
                i,
                i % 10,
                (i % 31) + 1,
                i,
                i,
                i,
                i,
                i
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Benchmark regular parsing (baseline)
    group.bench_function("regular_light_comments", |b| {
        b.iter(|| {
            regular_yaml
                .load_str(std::hint::black_box(light_comments))
                .unwrap()
        });
    });

    group.bench_function("regular_medium_comments", |b| {
        b.iter(|| {
            regular_yaml
                .load_str(std::hint::black_box(medium_comments))
                .unwrap()
        });
    });

    group.bench_function("regular_heavy_comments", |b| {
        b.iter(|| {
            regular_yaml
                .load_str(std::hint::black_box(&heavy_comments))
                .unwrap()
        });
    });

    // Benchmark comment-preserving parsing
    group.bench_function("preserved_light_comments", |b| {
        b.iter(|| {
            comment_yaml
                .load_str_with_comments(std::hint::black_box(light_comments))
                .unwrap()
        });
    });

    group.bench_function("preserved_medium_comments", |b| {
        b.iter(|| {
            comment_yaml
                .load_str_with_comments(std::hint::black_box(medium_comments))
                .unwrap()
        });
    });

    group.bench_function("preserved_heavy_comments", |b| {
        b.iter(|| {
            comment_yaml
                .load_str_with_comments(std::hint::black_box(&heavy_comments))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_comment_serialization_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("comment_serialization");

    let comment_config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let comment_yaml = Yaml::with_config(comment_config);
    let regular_yaml = Yaml::new();

    // Prepare test data with different comment densities
    let test_yamls = vec![
        (
            "light",
            r#"
# Light comments
name: "test"  # Simple comment
version: "1.0"
features: [fast, safe]
"#,
        ),
        (
            "medium",
            r#"
# Application Configuration
# Version 1.0 - Production Ready

# Basic Information
name: "rust-yaml"      # Project name
version: "1.0.0"       # Current version

# Feature Configuration  
features:
  # Core features
  - "parsing"          # YAML parsing
  - "serialization"    # YAML output
  
# Database Settings
database:
  # Connection details
  host: "localhost"    # Database host
  port: 5432          # PostgreSQL port
"#,
        ),
    ];

    for (density, yaml_content) in test_yamls {
        // Parse once for reuse
        let regular_value = regular_yaml.load_str(yaml_content).unwrap();
        let commented_value = comment_yaml.load_str_with_comments(yaml_content).unwrap();

        // Benchmark regular serialization
        group.bench_function(format!("regular_serialize_{}", density), |b| {
            b.iter(|| {
                regular_yaml
                    .dump_str(std::hint::black_box(&regular_value))
                    .unwrap()
            });
        });

        // Benchmark comment-preserving serialization
        group.bench_function(format!("preserved_serialize_{}", density), |b| {
            b.iter(|| {
                comment_yaml
                    .dump_str_with_comments(std::hint::black_box(&commented_value))
                    .unwrap()
            });
        });
    }

    group.finish();
}

fn bench_comment_round_trip_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("comment_round_trip");

    let comment_config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let comment_yaml = Yaml::with_config(comment_config);
    let regular_yaml = Yaml::new();

    let test_yaml = r#"
# Configuration File
# Generated on 2024-01-01

# Server Configuration
server:
  # Network settings
  host: "localhost"    # Bind address
  port: 8080          # Listen port
  
  # Security settings
  ssl: true           # Enable HTTPS
  cert: "/path/cert"  # Certificate path

# Database Configuration
database:
  # Connection pool
  host: "db.local"    # DB host
  port: 5432         # DB port
  pool_size: 10      # Connection pool size
  
# Feature flags
features:
  - "auth"           # Authentication
  - "metrics"        # Monitoring  
  - "cache"          # Caching layer
"#;

    // Benchmark regular round-trip (parse + serialize)
    group.bench_function("regular_round_trip", |b| {
        b.iter(|| {
            let parsed = regular_yaml
                .load_str(std::hint::black_box(test_yaml))
                .unwrap();
            regular_yaml
                .dump_str(std::hint::black_box(&parsed))
                .unwrap()
        });
    });

    // Benchmark comment-preserving round-trip
    group.bench_function("preserved_round_trip", |b| {
        b.iter(|| {
            let parsed = comment_yaml
                .load_str_with_comments(std::hint::black_box(test_yaml))
                .unwrap();
            comment_yaml
                .dump_str_with_comments(std::hint::black_box(&parsed))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_comment_construction_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("comment_construction");

    let comment_config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let comment_yaml = Yaml::with_config(comment_config);

    // Benchmark manual comment construction
    group.bench_function("manual_comment_construction", |b| {
        b.iter(|| {
            let mut comments = Comments::new();
            comments.add_leading("Leading comment 1".to_string());
            comments.add_leading("Leading comment 2".to_string());
            comments.set_trailing("Trailing comment".to_string());
            comments.add_inner("Inner comment".to_string());

            let value = Value::String("test value".to_string());
            let commented = CommentedValue {
                value,
                comments,
                style: Style::default(),
            };

            std::hint::black_box(commented);
        });
    });

    // Benchmark serialization of manually constructed comments
    group.bench_function("serialize_manual_comments", |b| {
        // Pre-construct the commented value
        let mut comments = Comments::new();
        comments.add_leading("Leading comment".to_string());
        comments.set_trailing("Trailing comment".to_string());

        let value = Value::String("test value".to_string());
        let commented = CommentedValue {
            value,
            comments,
            style: Style::default(),
        };

        b.iter(|| {
            comment_yaml
                .dump_str_with_comments(std::hint::black_box(&commented))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_comment_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("comment_memory");

    let comment_config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let comment_yaml = Yaml::with_config(comment_config);

    // Test with increasing numbers of comments
    for comment_count in [10, 50, 100, 500].iter() {
        let yaml_with_many_comments = (0..*comment_count)
            .map(|i| format!("# Comment number {}\nkey_{}: value_{}", i, i, i))
            .collect::<Vec<_>>()
            .join("\n");

        group.bench_with_input(
            BenchmarkId::new("parse_many_comments", comment_count),
            comment_count,
            |b, _| {
                b.iter(|| {
                    comment_yaml
                        .load_str_with_comments(std::hint::black_box(&yaml_with_many_comments))
                        .unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_complex_comment_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_comment_scenarios");

    let comment_config = YamlConfig {
        preserve_comments: true,
        loader_type: LoaderType::RoundTrip,
        ..Default::default()
    };
    let comment_yaml = Yaml::with_config(comment_config);

    // Scenario 1: Nested structures with comments
    let nested_yaml = r#"
# Root level comment
application:
  # Server section comment
  server:
    # Host configuration comment
    host: "localhost"  # Inline host comment
    # Port configuration comment  
    port: 8080        # Inline port comment
    
    # SSL section comment
    ssl:
      # Certificate comments
      enabled: true   # SSL enabled comment
      cert: "/cert"   # Certificate path comment
      
  # Database section comment
  database:
    # Connection comments
    connections:
      # Pool comments
      - host: "db1"   # Primary DB comment
        port: 5432    # Primary port comment
      - host: "db2"   # Secondary DB comment
        port: 5432    # Secondary port comment
"#;

    group.bench_function("nested_structures_with_comments", |b| {
        b.iter(|| {
            comment_yaml
                .load_str_with_comments(std::hint::black_box(nested_yaml))
                .unwrap()
        });
    });

    // Scenario 2: Arrays with many commented items
    let array_yaml = format!(
        r#"
# Configuration items array
items:
{}
"#,
        (0..20)
            .map(|i| {
                format!(
                    r#"  # Item {} configuration
  - id: {}              # Unique ID for item {}
    name: "Item {}"      # Display name for item {}
    active: true        # Status for item {}
    config:
      # Nested config for item {}
      setting1: value{}  # Setting 1 for item {}
      setting2: value{}  # Setting 2 for item {}"#,
                    i,
                    i,
                    i,
                    i,
                    i,
                    i,
                    i,
                    i * 2,
                    i,
                    i * 3,
                    i
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    );

    group.bench_function("arrays_with_many_comments", |b| {
        b.iter(|| {
            comment_yaml
                .load_str_with_comments(std::hint::black_box(&array_yaml))
                .unwrap()
        });
    });

    // Scenario 3: Multi-line strings with comments
    let multiline_yaml = r#"
# Scripts configuration
scripts:
  # Startup script with comments
  startup: |            # Literal block scalar comment
    #!/bin/bash
    # This is a startup script
    echo "Starting application..."
    # Set environment variables
    export NODE_ENV=production
    # Start the server
    node server.js
    
  # Backup script with comments  
  backup: >             # Folded block scalar comment
    This is a long description of the backup process
    that spans multiple lines and will be folded into
    a single line when processed by the YAML parser.
    
  # Configuration template
  config_template: |    # Another literal block
    # Generated configuration file
    server:
      host: ${HOST}
      port: ${PORT}
    database:
      url: ${DB_URL}
"#;

    group.bench_function("multiline_strings_with_comments", |b| {
        b.iter(|| {
            comment_yaml
                .load_str_with_comments(std::hint::black_box(multiline_yaml))
                .unwrap()
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_comment_parsing_performance,
    bench_comment_serialization_performance,
    bench_comment_round_trip_performance,
    bench_comment_construction_performance,
    bench_comment_memory_usage,
    bench_complex_comment_scenarios
);

criterion_main!(benches);
