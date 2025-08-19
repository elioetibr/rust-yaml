#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::write_with_newline)]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_yaml::{
    BasicComposer, BasicEmitter, BasicParser, BasicScanner, Composer, Emitter, Parser, Scanner,
    Yaml,
};
use std::fmt::Write;
use std::time::Duration;

/// Memory usage profiling benchmarks
fn bench_memory_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_scaling");
    group.measurement_time(Duration::from_secs(15));

    // Test memory scaling with document size
    for doc_size in [10, 100, 1000, 5000].iter() {
        let yaml_content = generate_large_yaml(*doc_size);
        let content_size = yaml_content.len();

        group.throughput(Throughput::Bytes(content_size as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_memory_scaling", doc_size),
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

fn bench_throughput_analysis(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput_analysis");

    // Different document types for throughput testing
    let test_cases = vec![
        ("simple_scalars", generate_scalar_heavy_yaml(1000)),
        ("nested_mappings", generate_mapping_heavy_yaml(500)),
        ("sequence_heavy", generate_sequence_heavy_yaml(1000)),
        ("mixed_structures", generate_mixed_yaml(500)),
    ];

    for (test_name, yaml_content) in test_cases {
        let content_size = yaml_content.len();
        group.throughput(Throughput::Bytes(content_size as u64));

        group.bench_function(format!("parse_throughput_{test_name}"), |b| {
            b.iter(|| {
                let yaml = Yaml::new();
                yaml.load_str(std::hint::black_box(&yaml_content)).unwrap()
            });
        });

        // Also test serialization throughput
        let yaml = Yaml::new();
        let parsed = yaml.load_str(&yaml_content).unwrap();

        group.bench_function(format!("serialize_throughput_{test_name}"), |b| {
            b.iter(|| yaml.dump_str(std::hint::black_box(&parsed)).unwrap());
        });
    }
    group.finish();
}

fn bench_component_profiling(c: &mut Criterion) {
    let yaml_content = generate_complex_yaml(200);

    c.bench_function("full_parsing_pipeline", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&yaml_content)).unwrap()
        });
    });

    // Profile individual components
    c.bench_function("scanner_only", |b| {
        b.iter(|| {
            let mut scanner = BasicScanner::new_eager(std::hint::black_box(yaml_content.clone()));
            let mut token_count = 0;
            while scanner.check_token() {
                if scanner.get_token().unwrap().is_some() {
                    token_count += 1;
                } else {
                    break;
                }
            }
            token_count
        });
    });

    c.bench_function("parser_only", |b| {
        b.iter(|| {
            let mut parser = BasicParser::new_eager(std::hint::black_box(yaml_content.clone()));
            let mut event_count = 0;
            while parser.check_event() {
                if parser.get_event().unwrap().is_some() {
                    event_count += 1;
                } else {
                    break;
                }
            }
            event_count
        });
    });

    c.bench_function("composer_only", |b| {
        b.iter(|| {
            let mut composer = BasicComposer::new_eager(std::hint::black_box(yaml_content.clone()));
            composer.compose_document().unwrap()
        });
    });

    // Profile serialization components
    let yaml = Yaml::new();
    let parsed = yaml.load_str(&yaml_content).unwrap();

    c.bench_function("emitter_only", |b| {
        b.iter(|| {
            let mut emitter = BasicEmitter::new();
            let mut output = Vec::new();
            emitter
                .emit(std::hint::black_box(&parsed), &mut output)
                .unwrap()
        });
    });
}

fn bench_allocation_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("allocation_patterns");

    // Test different allocation patterns
    let test_cases = vec![
        ("small_frequent", generate_many_small_docs()),
        ("large_infrequent", vec![generate_large_yaml(2000)]),
        ("mixed_sizes", generate_mixed_size_docs()),
    ];

    for (pattern_name, documents) in test_cases {
        group.bench_function(format!("parse_pattern_{pattern_name}"), |b| {
            b.iter(|| {
                let yaml = Yaml::new();
                for doc in &documents {
                    yaml.load_str(std::hint::black_box(doc)).unwrap();
                }
            });
        });
    }
    group.finish();
}

fn bench_concurrent_performance(c: &mut Criterion) {
    let yaml_content = generate_complex_yaml(100);
    let yaml_contents: Vec<String> = (0..10).map(|_| yaml_content.clone()).collect();

    c.bench_function("sequential_parsing", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            for content in &yaml_contents {
                yaml.load_str(std::hint::black_box(content)).unwrap();
            }
        });
    });

    // Note: For true concurrent benchmarks, we'd use rayon or similar,
    // but for now we'll simulate the overhead
    c.bench_function("simulated_concurrent_overhead", |b| {
        b.iter(|| {
            for content in &yaml_contents {
                let yaml = Yaml::new(); // New instance per "thread"
                yaml.load_str(std::hint::black_box(content)).unwrap();
            }
        });
    });
}

fn bench_worst_case_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("worst_case_scenarios");
    group.measurement_time(Duration::from_secs(20));

    // Deep nesting
    let deeply_nested = generate_deeply_nested_yaml(100);
    group.bench_function("deep_nesting", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&deeply_nested)).unwrap()
        });
    });

    // Wide mapping (many keys)
    let wide_mapping = generate_wide_mapping_yaml(1000);
    group.bench_function("wide_mapping", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&wide_mapping)).unwrap()
        });
    });

    // Long sequences
    let long_sequence = generate_long_sequence_yaml(5000);
    group.bench_function("long_sequence", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&long_sequence)).unwrap()
        });
    });

    // Complex key structures
    let complex_keys = generate_complex_keys_yaml(50);
    group.bench_function("complex_keys", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            yaml.load_str(std::hint::black_box(&complex_keys)).unwrap()
        });
    });

    group.finish();
}

// Helper functions to generate test data

fn generate_large_yaml(size: usize) -> String {
    let mut yaml = String::from("items:\n");
    for i in 0..size {
        writeln!(yaml, "  item_{}:", i).unwrap();
        writeln!(yaml, "    id: {}", i).unwrap();
        writeln!(yaml, "    name: 'Item {}'", i).unwrap();
        yaml.push_str("    active: true\n");
        yaml.push_str("    tags: [important, processed]\n");
    }
    yaml
}

fn generate_scalar_heavy_yaml(count: usize) -> String {
    let mut yaml = String::new();
    for i in 0..count {
        writeln!(yaml, "scalar_{}: 'value_{}'", i, i).unwrap();
    }
    yaml
}

fn generate_mapping_heavy_yaml(depth: usize) -> String {
    let mut yaml = String::from("root:\n");
    let mut current_indent = 2;

    for i in 0..depth {
        let indent = " ".repeat(current_indent);
        writeln!(yaml, "{}level_{}:", indent, i).unwrap();
        writeln!(yaml, "{}  value: {}", indent, i).unwrap();
        if i < depth - 1 {
            writeln!(yaml, "{}  nested:", indent).unwrap();
            current_indent += 4;
        }
    }
    yaml
}

fn generate_sequence_heavy_yaml(count: usize) -> String {
    let mut yaml = String::from("sequences:\n");
    for i in 0..count {
        writeln!(yaml, "  - item_{}", i).unwrap();
    }
    yaml
}

fn generate_mixed_yaml(complexity: usize) -> String {
    let mut yaml = String::from("mixed_structure:\n");

    // Add sequences
    yaml.push_str("  sequences:\n");
    for i in 0..complexity / 2 {
        writeln!(yaml, "    - seq_item_{}", i).unwrap();
    }

    // Add mappings
    yaml.push_str("  mappings:\n");
    for i in 0..complexity / 2 {
        writeln!(yaml, "    key_{}: value_{}", i, i).unwrap();
    }

    // Add nested structures
    yaml.push_str("  nested:\n");
    yaml.push_str("    deep:\n");
    yaml.push_str("      structure:\n");
    yaml.push_str("        with: values\n");

    yaml
}

fn generate_complex_yaml(complexity: usize) -> String {
    let mut yaml = String::from("# Complex YAML document\n");
    yaml.push_str("metadata:\n");
    yaml.push_str("  version: '1.0'\n");
    yaml.push_str("  created: '2024-01-01'\n\n");

    // Add base config with anchor
    yaml.push_str("base_config: &base\n");
    yaml.push_str("  timeout: 30\n");
    yaml.push_str("  retries: 3\n\n");

    // Add services with aliases and complex structures
    yaml.push_str("services:\n");
    for i in 0..complexity {
        writeln!(yaml, "  service_{}:", i).unwrap();
        yaml.push_str("    <<: *base\n");
        writeln!(yaml, "    name: 'Service {}'", i).unwrap();
        yaml.push_str("    endpoints:\n");
        writeln!(yaml, "      - '/api/v1/service{}'", i).unwrap();
        writeln!(yaml, "      - '/health/service{}'", i).unwrap();
        yaml.push_str("    config:\n");
        writeln!(yaml, "      port: {}", 8000 + i).unwrap();
        yaml.push_str("      features: [logging, metrics]\n");
    }

    yaml
}

fn generate_many_small_docs() -> Vec<String> {
    (0..100)
        .map(|i| format!("small_doc_{}:\n  value: {}\n  active: true\n", i, i))
        .collect()
}

fn generate_mixed_size_docs() -> Vec<String> {
    let mut docs = Vec::new();

    // Small docs
    for i in 0..50 {
        docs.push(format!("small_{}: value_{}\n", i, i));
    }

    // Medium docs
    for i in 0..20 {
        let mut doc = format!("medium_{}:\n", i);
        for j in 0..10 {
            write!(doc, "  item_{}: value_{}\n", j, j).unwrap();
        }
        docs.push(doc);
    }

    // Large docs
    for _i in 0..5 {
        docs.push(generate_large_yaml(100));
    }

    docs
}

fn generate_deeply_nested_yaml(depth: usize) -> String {
    let mut yaml = String::from("root:\n");
    for i in 0..depth {
        let indent = " ".repeat((i + 1) * 2);
        writeln!(yaml, "{}level_{}:", indent, i).unwrap();
    }

    let final_indent = " ".repeat((depth + 1) * 2);
    writeln!(yaml, "{}final_value: 'deeply nested'", final_indent).unwrap();
    yaml
}

fn generate_wide_mapping_yaml(width: usize) -> String {
    let mut yaml = String::from("wide_mapping:\n");
    for i in 0..width {
        writeln!(yaml, "  key_{}: 'value_{}'", i, i).unwrap();
    }
    yaml
}

fn generate_long_sequence_yaml(length: usize) -> String {
    let mut yaml = String::from("long_sequence:\n");
    for i in 0..length {
        writeln!(yaml, "  - item_{}", i).unwrap();
    }
    yaml
}

fn generate_complex_keys_yaml(count: usize) -> String {
    let mut yaml = String::from("complex_keys:\n");
    for i in 0..count {
        writeln!(yaml, "  ? [key_{}, type_{}]", i, i).unwrap();
        writeln!(yaml, "  : value_{}", i).unwrap();
    }
    yaml
}

criterion_group!(
    profiling_benches,
    bench_memory_scaling,
    bench_throughput_analysis,
    bench_component_profiling,
    bench_allocation_patterns,
    bench_concurrent_performance,
    bench_worst_case_scenarios
);
criterion_main!(profiling_benches);
