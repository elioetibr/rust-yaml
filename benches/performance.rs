#![allow(clippy::needless_raw_string_hashes)] // Test YAML strings

use criterion::{criterion_group, criterion_main, Criterion};
use rust_yaml::{BasicScanner, Scanner, Yaml, ZeroScanner};

fn parse_simple_document(c: &mut Criterion) {
    let yaml_str = r#"
name: John Doe
age: 30
email: john@example.com
active: true
"#;

    c.bench_function("parse_simple", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            std::hint::black_box(yaml.load_str(yaml_str).unwrap());
        });
    });
}

fn parse_complex_document(c: &mut Criterion) {
    let yaml_str = r#"
users:
  - name: Alice
    age: 28
    skills: [python, rust, javascript]
    projects:
      - name: Project A
        status: active
        budget: 50000
      - name: Project B
        status: completed
        budget: 75000
  - name: Bob
    age: 35
    skills: [java, go, kubernetes]
    projects:
      - name: Project C
        status: planning
        budget: 100000
settings:
  timeout: 30
  retries: 3
  debug: false
  endpoints:
    api: https://api.example.com
    auth: https://auth.example.com
"#;

    c.bench_function("parse_complex", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            std::hint::black_box(yaml.load_str(yaml_str).unwrap());
        });
    });
}

fn parse_with_anchors(c: &mut Criterion) {
    let yaml_str = r#"
defaults: &defaults
  timeout: 30
  retries: 3
  pool_size: 10

development:
  <<: *defaults
  host: localhost
  debug: true

production:
  <<: *defaults
  host: prod.example.com
  pool_size: 50
  debug: false
"#;

    c.bench_function("parse_anchors", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            std::hint::black_box(yaml.load_str(yaml_str).unwrap());
        });
    });
}

fn parse_large_sequence(c: &mut Criterion) {
    let mut items = Vec::new();
    for i in 0..1000 {
        items.push(format!("  - id: {}\n    value: item_{}", i, i));
    }
    let yaml_str = format!("items:\n{}", items.join("\n"));

    c.bench_function("parse_large_sequence", |b| {
        b.iter(|| {
            let yaml = Yaml::new();
            std::hint::black_box(yaml.load_str(&yaml_str).unwrap());
        });
    });
}

fn parse_comparison_zero_copy(c: &mut Criterion) {
    let yaml_str = r#"
name: John Doe
age: 30
email: john@example.com
active: true
settings:
  theme: dark
  notifications: enabled
  language: en-US
"#;

    let mut group = c.benchmark_group("parsing_comparison");

    group.bench_function("traditional_scanner", |b| {
        b.iter(|| {
            let mut scanner = BasicScanner::new(yaml_str.to_string());
            let mut token_count = 0;
            while let Ok(Some(_token)) = scanner.get_token() {
                token_count += 1;
            }
            std::hint::black_box(token_count);
        });
    });

    group.bench_function("zero_copy_scanner", |b| {
        b.iter(|| {
            let mut scanner = ZeroScanner::new(yaml_str);
            let mut scalar_count = 0;
            while scanner.current_char().is_some() {
                if matches!(scanner.current_char(), Some(ch) if ch.is_alphabetic()) {
                    let _token = scanner.scan_plain_scalar_zero_copy();
                    scalar_count += 1;
                } else {
                    scanner.advance();
                }
            }
            scanner.reset();
            std::hint::black_box(scalar_count);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    parse_simple_document,
    parse_complex_document,
    parse_with_anchors,
    parse_large_sequence,
    parse_comparison_zero_copy
);
criterion_main!(benches);
