// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! YAML version handling for `%YAML` directive support.
//!
//! The YAML 1.2.2 spec is the default. A `%YAML 1.1` directive enables
//! YAML 1.1 implicit-resolution rules (`yes`/`no`/`on`/`off` as booleans).

/// YAML spec version, derived from a `%YAML major.minor` directive or
/// defaulted when no directive is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum YamlVersion {
    /// YAML 1.1 — accepts legacy boolean forms.
    V1_1,
    /// YAML 1.2 (default per current spec).
    #[default]
    V1_2,
}

impl YamlVersion {
    /// Convert a parsed `(major, minor)` directive pair into a `YamlVersion`.
    ///
    /// `1.1` selects [`Self::V1_1`]. Any other `1.x` selects [`Self::V1_2`]
    /// (the 1.2.2 spec states that 1.x documents must be processed by the
    /// most-recent 1.x parser).
    #[must_use]
    pub const fn from_directive(major: u8, minor: u8) -> Self {
        match (major, minor) {
            (1, 1) => Self::V1_1,
            _ => Self::V1_2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_1_2() {
        assert_eq!(YamlVersion::default(), YamlVersion::V1_2);
    }

    #[test]
    fn from_directive_recognizes_1_1() {
        assert_eq!(YamlVersion::from_directive(1, 1), YamlVersion::V1_1);
    }

    #[test]
    fn from_directive_defaults_to_1_2_for_other_1x() {
        assert_eq!(YamlVersion::from_directive(1, 2), YamlVersion::V1_2);
        assert_eq!(YamlVersion::from_directive(1, 3), YamlVersion::V1_2);
        assert_eq!(YamlVersion::from_directive(1, 0), YamlVersion::V1_2);
    }
}
