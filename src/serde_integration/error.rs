// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bridge between rust-yaml's `Error` type and serde's error traits.

use crate::{Error, Position};
use std::fmt::Display;

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::construction(Position::new(), msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::construction(Position::new(), msg.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::Error;
    use serde::{de::Error as DeError, ser::Error as SerError};

    #[test]
    fn ser_error_custom_uses_construction_variant() {
        let e: Error = SerError::custom("boom");
        assert!(e.to_string().contains("boom"));
    }

    #[test]
    fn de_error_custom_uses_construction_variant() {
        let e: Error = DeError::custom("boom");
        assert!(e.to_string().contains("boom"));
    }
}
