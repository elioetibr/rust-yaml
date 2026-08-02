// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "serde")]
use criterion::{Criterion, criterion_group, criterion_main};
use serde::Serialize;
use std::hint::black_box;

#[derive(Serialize)]
struct Config {
    name: String,
    version: u32,
    enabled: bool,
    flags: Vec<String>,
}

fn make(n: usize) -> Config {
    Config {
        name: "rust".into(),
        version: 11,
        enabled: true,
        flags: (0..n).map(|i| format!("flag-{i}")).collect(),
    }
}

fn bench_dump(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde_dump");
    for (label, n) in [("small", 3usize), ("medium", 10), ("large", 50)] {
        let cfg = make(n);
        group.bench_function(format!("rust_yaml/{label}"), |b| {
            b.iter(|| {
                let _ = rust_yaml::to_string(black_box(&cfg)).unwrap();
            });
        });
        group.bench_function(format!("serde_yaml/{label}"), |b| {
            b.iter(|| {
                let _ = serde_yaml::to_string(black_box(&cfg)).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_dump);
criterion_main!(benches);
