// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]
use libfuzzer_sys::fuzz_target;
use rust_yaml::{Limits, LoaderType, Yaml, YamlConfig};

// Under the strictest resource limits, parsing arbitrary input must still
// terminate without panicking. This exercises the `ResourceTracker` caps
// (depth, anchors, alias nodes, collection size, complexity score, ...) and
// guards against a malformed document defeating any of them (#28).
fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let config = YamlConfig {
            limits: Limits::strict(),
            loader_type: LoaderType::Safe,
            ..YamlConfig::default()
        };
        let _ = Yaml::with_config(config).load_str(text);
    }
});
