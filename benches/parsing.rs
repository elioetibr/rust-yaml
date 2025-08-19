#![allow(clippy::uninlined_format_args)]
#![allow(clippy::needless_raw_string_hashes)]

use criterion::{Criterion, criterion_group, criterion_main};
use rust_yaml::Yaml;

fn bench_simple_scalars(c: &mut Criterion) {
    let yaml = Yaml::new();

    c.bench_function("parse_simple_integer", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box("42")).unwrap());
    });

    c.bench_function("parse_simple_string", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box("hello world")).unwrap());
    });

    c.bench_function("parse_simple_bool", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box("true")).unwrap());
    });

    c.bench_function("parse_simple_float", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box("3.14159")).unwrap());
    });
}

fn bench_flow_collections(c: &mut Criterion) {
    let yaml = Yaml::new();

    c.bench_function("parse_flow_sequence_small", |b| {
        b.iter(|| {
            yaml.load_str(std::hint::black_box("[1, 2, 3, 4, 5]"))
                .unwrap()
        });
    });

    c.bench_function("parse_flow_mapping_small", |b| {
        b.iter(|| {
            yaml.load_str(std::hint::black_box(
                r#"{"key1": "value1", "key2": "value2"}"#,
            ))
            .unwrap()
        });
    });
}

fn bench_block_collections(c: &mut Criterion) {
    let yaml = Yaml::new();

    let block_sequence = r#"
- item1
- item2
- item3
- item4
- item5
"#;

    c.bench_function("parse_block_sequence_small", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box(block_sequence)).unwrap());
    });

    let block_mapping = r#"
key1: value1
key2: value2
key3: value3
key4: value4
key5: value5
"#;

    c.bench_function("parse_block_mapping_small", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box(block_mapping)).unwrap());
    });
}

fn bench_nested_structures(c: &mut Criterion) {
    let yaml = Yaml::new();

    let nested_yaml = r#"
users:
  - name: "Alice"
    age: 30
    email: "alice@example.com"
    roles:
      - admin
      - user
  - name: "Bob"
    age: 25
    email: "bob@example.com"
    roles:
      - user
config:
  database:
    host: "localhost"
    port: 5432
    credentials:
      username: "dbuser"
      password: "secret123"  # pragma: allowlist secret
  cache:
    enabled: true
    ttl: 3600
"#;

    c.bench_function("parse_nested_structure", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box(nested_yaml)).unwrap());
    });
}

fn bench_multi_document(c: &mut Criterion) {
    let yaml = Yaml::new();

    let multi_doc = r#"
document: 1
data: [1, 2, 3]
---
document: 2
data: [4, 5, 6]
---
document: 3
data: [7, 8, 9]
"#;

    c.bench_function("parse_multi_document", |b| {
        b.iter(|| yaml.load_all_str(std::hint::black_box(multi_doc)).unwrap());
    });
}

fn bench_large_sequence(c: &mut Criterion) {
    let yaml = Yaml::new();

    // Create a larger sequence for performance testing
    let mut large_seq = String::from("[");
    for i in 0..1000 {
        if i > 0 {
            large_seq.push_str(", ");
        }
        large_seq.push_str(&i.to_string());
    }
    large_seq.push(']');

    c.bench_function("parse_large_sequence", |b| {
        b.iter(|| yaml.load_str(std::hint::black_box(&large_seq)).unwrap());
    });
}

criterion_group!(
    benches,
    bench_simple_scalars,
    bench_flow_collections,
    bench_block_collections,
    bench_nested_structures,
    bench_multi_document,
    bench_large_sequence
);
criterion_main!(benches);
