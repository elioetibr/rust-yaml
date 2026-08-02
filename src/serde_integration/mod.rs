// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Serde integration for rust-yaml.
//!
//! Provides a full serde data format (Serializer + Deserializer) and
//! impls of `Serialize` / `Deserialize` for [`crate::Value`].
//!
//! All items are gated by the `serde` feature.

#[cfg(feature = "serde")]
mod de;
#[cfg(feature = "serde")]
mod error;
#[cfg(feature = "serde")]
mod ser;
#[cfg(feature = "serde")]
mod value;

#[cfg(feature = "serde")]
pub use de::{from_reader, from_slice, from_str};
#[cfg(feature = "serde")]
pub use ser::{to_string, to_writer};
