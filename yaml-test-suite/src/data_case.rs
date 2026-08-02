// SPDX-FileCopyrightText: YAML test suite contributors
//
// SPDX-License-Identifier: MIT

//! `data/` directory test case model + loader.
//!
//! The loader functions ([`load_all_tests`], [`load_single_test`]) are added
//! via TDD in their own cycle; this file currently exposes only the data
//! structure so the runner module can depend on it.

use std::fs;
use std::path::Path;

/// Discover every test case under `data_dir`.
///
/// Recognises 4-char alphanumeric directory names as test IDs. A directory
/// that has an `in.yaml` directly is a single test case; otherwise it is
/// treated as a multi-subtest container and its numeric subdirectories
/// (`00/`, `01/`, …) are loaded with composite IDs (`PARENT/SUB`).
pub fn load_all_tests(data_dir: &Path) -> Vec<DataTestCase> {
    let mut tests = Vec::new();
    for entry in sorted_alphanum_test_dirs(data_dir) {
        let test_id = entry.file_name().to_string_lossy().to_string();
        let dir = entry.path();
        // Try loading as a single test first; on None the dir is a
        // multi-subtest container — descend into its sorted subdirs.
        if let Some(tc) = load_single_test(&dir, test_id.clone()) {
            tests.push(tc);
        } else {
            for sub in sorted_subdirs(&dir) {
                let sub_id = format!("{test_id}/{}", sub.file_name().to_string_lossy());
                if let Some(tc) = load_single_test(&sub.path(), sub_id) {
                    tests.push(tc);
                }
            }
        }
    }
    tests
}

fn sorted_alphanum_test_dirs(root: &Path) -> Vec<fs::DirEntry> {
    let mut entries: Vec<_> = fs::read_dir(root)
        .expect("failed to read data directory")
        .filter_map(Result::ok)
        .filter(|e| is_test_id(&e.file_name().to_string_lossy()))
        .collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    entries
}

fn sorted_subdirs(parent: &Path) -> Vec<fs::DirEntry> {
    let Ok(read) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut entries: Vec<_> = read
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(fs::DirEntry::file_name);
    entries
}

fn is_test_id(name: &str) -> bool {
    name.len() == 4 && name.chars().all(|c| c.is_ascii_alphanumeric())
}

fn load_single_test(dir: &Path, id: String) -> Option<DataTestCase> {
    let in_yaml = dir.join("in.yaml");
    // Read in.yaml or treat the directory as "not a single test" — this
    // handles both missing files and unreadable files (e.g., when
    // `in.yaml` is itself a directory) uniformly.
    let yaml = fs::read_to_string(&in_yaml).ok()?;

    let name = read_optional_trimmed(dir, "===");
    let expected_events = read_event_lines(dir, "test.event");
    let expect_error = dir.join("error").exists();

    Some(DataTestCase {
        id,
        name,
        yaml,
        expected_events,
        expect_error,
    })
}

/// Read a file under `dir/name` and return its trimmed contents, or empty
/// string when the file is absent or unreadable.
fn read_optional_trimmed(dir: &Path, name: &str) -> String {
    let path = dir.join(name);
    if !path.exists() {
        return String::new();
    }
    fs::read_to_string(path)
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Read a file under `dir/name` and return its non-empty lines.
fn read_event_lines(dir: &Path, name: &str) -> Vec<String> {
    let path = dir.join(name);
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// A single test case discovered in a yaml-test-suite `data/` tree.
#[derive(Debug, Clone)]
pub struct DataTestCase {
    /// Test ID (e.g. `"229Q"` or `"2G84/00"`).
    pub id: String,
    /// Human-readable name from the `===` file (empty when absent).
    pub name: String,
    /// Raw YAML input from `in.yaml`.
    pub yaml: String,
    /// Expected event lines from `test.event` (empty when absent).
    pub expected_events: Vec<String>,
    /// `true` when an `error` marker file is present — parser must reject input.
    pub expect_error: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn fresh() -> TempDir {
        tempfile::tempdir().expect("create tempdir")
    }

    #[test]
    fn load_single_test_returns_none_when_in_yaml_is_absent() {
        let dir = fresh();
        assert!(load_single_test(dir.path(), "X".to_string()).is_none());
    }

    #[test]
    fn load_single_test_reads_yaml_and_sets_id_when_only_in_yaml_exists() {
        let dir = fresh();
        fs::write(dir.path().join("in.yaml"), "42\n").unwrap();
        let tc = load_single_test(dir.path(), "ABCD".to_string()).expect("Some");
        assert_eq!(tc.id, "ABCD");
        assert_eq!(tc.yaml, "42\n");
        assert_eq!(tc.name, "");
        assert!(tc.expected_events.is_empty());
        assert!(!tc.expect_error);
    }

    #[test]
    fn load_single_test_populates_name_from_triple_equals_file() {
        let dir = fresh();
        fs::write(dir.path().join("in.yaml"), "x").unwrap();
        fs::write(dir.path().join("==="), "  Human Name  \n").unwrap();
        let tc = load_single_test(dir.path(), "Y".into()).unwrap();
        assert_eq!(tc.name, "Human Name");
    }

    #[test]
    fn load_single_test_reads_event_lines_skipping_blank_ones() {
        let dir = fresh();
        fs::write(dir.path().join("in.yaml"), "x").unwrap();
        fs::write(
            dir.path().join("test.event"),
            "+STR\n\n+DOC\n=VAL :x\n-DOC\n-STR\n",
        )
        .unwrap();
        let tc = load_single_test(dir.path(), "Y".into()).unwrap();
        assert_eq!(
            tc.expected_events,
            vec!["+STR", "+DOC", "=VAL :x", "-DOC", "-STR"]
        );
    }

    #[test]
    fn load_single_test_sets_expect_error_when_error_marker_exists() {
        let dir = fresh();
        fs::write(dir.path().join("in.yaml"), "x").unwrap();
        fs::write(dir.path().join("error"), "").unwrap();
        let tc = load_single_test(dir.path(), "Y".into()).unwrap();
        assert!(tc.expect_error);
    }

    // ── load_all_tests ─────────────────────────────────────────────

    fn mk_test(parent: &Path, id: &str, yaml: &str) {
        let d = parent.join(id);
        fs::create_dir(&d).unwrap();
        fs::write(d.join("in.yaml"), yaml).unwrap();
    }

    #[test]
    fn load_all_tests_returns_empty_for_empty_directory() {
        let root = fresh();
        assert!(load_all_tests(root.path()).is_empty());
    }

    #[test]
    fn load_all_tests_discovers_single_test_dirs_with_in_yaml() {
        let root = fresh();
        mk_test(root.path(), "ABCD", "v1");
        mk_test(root.path(), "EFGH", "v2");
        let tests = load_all_tests(root.path());
        assert_eq!(tests.len(), 2);
        // Sorted by name
        assert_eq!(tests[0].id, "ABCD");
        assert_eq!(tests[1].id, "EFGH");
    }

    #[test]
    fn load_all_tests_skips_non_4_char_alphanumeric_directories() {
        let root = fresh();
        mk_test(root.path(), "ABCD", "v");
        // These should be skipped:
        fs::create_dir(root.path().join("tags")).unwrap();
        fs::create_dir(root.path().join("name")).unwrap();
        fs::create_dir(root.path().join("ABC-")).unwrap(); // not alphanumeric
        fs::create_dir(root.path().join("ABCDE")).unwrap(); // too long
        let tests = load_all_tests(root.path());
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].id, "ABCD");
    }

    #[test]
    fn load_all_tests_expands_multi_subtest_dirs_with_slash_ids() {
        let root = fresh();
        let parent = root.path().join("WXYZ");
        fs::create_dir(&parent).unwrap();
        // No in.yaml directly → multi-subtest layout
        for sub in &["00", "01"] {
            let d = parent.join(sub);
            fs::create_dir(&d).unwrap();
            fs::write(d.join("in.yaml"), format!("v{sub}")).unwrap();
        }
        let tests = load_all_tests(root.path());
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].id, "WXYZ/00");
        assert_eq!(tests[1].id, "WXYZ/01");
    }

    #[test]
    fn load_all_tests_panics_with_helpful_message_when_unreadable_root() {
        let result =
            std::panic::catch_unwind(|| load_all_tests(Path::new("/this/does/not/exist/anywhere")));
        assert!(result.is_err(), "expected panic on unreadable data_dir");
    }

    #[test]
    fn load_single_test_returns_none_when_in_yaml_path_is_a_directory() {
        // Covers the `Err` branch of `fs::read_to_string(&in_yaml)` — when
        // the path exists but cannot be read as a file.
        let dir = fresh();
        fs::create_dir(dir.path().join("in.yaml")).unwrap();
        assert!(load_single_test(dir.path(), "X".into()).is_none());
    }

    #[test]
    fn load_all_tests_skips_subtest_subdirs_that_have_no_in_yaml() {
        // Covers the None branch inside the multi-subtest loop.
        let root = fresh();
        let parent = root.path().join("MULT");
        fs::create_dir(&parent).unwrap();
        let sub = parent.join("00");
        fs::create_dir(&sub).unwrap(); // empty subdir — no in.yaml
        let tests = load_all_tests(root.path());
        assert!(tests.is_empty());
    }

    #[test]
    fn sorted_subdirs_returns_empty_when_path_is_unreadable() {
        // Covers the early-return branch of `sorted_subdirs`.
        assert!(sorted_subdirs(Path::new("/no/such/path/anywhere")).is_empty());
    }
}
