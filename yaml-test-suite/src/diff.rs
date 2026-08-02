// SPDX-FileCopyrightText: YAML test suite contributors
//
// SPDX-License-Identifier: MIT

//! Compare expected vs actual event-tree lines.

const NO_DIFF: &str = "  (no difference found)";
const MISSING: &str = "<missing>";

pub fn show_diff(expected: &[String], actual: &[String]) -> String {
    let max = expected.len().max(actual.len());
    for i in 0..max {
        let exp = expected.get(i).map(String::as_str).unwrap_or(MISSING);
        let act = actual.get(i).map(String::as_str).unwrap_or(MISSING);
        if exp != act {
            return format!(
                "  line {n}: expected: {exp:?}\n  line {n}:   actual: {act:?}",
                n = i + 1,
            );
        }
    }
    NO_DIFF.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn equal_vectors_report_no_difference() {
        assert_eq!(
            show_diff(&s(&["+STR", "-STR"]), &s(&["+STR", "-STR"])),
            "  (no difference found)"
        );
    }

    #[test]
    fn empty_vectors_report_no_difference() {
        assert_eq!(show_diff(&[], &[]), "  (no difference found)");
    }

    #[test]
    fn first_diff_is_reported_with_one_based_line_numbers() {
        assert_eq!(
            show_diff(
                &s(&["+STR", "+DOC", "=VAL :a"]),
                &s(&["+STR", "+DOC", "=VAL :b"])
            ),
            "  line 3: expected: \"=VAL :a\"\n  line 3:   actual: \"=VAL :b\""
        );
    }

    #[test]
    fn missing_actual_lines_are_shown_as_marker() {
        assert_eq!(
            show_diff(&s(&["+STR", "-STR"]), &s(&["+STR"])),
            "  line 2: expected: \"-STR\"\n  line 2:   actual: \"<missing>\""
        );
    }

    #[test]
    fn extra_actual_lines_are_shown_as_marker_on_expected_side() {
        assert_eq!(
            show_diff(&s(&["+STR"]), &s(&["+STR", "-STR"])),
            "  line 2: expected: \"<missing>\"\n  line 2:   actual: \"-STR\""
        );
    }
}
