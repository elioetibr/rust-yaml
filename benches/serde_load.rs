// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "serde")]
use criterion::{Criterion, criterion_group, criterion_main};
use serde::Deserialize;
use std::hint::black_box;

#[derive(Deserialize)]
#[allow(dead_code)]
struct Config {
    name: String,
    version: u32,
    enabled: bool,
    flags: Vec<String>,
}

const SMALL: &str = "name: rust\nversion: 11\nenabled: true\nflags: [a, b, c]\n";

const MEDIUM: &str = "name: rust\nversion: 11\nenabled: true\n\
flags:\n  - a\n  - b\n  - c\n  - d\n  - e\n  - f\n  - g\n  - h\n  - i\n  - j\n";

const LARGE: &str = include_str!("./fixtures/config_large.yaml");

#[allow(clippy::unwrap_used)]
fn bench_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("serde_load");
    for (label, input) in [("small", SMALL), ("medium", MEDIUM), ("large", LARGE)] {
        group.bench_function(format!("rust_yaml/{label}"), |b| {
            b.iter(|| {
                let _: Config = rust_yaml::from_str(black_box(input)).unwrap();
            });
        });
        group.bench_function(format!("serde_yaml/{label}"), |b| {
            b.iter(|| {
                let _: Config = serde_yaml::from_str(black_box(input)).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_load);
criterion_main!(benches);
