//! Benchmarks for streaming YAML parsing

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_yaml::{stream_from_string, BasicParser, Parser, StreamConfig};

/// Generate a large YAML document for testing
fn generate_large_yaml(entries: usize) -> String {
    let mut yaml = String::new();
    yaml.push_str("---\n");
    yaml.push_str("metadata:\n");
    yaml.push_str("  version: 1.0\n");
    yaml.push_str("  generated: 2025-08-16\n");
    yaml.push_str("entries:\n");

    for i in 0..entries {
        yaml.push_str(&format!("  - id: {}\n", i));
        yaml.push_str(&format!("    name: \"Entry {}\"\n", i));
        yaml.push_str(&format!("    value: {}\n", i * 10));
        yaml.push_str("    tags:\n");
        yaml.push_str("      - tag1\n");
        yaml.push_str("      - tag2\n");
        yaml.push_str("      - tag3\n");
        yaml.push_str("    metadata:\n");
        yaml.push_str(&format!("      created: \"2025-08-{:02}\"\n", (i % 30) + 1));
        yaml.push_str("      modified: null\n");
    }

    yaml.push_str("...\n");
    yaml
}

fn benchmark_standard_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("standard_parser");

    for size in &[100, 1000, 10000] {
        let yaml = generate_large_yaml(*size);
        group.throughput(Throughput::Bytes(yaml.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &yaml, |b, yaml| {
            b.iter(|| {
                let mut parser = BasicParser::new(yaml.clone());
                let mut event_count = 0;
                while parser.check_event() {
                    if let Ok(Some(_event)) = parser.get_event() {
                        event_count += 1;
                    }
                }
                std::hint::black_box(event_count)
            });
        });
    }

    group.finish();
}

fn benchmark_streaming_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_parser");

    for size in &[100, 1000, 10000] {
        let yaml = generate_large_yaml(*size);
        group.throughput(Throughput::Bytes(yaml.len() as u64));

        group.bench_with_input(BenchmarkId::from_parameter(size), &yaml, |b, yaml| {
            b.iter(|| {
                let parser = stream_from_string(yaml.clone(), StreamConfig::default());
                let mut event_count = 0;

                for _event in parser.flatten() {
                    event_count += 1;
                }
                std::hint::black_box(event_count)
            });
        });
    }

    group.finish();
}

fn benchmark_incremental_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_parsing");

    let yaml = generate_large_yaml(1000);
    group.throughput(Throughput::Bytes(yaml.len() as u64));

    // Standard - parse all at once
    group.bench_function("all_at_once", |b| {
        b.iter(|| {
            let mut parser = BasicParser::new(yaml.clone());
            let mut events = Vec::new();
            while parser.check_event() {
                if let Ok(Some(event)) = parser.get_event() {
                    events.push(event);
                }
            }
            std::hint::black_box(events.len())
        });
    });

    // Streaming - parse incrementally
    group.bench_function("incremental", |b| {
        b.iter(|| {
            let mut parser = stream_from_string(yaml.clone(), StreamConfig::default());
            let mut event_count = 0;

            // Parse in chunks
            loop {
                match parser.parse_next() {
                    Ok(true) => {
                        while let Some(event) = parser.next_event() {
                            event_count += 1;
                            std::hint::black_box(event);
                        }
                    }
                    Ok(false) => break,
                    Err(_) => break,
                }
            }
            std::hint::black_box(event_count)
        });
    });

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");

    // Test with different buffer configurations
    let yaml = generate_large_yaml(5000);
    group.throughput(Throughput::Bytes(yaml.len() as u64));

    group.bench_function("default_buffer", |b| {
        b.iter(|| {
            let config = StreamConfig::default();
            let parser = stream_from_string(yaml.clone(), config);
            let mut event_count = 0;

            for event in parser {
                if event.is_ok() {
                    event_count += 1;
                }
            }
            std::hint::black_box(event_count)
        });
    });

    group.bench_function("large_buffer", |b| {
        b.iter(|| {
            let config = StreamConfig::large_file();
            let parser = stream_from_string(yaml.clone(), config);
            let mut event_count = 0;

            for event in parser {
                if event.is_ok() {
                    event_count += 1;
                }
            }
            std::hint::black_box(event_count)
        });
    });

    group.bench_function("low_memory", |b| {
        b.iter(|| {
            let config = StreamConfig::low_memory();
            let parser = stream_from_string(yaml.clone(), config);
            let mut event_count = 0;

            for event in parser {
                if event.is_ok() {
                    event_count += 1;
                }
            }
            std::hint::black_box(event_count)
        });
    });

    group.finish();
}

fn benchmark_chunk_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("chunk_sizes");

    let yaml = generate_large_yaml(2000);
    group.throughput(Throughput::Bytes(yaml.len() as u64));

    for chunk_size in &[1024, 4096, 8192, 16384, 65536] {
        group.bench_with_input(
            BenchmarkId::from_parameter(chunk_size),
            chunk_size,
            |b, &chunk_size| {
                b.iter(|| {
                    let config = StreamConfig {
                        chunk_size,
                        ..StreamConfig::default()
                    };

                    let parser = stream_from_string(yaml.clone(), config);
                    let mut event_count = 0;

                    for event in parser {
                        if event.is_ok() {
                            event_count += 1;
                        }
                    }
                    std::hint::black_box(event_count)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark multi-document YAML files
fn benchmark_multi_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("multi_document");

    // Generate multi-document YAML
    let mut yaml = String::new();
    for doc in 0..100 {
        yaml.push_str("---\n");
        yaml.push_str(&format!("document: {}\n", doc));
        yaml.push_str("data:\n");
        for i in 0..50 {
            yaml.push_str(&format!("  item{}: value{}\n", i, i));
        }
    }

    group.throughput(Throughput::Bytes(yaml.len() as u64));

    group.bench_function("standard", |b| {
        b.iter(|| {
            let mut parser = BasicParser::new(yaml.clone());
            let mut doc_count = 0;
            let mut event_count = 0;

            while parser.check_event() {
                if let Ok(Some(event)) = parser.get_event() {
                    if matches!(event.event_type, rust_yaml::EventType::DocumentStart { .. }) {
                        doc_count += 1;
                    }
                    event_count += 1;
                }
            }
            std::hint::black_box((doc_count, event_count))
        });
    });

    group.bench_function("streaming", |b| {
        b.iter(|| {
            let parser = stream_from_string(yaml.clone(), StreamConfig::default());
            let mut doc_count = 0;
            let mut event_count = 0;

            for event in parser.flatten() {
                if matches!(event.event_type, rust_yaml::EventType::DocumentStart { .. }) {
                    doc_count += 1;
                }
                event_count += 1;
            }
            std::hint::black_box((doc_count, event_count))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_standard_parser,
    benchmark_streaming_parser,
    benchmark_incremental_parsing,
    benchmark_memory_usage,
    benchmark_chunk_sizes,
    benchmark_multi_document
);
criterion_main!(benches);
