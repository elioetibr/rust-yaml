use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use rust_yaml::{BasicScanner, Scanner, ZeroScanner};
use std::time::Duration;

fn benchmark_scalar_scanning(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalar_scanning");

    let long_string = "a".repeat(1000);
    let very_long_string = "word ".repeat(500);

    let inputs = vec![
        ("short", "hello"),
        (
            "medium",
            "this is a longer scalar value with multiple words",
        ),
        ("long", long_string.as_str()),
        ("very_long", very_long_string.as_str()),
    ];

    for (name, input) in inputs {
        group.bench_with_input(BenchmarkId::new("traditional", name), &input, |b, input| {
            b.iter(|| {
                let mut scanner = BasicScanner::new((*input).to_string());
                while let Ok(Some(_token)) = scanner.get_token() {
                    // Process tokens
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("zero_copy", name), &input, |b, input| {
            b.iter(|| {
                let mut scanner = ZeroScanner::new(input);
                while scanner.current_char().is_some() {
                    // Scan a scalar if possible
                    if matches!(scanner.current_char(), Some(ch) if ch.is_alphabetic()) {
                        let _token = scanner.scan_plain_scalar_zero_copy();
                    } else {
                        scanner.advance();
                    }
                }
                scanner.reset();
            });
        });
    }

    group.finish();
}

fn benchmark_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    group.measurement_time(Duration::from_secs(10));

    let large_yaml = format!(
        "items:\n{}",
        (0..1000)
            .map(|i| format!("  - name: item_{}\n    value: {}", i, i * 2))
            .collect::<Vec<_>>()
            .join("\n")
    );

    group.bench_function("traditional_large_yaml", |b| {
        b.iter(|| {
            let mut scanner = BasicScanner::new(large_yaml.clone());
            let mut token_count = 0;
            while let Ok(Some(_token)) = scanner.get_token() {
                token_count += 1;
            }
            std::hint::black_box(token_count);
        });
    });

    group.bench_function("zero_copy_large_yaml", |b| {
        b.iter(|| {
            let mut scanner = ZeroScanner::new(&large_yaml);
            let mut token_count = 0;
            while scanner.current_char().is_some() {
                if matches!(scanner.current_char(), Some(ch) if ch.is_alphabetic()) {
                    let _token = scanner.scan_plain_scalar_zero_copy();
                    token_count += 1;
                } else {
                    scanner.advance();
                }
            }
            scanner.reset();
            std::hint::black_box(token_count);
        });
    });

    group.finish();
}

fn benchmark_character_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("character_operations");

    let test_input = "hello world this is a test input with many characters";

    group.bench_function("traditional_char_advance", |b| {
        b.iter(|| {
            let mut scanner = BasicScanner::new(test_input.to_owned());
            let mut char_count = 0;
            while let Ok(Some(_token)) = scanner.get_token() {
                char_count += 1;
            }
            std::hint::black_box(char_count);
        });
    });

    group.bench_function("zero_copy_char_advance", |b| {
        b.iter(|| {
            let mut scanner = ZeroScanner::new(test_input);
            let mut char_count = 0;
            while scanner.current_char().is_some() {
                scanner.advance();
                char_count += 1;
            }
            scanner.reset();
            std::hint::black_box(char_count);
        });
    });

    group.finish();
}

fn benchmark_identifier_scanning(c: &mut Criterion) {
    let mut group = c.benchmark_group("identifier_scanning");

    let identifiers = vec![
        "short_id",
        "medium_length_identifier",
        "very_long_identifier_with_many_underscores_and_numbers_123",
    ];

    for identifier in identifiers {
        group.bench_with_input(
            BenchmarkId::new("zero_copy_identifier", identifier.len()),
            &identifier,
            |b, input| {
                b.iter(|| {
                    let mut scanner = ZeroScanner::new(input);
                    let _result = scanner.scan_identifier_zero_copy();
                    scanner.reset();
                });
            },
        );
    }

    group.finish();
}

fn benchmark_token_pool_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_allocation");

    group.bench_function("token_pool_reuse", |b| {
        b.iter(|| {
            let mut scanner = ZeroScanner::new("test input");

            // Simulate multiple parsing sessions with pool reuse
            for _ in 0..100 {
                let _stats = scanner.stats();
                scanner.reset();
            }
        });
    });

    group.bench_function("fresh_allocation", |b| {
        b.iter(|| {
            // Simulate creating new scanners each time
            for _ in 0..100 {
                let scanner = ZeroScanner::new("test input");
                let _stats = scanner.stats();
            }
        });
    });

    group.finish();
}

fn benchmark_slice_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("slice_operations");

    let input = "key1: value1\nkey2: value2\nkey3: value3";

    group.bench_function("zero_copy_slicing", |b| {
        b.iter(|| {
            let mut scanner = ZeroScanner::new(input);
            let start_pos = scanner.position;

            // Advance through some characters
            for _ in 0..4 {
                scanner.advance();
            }

            let _slice = scanner.slice_from(start_pos);
            scanner.reset();
        });
    });

    group.bench_function("string_allocation", |b| {
        b.iter(|| {
            let input_chars: Vec<char> = input.chars().collect();
            let _substring = input_chars[0..4].iter().collect::<String>();
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_scalar_scanning,
    benchmark_memory_usage,
    benchmark_character_operations,
    benchmark_identifier_scanning,
    benchmark_token_pool_allocation,
    benchmark_slice_operations
);
criterion_main!(benches);
