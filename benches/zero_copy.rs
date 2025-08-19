//! Zero-copy parsing benchmarks

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rust_yaml::{
    BasicComposer, Composer, OptimizedComposer, OptimizedValue, ReducedAllocComposer, Value,
};

const SMALL_YAML: &str = r#"
name: "John Doe"
age: 30
active: true
"#;

const MEDIUM_YAML: &str = r#"
users:
  - name: "Alice"
    age: 28
    skills: ["Python", "Rust", "JavaScript"]
  - name: "Bob"
    age: 32
    skills: ["Java", "C++", "Go"]
  - name: "Charlie"
    age: 25
    skills: ["Ruby", "PHP", "Swift"]
config:
  debug: false
  timeout: 30
  retries: 3
  servers:
    - host: "server1.example.com"
      port: 8080
    - host: "server2.example.com"
      port: 8081
"#;

const LARGE_YAML: &str = r#"
database:
  connections:
    primary:
      host: "db1.example.com"
      port: 5432
      database: "production"
      username: "dbuser"
      password: "secret"  # pragma: allowlist secret
      pool:
        min: 10
        max: 100
        timeout: 30
    replica:
      host: "db2.example.com"
      port: 5432
      database: "production"
      username: "dbuser"
      password: "secret"  # pragma: allowlist secret
      pool:
        min: 5
        max: 50
        timeout: 30
services:
  web:
    instances: 10
    memory: "2G"
    cpu: 2
    environment:
      NODE_ENV: "production"
      LOG_LEVEL: "info"
      API_KEY: "xyz123"  # pragma: allowlist secret
  worker:
    instances: 5
    memory: "1G"
    cpu: 1
    environment:
      NODE_ENV: "production"
      LOG_LEVEL: "debug"
      QUEUE_SIZE: 1000
monitoring:
  metrics:
    enabled: true
    interval: 60
    retention: 30
  alerts:
    - name: "High CPU"
      threshold: 90
      duration: 300
    - name: "Low Memory"
      threshold: 10
      duration: 180
    - name: "Error Rate"
      threshold: 5
      duration: 60
"#;

fn benchmark_standard_composer(c: &mut Criterion) {
    let mut group = c.benchmark_group("standard_composer");

    for (name, yaml) in &[
        ("small", SMALL_YAML),
        ("medium", MEDIUM_YAML),
        ("large", LARGE_YAML),
    ] {
        group.throughput(Throughput::Bytes(yaml.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), yaml, |b, yaml| {
            b.iter(|| {
                let mut composer = BasicComposer::new(yaml.to_string());
                let _ = std::hint::black_box(composer.compose_document());
            });
        });
    }

    group.finish();
}

fn benchmark_optimized_composer(c: &mut Criterion) {
    let mut group = c.benchmark_group("optimized_composer");

    for (name, yaml) in &[
        ("small", SMALL_YAML),
        ("medium", MEDIUM_YAML),
        ("large", LARGE_YAML),
    ] {
        group.throughput(Throughput::Bytes(yaml.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(name), yaml, |b, yaml| {
            b.iter(|| {
                let mut composer = ReducedAllocComposer::new(yaml.to_string());
                let _ = std::hint::black_box(composer.compose_document());
            });
        });
    }

    group.finish();
}

fn benchmark_string_heavy(c: &mut Criterion) {
    // Generate YAML with many string values to highlight borrowing benefits
    let mut yaml = String::from("strings:\n");
    for i in 0..100 {
        yaml.push_str(&format!(
            "  - \"This is string number {} with some content\"\n",
            i
        ));
    }

    let mut group = c.benchmark_group("string_heavy");
    group.throughput(Throughput::Bytes(yaml.len() as u64));

    group.bench_function("standard", |b| {
        b.iter(|| {
            let mut composer = BasicComposer::new(yaml.clone());
            let _ = std::hint::black_box(composer.compose_document());
        });
    });

    group.bench_function("optimized", |b| {
        b.iter(|| {
            let mut composer = ReducedAllocComposer::new(yaml.clone());
            let _ = std::hint::black_box(composer.compose_document());
        });
    });

    group.finish();
}

fn benchmark_anchor_heavy(c: &mut Criterion) {
    let yaml = r#"
base: &base
  name: "Base Config"
  timeout: 30
  retries: 3

config1:
  <<: *base
  server: "server1"

config2:
  <<: *base
  server: "server2"

config3:
  <<: *base
  server: "server3"
"#;

    let mut group = c.benchmark_group("anchor_heavy");
    group.throughput(Throughput::Bytes(yaml.len() as u64));

    group.bench_function("standard", |b| {
        b.iter(|| {
            let mut composer = BasicComposer::new(yaml.to_string());
            let _ = std::hint::black_box(composer.compose_document());
        });
    });

    group.bench_function("optimized", |b| {
        b.iter(|| {
            let mut composer = ReducedAllocComposer::new(yaml.to_string());
            let _ = std::hint::black_box(composer.compose_document());
        });
    });

    group.finish();
}

fn benchmark_memory_allocations(c: &mut Criterion) {
    let yaml = MEDIUM_YAML;

    c.bench_function("allocations_standard", |b| {
        b.iter(|| {
            let mut composer = BasicComposer::new(yaml.to_string());
            let result = composer.compose_document().unwrap().unwrap();
            // Force evaluation to ensure all allocations happen
            if let Value::Mapping(map) = result {
                for (k, v) in map {
                    std::hint::black_box(k);
                    std::hint::black_box(v);
                }
            }
        });
    });

    c.bench_function("allocations_optimized", |b| {
        b.iter(|| {
            let mut composer = ReducedAllocComposer::new(yaml.to_string());
            let result = composer.compose_document().unwrap().unwrap();
            // Force evaluation
            if let OptimizedValue::Mapping(map) = result {
                for (k, v) in map.iter() {
                    std::hint::black_box(k);
                    std::hint::black_box(v);
                }
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_standard_composer,
    benchmark_optimized_composer,
    benchmark_string_heavy,
    benchmark_anchor_heavy,
    benchmark_memory_allocations
);
criterion_main!(benches);
