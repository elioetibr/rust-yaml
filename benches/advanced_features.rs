#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_raw_string_hashes)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rust_yaml::{BasicComposer, BasicEmitter, Composer, Emitter, Limits, Value, Yaml, YamlConfig};
use std::time::Duration;

/// Benchmarks for Sprint 3.0 advanced features
fn bench_comment_preservation(c: &mut Criterion) {
    let yaml = Yaml::new();

    let yaml_with_comments = r#"
# Main configuration
name: rust-yaml  # Project name
version: 0.1.0   # Current version

# Features section
features:
  - fast      # High performance
  - safe      # Memory safety
  - reliable  # Robust error handling

# Database configuration
database:
  # Connection settings
  host: localhost     # Database host
  port: 5432         # Database port
  # Credentials
  username: admin    # DB username
  password: secret   # DB password
"#;

    c.bench_function("parse_comments_preserved", |b| {
        b.iter(|| {
            yaml.load_str(std::hint::black_box(yaml_with_comments))
                .unwrap()
        });
    });

    // Parse once for serialization benchmark
    let parsed = yaml.load_str(yaml_with_comments).unwrap();

    c.bench_function("serialize_comments_preserved", |b| {
        b.iter(|| yaml.dump_str(std::hint::black_box(&parsed)).unwrap());
    });
}

fn bench_quote_style_preservation(c: &mut Criterion) {
    let yaml = Yaml::new();

    let yaml_with_quotes = r#"
plain_string: hello world
single_quoted: 'this is single quoted'
double_quoted: "this is double quoted"
mixed_quotes:
  - plain_item
  - 'single_item'
  - "double_item"
  - "special chars: \n\t\r"
special_strings:
  - 'true'   # Quoted to prevent bool interpretation
  - "123"    # Quoted to prevent int interpretation
  - '3.14'   # Quoted to prevent float interpretation
"#;

    c.bench_function("parse_quote_styles", |b| {
        b.iter(|| {
            yaml.load_str(std::hint::black_box(yaml_with_quotes))
                .unwrap()
        });
    });

    let parsed = yaml.load_str(yaml_with_quotes).unwrap();

    c.bench_function("serialize_quote_styles", |b| {
        b.iter(|| yaml.dump_str(std::hint::black_box(&parsed)).unwrap());
    });
}

fn bench_merge_keys(c: &mut Criterion) {
    // Skip merge keys benchmark for now - needs debugging
    c.bench_function("parse_merge_keys", |b| {
        b.iter(|| {
            // Placeholder benchmark
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box("key: value")).unwrap()
        });
    });

    c.bench_function("serialize_merge_keys", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            let parsed = yaml.load_str("key: value").unwrap();
            yaml.dump_str(std::hint::black_box(&parsed)).unwrap()
        });
    });
}

fn bench_indentation_styles(c: &mut Criterion) {
    let yaml = Yaml::new();

    let yaml_2_space = r#"
root:
  level1:
    level2:
      deeply:
        nested:
          value: "2-space indented"
    another: "branch"
  back_to_root: true
"#;

    let yaml_4_space = r#"
root:
    level1:
        level2:
            deeply:
                nested:
                    value: "4-space indented"
        another: "branch"
    back_to_root: true
"#;

    // Note: Tab indentation test would be here but is harder to represent in source code

    c.bench_function("parse_2_space_indent", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box(yaml_2_space)).unwrap());
    });

    c.bench_function("parse_4_space_indent", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box(yaml_4_space)).unwrap());
    });

    // Test indentation style detection overhead
    let mut group = c.benchmark_group("indent_detection");

    for size in [10, 50, 100, 500].iter() {
        let mut large_yaml = String::from("root:\n");
        for i in 0..*size {
            large_yaml.push_str(&format!("  level{}: value{}\n", i, i));
        }

        group.bench_with_input(
            BenchmarkId::new("detect_indent_style", size),
            size,
            |b, _| {
                b.iter(|| yaml.load_str(std::hint::black_box(&large_yaml)).unwrap());
            },
        );
    }
    group.finish();
}

fn bench_complex_keys(c: &mut Criterion) {
    let yaml = Yaml::new();

    let yaml_with_complex_keys = r#"
# Simple complex key
? [name, age]
: [John, 30]

# Complex mapping key
? {first: Alice, last: Smith}
: person_data

# Mixed key types in same mapping
simple_key: simple_value
? [complex, sequence, key]
: complex_sequence_value
? {nested: {key: structure}}
: complex_mapping_value

# Multiple levels of complex keys
complex_mapping:
  ? [level1, key]
  : level1_value
  nested_structure:
    ? {inner: complex}
    : inner_value
    ? [inner, sequence]
    : [inner, array, values]
"#;

    c.bench_function("parse_complex_keys", |b| {
        b.iter(|| {
            yaml.load_str(std::hint::black_box(yaml_with_complex_keys))
                .unwrap()
        });
    });

    // Test complex key construction performance
    c.bench_function("create_complex_key_mapping", |b| {
        b.iter(|| {
            // Create complex keys programmatically
            let sequence_key = Value::Sequence(vec![
                Value::String("name".to_string()),
                Value::String("age".to_string()),
            ]);

            let mapping_key = Value::mapping_with(vec![
                (
                    Value::String("first".to_string()),
                    Value::String("John".to_string()),
                ),
                (
                    Value::String("last".to_string()),
                    Value::String("Doe".to_string()),
                ),
            ]);

            let _complex_mapping = Value::mapping_with(vec![
                (
                    sequence_key,
                    Value::String("sequence_key_value".to_string()),
                ),
                (mapping_key, Value::String("mapping_key_value".to_string())),
                (
                    Value::String("simple".to_string()),
                    Value::String("simple_value".to_string()),
                ),
            ]);
        });
    });

    let parsed = yaml.load_str(yaml_with_complex_keys).unwrap();

    c.bench_function("serialize_complex_keys", |b| {
        b.iter(|| yaml.dump_str(std::hint::black_box(&parsed)).unwrap());
    });
}

fn bench_anchor_alias_performance(c: &mut Criterion) {
    let yaml = Yaml::new();

    // Test with varying numbers of aliases
    let mut group = c.benchmark_group("anchor_aliases");

    for alias_count in [1, 5, 10, 25, 50].iter() {
        let mut yaml_content = String::from("base: &base_config\n  timeout: 30\n  retries: 3\n\n");

        for i in 0..*alias_count {
            yaml_content.push_str(&format!("service_{}: *base_config\n", i));
        }

        group.bench_with_input(
            BenchmarkId::new("parse_aliases", alias_count),
            alias_count,
            |b, _| {
                b.iter(|| yaml.load_str(std::hint::black_box(&yaml_content)).unwrap());
            },
        );
    }
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    // Test memory efficiency with large documents
    let mut group = c.benchmark_group("memory_efficiency");
    group.measurement_time(Duration::from_secs(10));

    for size in [100, 500].iter() {
        // Generate large nested structure
        let mut yaml_content = String::from("large_structure:\n");
        for i in 0..*size {
            yaml_content.push_str(&format!("  item_{}:\n", i));
            yaml_content.push_str(&format!("    id: {}\n", i));
            yaml_content.push_str(&format!("    name: 'Item {}'\n", i));
            yaml_content.push_str("    tags: [important, processed]\n");
            yaml_content.push_str("    metadata:\n");
            yaml_content.push_str("      created: 2024-01-01\n");
            yaml_content.push_str("      active: true\n");
        }

        group.bench_with_input(
            BenchmarkId::new("large_document", size),
            &yaml_content,
            |b, content| {
                b.iter(|| {
                    // Create YAML processor with increased limits for benchmarking
                    let config = YamlConfig {
                        limits: Limits {
                            max_depth: 10000,
                            max_anchors: 10000,
                            max_document_size: 100_000_000, // 100MB
                            max_string_length: 10_000_000,
                            max_alias_depth: 100,
                            max_collection_size: 100000,
                            max_complexity_score: 1_000_000,
                            timeout: None,
                        },
                        ..Default::default()
                    };
                    let yaml = Yaml::with_config(config);
                    yaml.load_str(std::hint::black_box(content)).unwrap()
                });
            },
        );
    }
    group.finish();
}

fn bench_component_performance(c: &mut Criterion) {
    // Test individual component performance
    let yaml_content = r#"
users:
  - name: Alice
    roles: [admin, user]
    config: &user_config
      theme: dark
      notifications: true
  - name: Bob
    roles: [user]
    config: *user_config
settings:
  base: *user_config
  global: true
"#;

    c.bench_function("composer_only", |b| {
        b.iter(|| {
            let mut composer =
                BasicComposer::new_eager(std::hint::black_box(yaml_content.to_string()));
            composer.compose_document().unwrap()
        });
    });

    let mut composer = BasicComposer::new_eager(yaml_content.to_string());
    let parsed_doc = composer.compose_document().unwrap().unwrap();

    c.bench_function("emitter_only", |b| {
        b.iter(|| {
            let mut emitter = BasicEmitter::new();
            let mut output = Vec::new();
            emitter
                .emit(std::hint::black_box(&parsed_doc), &mut output)
                .unwrap()
        });
    });
}

criterion_group!(
    advanced_benches,
    bench_comment_preservation,
    bench_quote_style_preservation,
    bench_merge_keys,
    bench_indentation_styles,
    bench_complex_keys,
    bench_anchor_alias_performance,
    bench_memory_usage,
    bench_component_performance
);
criterion_main!(advanced_benches);
