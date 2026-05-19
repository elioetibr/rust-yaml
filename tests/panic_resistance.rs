//! Regression tests proving that crafted-malformed YAML never panics
//! the parser. Each input here is shaped to exercise the parser-state
//! stack edges that previously used `.unwrap()` (#17, #18) or other
//! panic-on-input paths.
//!
//! Failure mode without the fix: the test harness aborts with a
//! `panicked at 'called Option::unwrap() on a None value'`. Pass
//! condition: every input produces either `Ok` or `Err` — never
//! a panic.

use rust_yaml::Yaml;
use std::panic::{AssertUnwindSafe, catch_unwind};

const MALFORMED_INPUTS: &[(&str, &str)] = &[
    // Flow-sequence close with nothing on the state stack (#17 site 1)
    ("bare flow seq close", "]"),
    ("bare flow seq close after start", "[}"),
    ("flow seq close after key marker", "? ]"),
    // Flow-mapping close with nothing on the state stack (#17 site 3)
    ("bare flow map close", "}"),
    ("flow mapping close after seq start", "[}"),
    // Inline-wrapped key edge (#17 site 2)
    ("nested explicit key with truncation", "?\n a: b\n?"),
    // Misc malformed forms that hammer scanner/parser corners
    ("dangling colon", ":"),
    ("dangling dash", "-"),
    ("only directive marker", "---"),
    ("only doc end", "..."),
    ("nested brackets unclosed", "[[[[[[[[[[[[[[[[[[[["),
    ("nested braces unclosed", "{{{{{{{{{{{{{{{{{{{{"),
    ("flow seq with bare commas", "[,,,,]"),
    ("flow map with bare colons", "{::::}"),
    ("alt close then open", "]["),
    ("explicit-key without colon then close", "? a ]"),
    ("explicit-key without colon then map close", "? a }"),
    // Indented edge cases that touched scanner indent stack (#18)
    ("leading tab indent", "\t- a\n\t- b"),
    ("only indent then nothing", "   "),
];

#[test]
fn malformed_inputs_never_panic() {
    let yaml = Yaml::new();
    for (label, input) in MALFORMED_INPUTS {
        // Discard the load result inside the closure: we only care
        // whether the call returned at all, not what it returned.
        // Returning `Result<Value, Error>` from the closure trips
        // `clippy::result-large-err` (Rust 1.95+) because `Error` is
        // ~160 bytes.
        let result = catch_unwind(AssertUnwindSafe(|| {
            let _ = yaml.load_str(input);
        }));
        assert!(
            result.is_ok(),
            "input '{label}' (`{input:?}`) panicked — must return Err instead"
        );
    }
}
