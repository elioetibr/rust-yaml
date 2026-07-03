//! Byte-fidelity property over the official YAML test suite:
//! every input that `RoundTripDocument::parse_all` ACCEPTS must be
//! reproduced byte-for-byte by concatenating the parsed documents.
//!
//! Inputs the parser rejects are fine (this is a fidelity property, not a
//! conformance property — conformance is `yaml_test_suite.rs`); an accepted
//! input that fails to round-trip is a hard failure, because it means the
//! span/source machinery silently rewrote bytes.
//!
//! Each case runs in a dedicated thread with a hard timeout, mirroring
//! `yaml_test_suite.rs` (the parser has known hangs on pathological inputs).

use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use rust_yaml::RoundTripDocument;

/// Hard per-case timeout, matching the conformance harness.
const PER_TEST_TIMEOUT: Duration = Duration::from_secs(2);

enum Outcome {
    /// Parsed and reproduced the input byte-for-byte.
    Faithful,
    /// Parser rejected the input (acceptable; not a fidelity concern).
    Rejected,
    /// Parsed but did NOT reproduce the input — a fidelity bug.
    Infidelity(String),
    /// Did not finish within the budget.
    Timeout,
}

fn check_case(input: String) -> Outcome {
    match RoundTripDocument::parse_all(&input) {
        Err(_) => Outcome::Rejected,
        Ok(docs) => {
            let joined: String = docs.iter().map(RoundTripDocument::source).collect();
            if joined == input {
                Outcome::Faithful
            } else {
                let diff_at = input
                    .bytes()
                    .zip(joined.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or_else(|| input.len().min(joined.len()));
                Outcome::Infidelity(format!(
                    "in {} bytes, out {} bytes, first diff at byte {diff_at}",
                    input.len(),
                    joined.len()
                ))
            }
        }
    }
}

fn check_with_timeout(input: String, timeout: Duration) -> Outcome {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(check_case(input));
    });
    rx.recv_timeout(timeout).unwrap_or(Outcome::Timeout)
}

#[test]
fn round_trip_fidelity_over_test_suite() {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    if !data_dir.exists() {
        eprintln!(
            "YAML test suite data not found at {}. Skipping.",
            data_dir.display()
        );
        eprintln!("Run: git submodule update --init");
        return;
    }

    let mut faithful = 0usize;
    let mut rejected = 0usize;
    let mut timeouts = 0usize;
    let mut violations: Vec<String> = Vec::new();

    let mut stack = vec![data_dir];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().is_some_and(|n| n == "in.yaml") {
                // Raw bytes; skip the few non-UTF-8 corpus files (the public
                // API is &str, so they cannot reach RoundTripDocument).
                let Ok(input) = fs::read_to_string(&path) else {
                    continue;
                };
                match check_with_timeout(input, PER_TEST_TIMEOUT) {
                    Outcome::Faithful => faithful += 1,
                    Outcome::Rejected => rejected += 1,
                    Outcome::Timeout => timeouts += 1,
                    Outcome::Infidelity(detail) => {
                        violations.push(format!("{}: {detail}", path.display()));
                    }
                }
            }
        }
    }

    println!(
        "round-trip fidelity: {faithful} faithful, {rejected} rejected, \
         {timeouts} timeouts, {} violations",
        violations.len()
    );

    assert!(
        violations.is_empty(),
        "{} accepted input(s) were not reproduced byte-for-byte:\n{}",
        violations.len(),
        violations.join("\n")
    );
    // Sanity floor: the property must actually exercise a meaningful share of
    // the corpus, not vacuously pass by rejecting everything.
    assert!(
        faithful >= 100,
        "only {faithful} faithful cases — the fidelity property barely ran"
    );
}
