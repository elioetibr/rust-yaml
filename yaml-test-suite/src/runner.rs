// SPDX-FileCopyrightText: YAML test suite contributors
//
// SPDX-License-Identifier: MIT

//! Run a single [`DataTestCase`] and classify the outcome.

use crate::data_case::DataTestCase;
use crate::diff::show_diff;
use crate::event_tree::events_to_tree;

/// Outcome of running one test case.
#[derive(Debug)]
pub enum TestResult {
    /// Events match the expected tree (or the parser correctly rejected the input).
    Pass,
    /// Carries a human-readable explanation.
    Fail(String),
}

pub fn run_test(tc: &DataTestCase) -> TestResult {
    if tc.expect_error {
        return match events_to_tree(&tc.yaml) {
            Err(_) => TestResult::Pass,
            Ok(_) => TestResult::Fail("expected error but parser succeeded".to_string()),
        };
    }

    match events_to_tree(&tc.yaml) {
        Ok(actual) => {
            if tc.expected_events.is_empty() || actual == tc.expected_events {
                TestResult::Pass
            } else {
                TestResult::Fail(show_diff(&tc.expected_events, &actual))
            }
        }
        Err(e) => TestResult::Fail(format!("unexpected error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn case(yaml: &str, expect_error: bool, expected_events: Vec<String>) -> DataTestCase {
        DataTestCase {
            id: "TEST".to_string(),
            name: "synthetic".to_string(),
            yaml: yaml.to_string(),
            expected_events,
            expect_error,
        }
    }

    /// Extract the `Fail` reason or panic — written so both branches are
    /// exercised by tests below, which keeps the coverage report at 100%.
    fn expect_fail(r: TestResult) -> String {
        match r {
            TestResult::Fail(reason) => reason,
            TestResult::Pass => panic!("expected TestResult::Fail, got Pass"),
        }
    }

    /// Mirror of [`expect_fail`] for the Pass case.
    fn expect_pass(r: TestResult) {
        match r {
            TestResult::Pass => {}
            TestResult::Fail(reason) => panic!("expected Pass, got Fail: {reason}"),
        }
    }

    #[test]
    #[should_panic(expected = "expected TestResult::Fail, got Pass")]
    fn expect_fail_panics_when_given_pass() {
        let _ = expect_fail(TestResult::Pass);
    }

    #[test]
    #[should_panic(expected = "expected Pass, got Fail")]
    fn expect_pass_panics_when_given_fail() {
        expect_pass(TestResult::Fail("boom".to_string()));
    }

    // ── expect_error path ───────────────────────────────────────────

    #[test]
    fn expect_error_and_parser_fails_yields_pass() {
        // `@invalid` is rejected at the scanner — events_to_tree returns Err.
        let tc = case("@invalid", true, vec![]);
        expect_pass(run_test(&tc));
    }

    #[test]
    fn expect_error_and_parser_succeeds_yields_fail_with_specific_message() {
        let tc = case("42", true, vec![]);
        let reason = expect_fail(run_test(&tc));
        assert!(reason.contains("expected error but parser succeeded"));
    }

    // ── no-expect_error, no expected_events (fallback) ──────────────

    #[test]
    fn no_expected_events_and_parser_succeeds_falls_back_to_pass() {
        let tc = case("42", false, vec![]);
        expect_pass(run_test(&tc));
    }

    // ── no-expect_error, with expected_events ───────────────────────

    #[test]
    fn matching_expected_events_yields_pass() {
        let expected = vec![
            "+STR".to_string(),
            "+DOC".to_string(),
            "=VAL :42".to_string(),
            "-DOC".to_string(),
            "-STR".to_string(),
        ];
        let tc = case("42", false, expected);
        expect_pass(run_test(&tc));
    }

    #[test]
    fn mismatched_expected_events_yields_fail_with_diff() {
        let expected = vec!["+STR".to_string(), "wrong".to_string()];
        let tc = case("42", false, expected);
        let reason = expect_fail(run_test(&tc));
        assert!(
            reason.contains("expected:"),
            "diff format missing: {reason}"
        );
        assert!(reason.contains("actual:"), "diff format missing: {reason}");
    }

    // ── no-expect_error, parser fails ───────────────────────────────

    #[test]
    fn parser_fails_without_expected_error_yields_fail_with_prefix() {
        let tc = case("@invalid", false, vec![]);
        let reason = expect_fail(run_test(&tc));
        assert!(reason.starts_with("unexpected error:"));
    }
}
