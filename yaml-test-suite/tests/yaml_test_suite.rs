// SPDX-FileCopyrightText: YAML test suite contributors
//
// SPDX-License-Identifier: MIT

//! Integration tests using the official YAML test suite.
//!
//! Runs all test cases from <https://github.com/yaml/yaml-test-suite>
//! using the `data/` directory format and reports pass/fail statistics.
//!
//! Each test runs in a dedicated thread with a hard timeout — rust-yaml's
//! parser currently has infinite-loop bugs on some pathological inputs, and
//! a per-test budget keeps one hang from stalling the whole sweep.

use std::fs;
use std::path::Path;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use yaml_test_suite::{DataTestCase, TestResult, load_all_tests, run_test};

/// Hard per-test timeout. Inputs that don't return within this budget are
/// reported as `Timeout` failures instead of blocking the run.
const PER_TEST_TIMEOUT: Duration = Duration::from_secs(2);

#[test]
fn yaml_test_suite() {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");

    if !data_dir.exists() {
        eprintln!(
            "YAML test suite data not found at {}. Skipping.",
            data_dir.display()
        );
        eprintln!("Run: git submodule update --init");
        return;
    }

    let all_tests = load_all_tests(&data_dir);

    let mut total = 0usize;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut timed_out = 0usize;
    let mut failures: Vec<String> = Vec::new();

    for tc in &all_tests {
        total += 1;

        match run_test_with_timeout(tc, PER_TEST_TIMEOUT) {
            TestOutcome::Pass => passed += 1,
            TestOutcome::Fail(reason) => {
                failed += 1;
                failures.push(format!("{} ({}):\n{reason}", tc.id, tc.name));
            }
            TestOutcome::Timeout => {
                timed_out += 1;
                failures.push(format!("{} ({}):\ntimeout (parser hang)", tc.id, tc.name));
            }
        }
    }

    let pass_rate = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    eprintln!();
    eprintln!("╔══════════════════════════════════════╗");
    eprintln!("║     YAML Test Suite Results          ║");
    eprintln!("╠══════════════════════════════════════╣");
    eprintln!("║  Total:    {total:>5}                     ║");
    eprintln!("║  Passed:   {passed:>5}                     ║");
    eprintln!("║  Failed:   {failed:>5}                     ║");
    eprintln!("║  Timeout:  {timed_out:>5}                     ║");
    eprintln!("║  Rate:    {pass_rate:>5.1}%                     ║");
    eprintln!("╚══════════════════════════════════════╝");

    // Categorize failures (timeout messages are counted separately above).
    let mut cat_wrong_accept = 0usize;
    let mut cat_wrong_reject = 0usize;
    let mut cat_wrong_events = 0usize;
    let mut cat_timeout = 0usize;

    for f in &failures {
        if f.contains("timeout (parser hang)") {
            cat_timeout += 1;
        } else if f.contains("expected error but parser succeeded") {
            cat_wrong_accept += 1;
        } else if f.contains("unexpected error") {
            cat_wrong_reject += 1;
        } else {
            cat_wrong_events += 1;
        }
    }

    eprintln!();
    eprintln!("  Failure breakdown:");
    eprintln!("    Wrong accept (should reject): {cat_wrong_accept}");
    eprintln!("    Wrong reject (should accept): {cat_wrong_reject}");
    eprintln!("    Wrong events (diff):          {cat_wrong_events}");
    eprintln!("    Timeout (parser hang):        {cat_timeout}");

    write_failure_report(&failures);

    assert!(
        pass_rate >= 10.0,
        "Pass rate {pass_rate:.1}% is below minimum threshold of 10%"
    );
}

/// Write a categorized failure listing to `target/yaml-test-suite-failures.txt`
/// for offline analysis. One section per failure category.
fn write_failure_report(failures: &[String]) {
    let mut wrong_accept = Vec::new();
    let mut wrong_reject = Vec::new();
    let mut wrong_events = Vec::new();
    let mut timeouts = Vec::new();
    for f in failures {
        if f.contains("timeout (parser hang)") {
            timeouts.push(f.as_str());
        } else if f.contains("expected error but parser succeeded") {
            wrong_accept.push(f.as_str());
        } else if f.contains("unexpected error") {
            wrong_reject.push(f.as_str());
        } else {
            wrong_events.push(f.as_str());
        }
    }

    let mut out = String::new();
    out.push_str(&format!(
        "# yaml-test-suite failures ({} total)\n\n",
        failures.len()
    ));

    let sections: &[(&str, &Vec<&str>)] = &[
        ("Timeouts (parser hangs)", &timeouts),
        ("Wrong reject (parser failed on valid YAML)", &wrong_reject),
        (
            "Wrong accept (parser succeeded on invalid YAML)",
            &wrong_accept,
        ),
        ("Wrong events (parsed but tree differs)", &wrong_events),
    ];
    for (title, items) in sections {
        out.push_str(&format!("## {} — {}\n\n", title, items.len()));
        for f in items.iter() {
            out.push_str(f);
            out.push_str("\n\n");
        }
    }

    let report = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("target")
        .join("yaml-test-suite-failures.txt");
    let _ = fs::create_dir_all(report.parent().unwrap());
    if let Err(e) = fs::write(&report, out) {
        eprintln!("warning: failed to write failure report: {e}");
    } else {
        eprintln!("Failure report written to {}", report.display());
    }
}

/// Outcome local to the integration test driver. Adds a `Timeout` variant
/// that the pure library `TestResult` doesn't expose.
enum TestOutcome {
    Pass,
    Fail(String),
    Timeout,
}

/// Run `tc` in a dedicated thread with a hard `timeout`.
///
/// On timeout the worker thread is *abandoned* (cannot be cancelled safely
/// in Rust) and the harness moves on. Acceptable for a single run; the
/// thread will eventually finish or terminate with the process.
fn run_test_with_timeout(tc: &DataTestCase, timeout: Duration) -> TestOutcome {
    let tc_clone = tc.clone();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let _ = tx.send(run_test(&tc_clone));
    });

    match rx.recv_timeout(timeout) {
        Ok(TestResult::Pass) => TestOutcome::Pass,
        Ok(TestResult::Fail(reason)) => TestOutcome::Fail(reason),
        Err(mpsc::RecvTimeoutError::Timeout) => TestOutcome::Timeout,
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            TestOutcome::Fail("worker thread panicked".to_string())
        }
    }
}
