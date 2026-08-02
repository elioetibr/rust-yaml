// SPDX-FileCopyrightText: YAML test suite contributors
//
// SPDX-License-Identifier: MIT

//! Escape sequences for scalar values in the yaml-test-suite tree DSL.

pub fn escape_value(s: &str) -> String {
    // Backslash MUST be replaced first so backslashes inserted by the
    // following substitutions are not re-mangled.
    s.replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .replace('\x08', "\\b")
        .replace('\0', "\\0")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backslash_is_escaped_as_double_backslash() {
        assert_eq!(escape_value("\\"), "\\\\");
    }

    #[test]
    fn newline_is_escaped_as_backslash_n() {
        assert_eq!(escape_value("\n"), "\\n");
    }

    #[test]
    fn carriage_return_is_escaped_as_backslash_r() {
        assert_eq!(escape_value("\r"), "\\r");
    }

    #[test]
    fn tab_is_escaped_as_backslash_t() {
        assert_eq!(escape_value("\t"), "\\t");
    }

    #[test]
    fn backspace_is_escaped_as_backslash_b() {
        assert_eq!(escape_value("\x08"), "\\b");
    }

    #[test]
    fn null_is_escaped_as_backslash_zero() {
        assert_eq!(escape_value("\0"), "\\0");
    }

    #[test]
    fn empty_string_returns_empty() {
        assert_eq!(escape_value(""), "");
    }

    #[test]
    fn plain_text_is_unchanged() {
        assert_eq!(escape_value("hello world"), "hello world");
    }

    #[test]
    fn pre_existing_backslash_n_in_input_is_double_escaped() {
        // Critical: backslash MUST be replaced first, otherwise the \\ we just
        // inserted gets mangled when we substitute other escapes that contain
        // backslashes. This locks in the ordering.
        assert_eq!(escape_value("\\n"), "\\\\n");
    }
}
