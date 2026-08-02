// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]
use libfuzzer_sys::fuzz_target;
use rust_yaml::Yaml;

// Round-trip invariant: if a document loads, then dumping it and loading the
// dumped form must reproduce an equal value. Failing to re-parse our own
// output, or a value that mutates across the trip, is a correctness bug (#28).
fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    let yaml = Yaml::new();
    let Ok(first) = yaml.load_str(text) else {
        return;
    };
    let Ok(dumped) = yaml.dump_str(&first) else {
        return;
    };
    match yaml.load_str(&dumped) {
        Ok(second) => assert_eq!(
            first, second,
            "round-trip changed the value; dumped form was:\n{dumped}"
        ),
        Err(e) => panic!("dumped output failed to re-parse: {e}\n{dumped}"),
    }
});
