// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]
use libfuzzer_sys::fuzz_target;
use rust_yaml::Yaml;

// Parsing arbitrary input must never panic. A panic is a denial of service:
// `[profile.release]` sets `panic = "abort"`, so a panic terminates the host
// process. Returning an `Err` is the only acceptable failure mode (#28).
fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = Yaml::new().load_str(text);
    }
});
