<!--
SPDX-FileCopyrightText: Rust Yaml contributors

SPDX-License-Identifier: MIT OR Apache-2.0
-->

# Fuzzing

Coverage-guided fuzz targets for `rust-yaml`, built on
[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) and libFuzzer.

## Targets

- `load_str` — `Yaml::load_str` must never panic on arbitrary input
  (`panic = "abort"` in release makes any panic a denial of service).
- `load_str_strict` — the same, under `Limits::strict()`, exercising every
  `ResourceTracker` cap.
- `roundtrip` — `load → dump → load` must reproduce an equal value.

## Running

Requires a nightly toolchain and `cargo-fuzz`:

```sh
cargo install cargo-fuzz
cargo +nightly fuzz run load_str
cargo +nightly fuzz run load_str_strict
cargo +nightly fuzz run roundtrip
```

Append `-- -max_total_time=60` to bound a run to 60 seconds.

## Notes

This crate is a standalone workspace, excluded from the parent `rust-yaml`
workspace, so ordinary `cargo build` / `cargo test` never compile it.

`roundtrip` currently surfaces issue #66 (numeric mapping keys mis-parsed);
the `Fuzz` CI workflow runs that target non-blocking until #66 is fixed.
