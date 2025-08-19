#![allow(clippy::uninlined_format_args)]
#![allow(clippy::approx_constant)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::needless_raw_string_hashes)]

use criterion::{Criterion, criterion_group, criterion_main};
use indexmap::IndexMap;
use rust_yaml::{Value, Yaml};

fn create_test_values() -> Vec<(&'static str, Value)> {
    vec![
        ("simple_null", Value::Null),
        ("simple_bool", Value::Bool(true)),
        ("simple_int", Value::Int(42)),
        ("simple_float", Value::Float(std::f64::consts::PI)),
        ("simple_string", Value::String("hello world".to_string())),
        (
            "flow_sequence",
            Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        ),
        ("flow_mapping", {
            let mut map = IndexMap::new();
            map.insert(
                Value::String("key1".to_string()),
                Value::String("value1".to_string()),
            );
            map.insert(
                Value::String("key2".to_string()),
                Value::String("value2".to_string()),
            );
            Value::Mapping(map)
        }),
        ("nested_structure", {
            let mut root = IndexMap::new();

            // Users array
            let users = vec![
                {
                    let mut user = IndexMap::new();
                    user.insert(
                        Value::String("name".to_string()),
                        Value::String("Alice".to_string()),
                    );
                    user.insert(Value::String("age".to_string()), Value::Int(30));
                    user.insert(
                        Value::String("roles".to_string()),
                        Value::Sequence(vec![
                            Value::String("admin".to_string()),
                            Value::String("user".to_string()),
                        ]),
                    );
                    Value::Mapping(user)
                },
                {
                    let mut user = IndexMap::new();
                    user.insert(
                        Value::String("name".to_string()),
                        Value::String("Bob".to_string()),
                    );
                    user.insert(Value::String("age".to_string()), Value::Int(25));
                    user.insert(
                        Value::String("roles".to_string()),
                        Value::Sequence(vec![Value::String("user".to_string())]),
                    );
                    Value::Mapping(user)
                },
            ];

            // Config object
            let mut config = IndexMap::new();
            let mut database = IndexMap::new();
            database.insert(
                Value::String("host".to_string()),
                Value::String("localhost".to_string()),
            );
            database.insert(Value::String("port".to_string()), Value::Int(5432));

            config.insert(
                Value::String("database".to_string()),
                Value::Mapping(database),
            );
            config.insert(Value::String("debug".to_string()), Value::Bool(true));

            root.insert(Value::String("users".to_string()), Value::Sequence(users));
            root.insert(Value::String("config".to_string()), Value::Mapping(config));

            Value::Mapping(root)
        }),
    ]
}

fn bench_dump_scalars(c: &mut Criterion) {
    let yaml = Yaml::new();
    let test_values = create_test_values();

    for (name, value) in &test_values[0..5] {
        // First 5 are scalars
        c.bench_function(&format!("dump_{name}"), |b| {
            b.iter(|| yaml.dump_str(std::hint::black_box(value)).unwrap());
        });
    }
}

fn bench_dump_collections(c: &mut Criterion) {
    let yaml = Yaml::new();
    let test_values = create_test_values();

    for (name, value) in &test_values[5..7] {
        // Collections
        c.bench_function(&format!("dump_{name}"), |b| {
            b.iter(|| yaml.dump_str(std::hint::black_box(value)).unwrap());
        });
    }
}

fn bench_dump_nested(c: &mut Criterion) {
    let yaml = Yaml::new();
    let test_values = create_test_values();

    let nested_value = &test_values[7].1; // Nested structure

    c.bench_function("dump_nested_structure", |b| {
        b.iter(|| yaml.dump_str(std::hint::black_box(nested_value)).unwrap());
    });
}

fn bench_dump_large_sequence(c: &mut Criterion) {
    let yaml = Yaml::new();

    // Create a large sequence
    let large_sequence: Vec<Value> = (0..1000).map(Value::Int).collect();
    let large_value = Value::Sequence(large_sequence);

    c.bench_function("dump_large_sequence", |b| {
        b.iter(|| yaml.dump_str(std::hint::black_box(&large_value)).unwrap());
    });
}

fn bench_dump_large_mapping(c: &mut Criterion) {
    let yaml = Yaml::new();

    // Create a large mapping
    let mut large_mapping = IndexMap::new();
    for i in 0..1000 {
        large_mapping.insert(
            Value::String(format!("key_{}", i)),
            Value::String(format!("value_{}", i)),
        );
    }
    let large_value = Value::Mapping(large_mapping);

    c.bench_function("dump_large_mapping", |b| {
        b.iter(|| yaml.dump_str(std::hint::black_box(&large_value)).unwrap());
    });
}

fn bench_roundtrip(c: &mut Criterion) {
    let yaml = Yaml::new();

    let yaml_content = r#"
name: "rust-yaml"
version: "0.1.0"
features:
  - fast
  - safe
  - reliable
config:
  debug: true
  max_depth: 100
  cache:
    enabled: true
    size: 1000
users:
  - name: "Alice"
    permissions: ["read", "write"]
  - name: "Bob"
    permissions: ["read"]
"#;

    c.bench_function("roundtrip_parse_and_dump", |b| {
        b.iter(|| {
            let parsed = yaml.load_str(std::hint::black_box(yaml_content)).unwrap();
            yaml.dump_str(&parsed).unwrap()
        });
    });
}

fn bench_multi_document_dump(c: &mut Criterion) {
    let yaml = Yaml::new();

    let documents = vec![
        {
            let mut doc = IndexMap::new();
            doc.insert(Value::String("document".to_string()), Value::Int(1));
            doc.insert(
                Value::String("type".to_string()),
                Value::String("config".to_string()),
            );
            Value::Mapping(doc)
        },
        {
            let mut doc = IndexMap::new();
            doc.insert(Value::String("document".to_string()), Value::Int(2));
            doc.insert(
                Value::String("type".to_string()),
                Value::String("data".to_string()),
            );
            Value::Mapping(doc)
        },
        {
            let mut doc = IndexMap::new();
            doc.insert(Value::String("document".to_string()), Value::Int(3));
            doc.insert(
                Value::String("type".to_string()),
                Value::String("metadata".to_string()),
            );
            Value::Mapping(doc)
        },
    ];

    c.bench_function("dump_multi_document", |b| {
        b.iter(|| yaml.dump_all_str(std::hint::black_box(&documents)).unwrap());
    });
}

criterion_group!(
    benches,
    bench_dump_scalars,
    bench_dump_collections,
    bench_dump_nested,
    bench_dump_large_sequence,
    bench_dump_large_mapping,
    bench_roundtrip,
    bench_multi_document_dump
);
criterion_main!(benches);
