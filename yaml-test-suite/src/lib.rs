// SPDX-FileCopyrightText: YAML test suite contributors
//
// SPDX-License-Identifier: MIT

//! YAML test suite integration for rust-yaml.
//!
//! Dev-only, not published. Drives the upstream `yaml/yaml-test-suite` corpus
//! against `rust-yaml`'s parser using the canonical `test.event` tree format.
//!
//! The crate is organized into small modules along single-responsibility
//! lines, each owning one concern:
//!
//! * [`escape`] – escape sequences for scalar values in the tree DSL
//! * [`event_tree`] – convert parser events into the tree DSL
//! * [`diff`] – compare expected vs actual event trees
//! * [`data_case`] – load `data/` directory test cases from disk
//! * [`runner`] – run a single test case and classify the result

pub mod data_case;
pub mod diff;
pub mod escape;
pub mod event_tree;
pub mod runner;

pub use data_case::{DataTestCase, load_all_tests};
pub use diff::show_diff;
pub use event_tree::events_to_tree;
pub use runner::{TestResult, run_test};
