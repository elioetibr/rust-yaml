#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_raw_string_hashes)]

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rust_yaml::{BasicParser, BasicScanner, Parser, Scanner, Yaml};
use std::time::Duration;

/// Performance regression tests to track optimization improvements
fn bench_baseline_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("baseline_performance");
    group.measurement_time(Duration::from_secs(10));

    let yaml = Yaml::new();
    let simple_yaml = "key: value\nlist: [1, 2, 3]\nnested: {inner: data}";

    // Baseline parsing performance
    group.bench_function("simple_parse_baseline", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box(simple_yaml)).unwrap());
    });

    // Baseline serialization performance
    let parsed = yaml.load_str(simple_yaml).unwrap();
    group.bench_function("simple_serialize_baseline", |b| {
        b.iter(|| yaml.dump_str(std::hint::black_box(&parsed)).unwrap());
    });

    group.finish();
}

fn bench_memory_allocation_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");

    // Test different document sizes to measure allocation efficiency
    for doc_size in [10, 100, 500, 1000].iter() {
        let yaml_content = generate_structured_yaml(*doc_size);
        let content_size = yaml_content.len();

        group.throughput(Throughput::Bytes(content_size as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_memory_efficiency", doc_size),
            &yaml_content,
            |b, content| {
                b.iter(|| {
                    let yaml = Yaml::new();
                    yaml.load_str(std::hint::black_box(content)).unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_scanner_optimizations(c: &mut Criterion) {
    let mut group = c.benchmark_group("scanner_optimizations");

    // Test different string types for scanner performance
    let test_cases = vec![
        ("quoted_strings", generate_quoted_string_yaml(100)),
        ("plain_scalars", generate_plain_scalar_yaml(100)),
        ("mixed_content", generate_mixed_content_yaml(100)),
    ];

    for (test_name, content) in test_cases {
        group.bench_function(format!("scan_{test_name}"), |b| {
            b.iter(|| {
                let mut scanner = BasicScanner::new_eager(std::hint::black_box(content.clone()));
                let mut token_count = 0;
                while scanner.check_token() {
                    if let Ok(Some(_)) = scanner.get_token() {
                        token_count += 1;
                    } else {
                        break;
                    }
                }
                token_count
            });
        });
    }

    group.finish();
}

fn bench_parser_state_efficiency(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_state_efficiency");

    // Test different nesting depths for parser state management
    for depth in [5, 10, 20, 50].iter() {
        let yaml_content = generate_nested_yaml(*depth);

        group.bench_with_input(
            BenchmarkId::new("parse_nested", depth),
            &yaml_content,
            |b, content| {
                b.iter(|| {
                    let mut parser = BasicParser::new_eager(std::hint::black_box(content.clone()));
                    let mut event_count = 0;
                    while parser.check_event() {
                        if let Ok(Some(_)) = parser.get_event() {
                            event_count += 1;
                        } else {
                            break;
                        }
                    }
                    event_count
                });
            },
        );
    }

    group.finish();
}

fn bench_round_trip_efficiency(c: &mut Criterion) {
    let yaml = Yaml::new();
    let complex_yaml = generate_complex_document();

    c.bench_function("round_trip_performance", |b| {
        b.iter(|| {
            let parsed = yaml.load_str(std::hint::black_box(&complex_yaml)).unwrap();
            let serialized = yaml.dump_str(&parsed).unwrap();
            let reparsed = yaml.load_str(&serialized).unwrap();
            assert_eq!(parsed, reparsed);
        });
    });
}

fn bench_profiling_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("profiling_overhead");

    let yaml_content = generate_structured_yaml(100);

    // Benchmark without profiling
    group.bench_function("without_profiling", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&yaml_content)).unwrap()
        });
    });

    // Benchmark with profiling enabled
    group.bench_function("with_profiling", |b| {
        b.iter(|| {
            unsafe {
                std::env::set_var("RUST_YAML_PROFILE", "1");
            }
            let yaml = Yaml::new();
            let result = yaml.load_str(std::hint::black_box(&yaml_content)).unwrap();
            unsafe {
                std::env::remove_var("RUST_YAML_PROFILE");
            }
            result
        });
    });

    group.finish();
}

fn bench_string_interning_benefits(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");

    // Generate YAML with repeated string values
    let yaml_with_repeated_strings = generate_repeated_strings_yaml(200);

    group.bench_function("repeated_strings_parse", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&yaml_with_repeated_strings))
                .unwrap()
        });
    });

    group.finish();
}

fn bench_flow_vs_block_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("flow_vs_block");

    let block_yaml = generate_block_style_yaml(100);
    let flow_yaml = generate_flow_style_yaml(100);

    group.bench_function("block_style_parsing", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&block_yaml)).unwrap()
        });
    });

    group.bench_function("flow_style_parsing", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&flow_yaml)).unwrap()
        });
    });

    group.finish();
}

// Helper functions to generate test data

fn generate_structured_yaml(size: usize) -> String {
    let mut yaml = String::from("items:\n");
    for i in 0..size {
        yaml.push_str(&format!("  item_{}:\n", i));
        yaml.push_str(&format!("    id: {}\n", i));
        yaml.push_str(&format!("    name: 'Item {}'\n", i));
        yaml.push_str(&format!("    value: {}\n", i * 10));
        yaml.push_str("    active: true\n");
    }
    yaml
}

fn generate_quoted_string_yaml(count: usize) -> String {
    let mut yaml = String::new();
    for i in 0..count {
        yaml.push_str(&format!(
            "str_{}: \"This is a quoted string number {}\"\n",
            i, i
        ));
    }
    yaml
}

fn generate_plain_scalar_yaml(count: usize) -> String {
    let mut yaml = String::new();
    for i in 0..count {
        yaml.push_str(&format!("key_{}: value_{}\n", i, i));
    }
    yaml
}

fn generate_mixed_content_yaml(count: usize) -> String {
    let mut yaml = String::from("mixed_data:\n");
    for i in 0..count {
        match i % 4 {
            0 => yaml.push_str(&format!("  plain_{}: simple_value\n", i)),
            1 => yaml.push_str(&format!("  quoted_{}: \"quoted value {}\"\n", i, i)),
            2 => yaml.push_str(&format!("  number_{}: {}\n", i, i * 100)),
            3 => yaml.push_str(&format!("  bool_{}: {}\n", i, i % 2 == 0)),
            _ => unreachable!(),
        }
    }
    yaml
}

fn generate_nested_yaml(depth: usize) -> String {
    let mut yaml = String::from("root:\n");
    let mut current_indent = 2;

    for i in 0..depth {
        let indent = " ".repeat(current_indent);
        yaml.push_str(&format!("{}level_{}:\n", indent, i));
        yaml.push_str(&format!("{}  value: {}\n", indent, i));
        if i < depth - 1 {
            yaml.push_str(&format!("{}  nested:\n", indent));
            current_indent += 4;
        }
    }

    yaml
}

fn generate_complex_document() -> String {
    r#"
# Complex YAML document for performance testing
metadata:
  version: "1.0"
  created: "2024-01-01T00:00:00Z"
  author: "Performance Test"

configuration: &config
  timeout: 30
  retries: 3
  debug: false
  features:
    - logging
    - metrics
    - caching

services:
  web:
    <<: *config
    port: 8080
    host: localhost
    routes:
      - path: /api
        methods: [GET, POST, PUT, DELETE]
      - path: /health
        methods: [GET]

  database:
    <<: *config
    driver: postgresql
    host: db.example.com
    port: 5432
    credentials:
      username: app_user
      password: secret123

environments:
  - name: development
    active: true
    services: [web, database]
  - name: staging
    active: true
    services: [web, database]
  - name: production
    active: false
    services: [web, database]

data_samples:
  - id: 1
    name: "Sample 1"
    values: [1, 2, 3, 4, 5]
  - id: 2
    name: "Sample 2"
    values: [6, 7, 8, 9, 10]
"#
    .to_string()
}

fn generate_repeated_strings_yaml(count: usize) -> String {
    let mut yaml = String::from("repeated_data:\n");
    let common_strings = ["active", "inactive", "pending", "completed", "failed"];

    for i in 0..count {
        let status = common_strings[i % common_strings.len()];
        yaml.push_str(&format!("  item_{}:\n", i));
        yaml.push_str(&format!("    status: {}\n", status));
        yaml.push_str(&format!(
            "    priority: {}\n",
            common_strings[(i + 1) % common_strings.len()]
        ));
    }

    yaml
}

fn generate_block_style_yaml(count: usize) -> String {
    let mut yaml = String::from("block_structure:\n");
    for i in 0..count {
        yaml.push_str(&format!("  item_{}:\n", i));
        yaml.push_str(&format!("    name: Item {}\n", i));
        yaml.push_str("    tags:\n");
        yaml.push_str("      - important\n");
        yaml.push_str("      - processed\n");
    }
    yaml
}

fn generate_flow_style_yaml(count: usize) -> String {
    let mut yaml = String::from("flow_structure: {\n");
    for i in 0..count {
        if i > 0 {
            yaml.push_str(",\n");
        }
        yaml.push_str(&format!(
            "  item_{}: {{name: \"Item {}\", tags: [important, processed]}}",
            i, i
        ));
    }
    yaml.push_str("\n}");
    yaml
}

criterion_group!(
    performance_regression_benches,
    bench_baseline_performance,
    bench_memory_allocation_efficiency,
    bench_scanner_optimizations,
    bench_parser_state_efficiency,
    bench_round_trip_efficiency,
    bench_profiling_overhead,
    bench_string_interning_benefits,
    bench_flow_vs_block_performance
);
criterion_main!(performance_regression_benches);
