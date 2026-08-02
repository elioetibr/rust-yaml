// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! YAML resolver for tag resolution and implicit typing

use crate::version::YamlVersion;
use crate::{Error, Position};
use std::collections::HashMap;

/// Build the error returned when the resolver detects the YAML 1.1
/// `tag:yaml.org,2002:value` indicator (`=`).
///
/// Centralized so all composers report the same message — matches the
/// `ConstructorError` raised by `ruamel.yaml` typ="safe" / typ="unsafe".
/// See YAML 1.1 §10.3.4 — <https://yaml.org/spec/1.1/#id903992>.
#[must_use]
pub fn value_tag_error(position: Position) -> Error {
    Error::construction(
        position,
        "the YAML 1.1 `=` indicator (tag:yaml.org,2002:value) has no \
         constructor in rust-yaml; drop the `%YAML 1.1` directive or \
         quote the value (`'='`) to keep it as a string",
    )
}

/// Result of resolving a plain (unquoted) scalar to a YAML type.
///
/// This is used by every composer variant to share implicit-resolution
/// logic. Each composer maps the variants to its own value type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlainScalarType {
    /// Null (`null`, `Null`, `NULL`, `~`).
    Null,
    /// Boolean — under YAML 1.2 only `true`/`false` (any case); under
    /// YAML 1.1 also `yes`/`no`/`on`/`off`.
    Bool(bool),
    /// 64-bit signed integer (decimal).
    Int(i64),
    /// 64-bit float.
    Float(f64),
    /// Falls through to a string — the caller keeps the original input.
    Str,
    /// YAML 1.1 `tag:yaml.org,2002:value` (the bare `=` indicator,
    /// §10.3.4 of the 1.1 spec). Dropped from the 1.2 Core Schema, so
    /// the resolver only emits this under a `%YAML 1.1` directive.
    /// Composers should reject it — there is no `Value` variant in the
    /// user-facing tree to construct it into. This mirrors
    /// `ruamel.yaml typ="safe"` / `typ="unsafe"`, both of which raise
    /// `ConstructorError`.
    Value,
}

/// Resolve a plain scalar to a [`PlainScalarType`] under the given
/// YAML version.
///
/// This is the single source of truth for implicit scalar typing.
/// Composers call it instead of duplicating the resolution sequence.
///
/// Empty plain scalars currently fall through to `Str` to preserve
/// existing rust-yaml behavior; the YAML 1.2 spec treats them as `Null`,
/// which is tracked as a separate Core Schema gap.
#[must_use]
pub fn resolve_plain_scalar(value: &str, version: YamlVersion) -> PlainScalarType {
    // §10.2 (Core Schema) and §10.3 failsafe table: an empty plain
    // scalar resolves to `tag:yaml.org,2002:null` — i.e. \`Null\`.
    if value.is_empty() {
        return PlainScalarType::Null;
    }

    // Decimal integers.
    if let Ok(i) = value.parse::<i64>() {
        return PlainScalarType::Int(i);
    }

    // Hex / octal / binary integers. Mirrors the `!!int` tag constructor
    // (`tag.rs::construct_int`) so implicit and explicit typing agree (#22).
    if let Some(i) = resolve_radix_int(value) {
        return PlainScalarType::Int(i);
    }

    // §10.2 float infinity / NaN: only the dotted spellings (`.inf`,
    // `.nan`, with an optional sign on `.inf`) are YAML 1.2 floats (#22).
    if let Some(f) = resolve_inf_nan(value) {
        return PlainScalarType::Float(f);
    }

    // Finite floats. The digit guard stops Rust's `f64` parser from
    // accepting the bare `inf` / `infinity` / `nan` words, which YAML 1.2
    // does not recognize — those fall through to a string.
    if value.bytes().any(|b| b.is_ascii_digit()) {
        if let Ok(f) = value.parse::<f64>() {
            return PlainScalarType::Float(f);
        }
    }

    // YAML 1.1 §10.3.4: bare `=` is the `tag:yaml.org,2002:value`
    // indicator. Case-sensitive (only literal `=`), and the full scalar
    // must be exactly `=` — `a=b` / `==` / etc. stay as plain strings.
    if version == YamlVersion::V1_1 && value == "=" {
        return PlainScalarType::Value;
    }

    let lower = value.to_lowercase();
    match lower.as_str() {
        "true" => PlainScalarType::Bool(true),
        "false" => PlainScalarType::Bool(false),
        "null" | "~" => PlainScalarType::Null,
        "yes" | "on" if version == YamlVersion::V1_1 => PlainScalarType::Bool(true),
        "no" | "off" if version == YamlVersion::V1_1 => PlainScalarType::Bool(false),
        _ => PlainScalarType::Str,
    }
}

/// Parse a hex (`0x`), octal (`0o`), or binary (`0b`) integer literal.
///
/// Accepts the upper- and lower-case prefix forms, mirroring the `!!int`
/// tag constructor. Returns `None` for any other input, or when the digits
/// are missing or invalid for the radix.
fn resolve_radix_int(value: &str) -> Option<i64> {
    /// `(lowercase prefix, uppercase prefix, radix)`, tried in order. No prefix
    /// is a prefix of another, so the order is presentational rather than
    /// significant.
    const PREFIXES: [(&str, &str, u32); 3] = [("0x", "0X", 16), ("0o", "0O", 8), ("0b", "0B", 2)];

    let (radix, digits) = PREFIXES.iter().find_map(|&(lower, upper, radix)| {
        value
            .strip_prefix(lower)
            .or_else(|| value.strip_prefix(upper))
            .map(|digits| (radix, digits))
    })?;

    if digits.is_empty() {
        return None;
    }
    i64::from_str_radix(digits, radix).ok()
}

/// Resolve the YAML 1.2 §10.2 float infinity / NaN spellings. The leading
/// dot is mandatory; `.inf` takes an optional sign, `.nan` does not.
fn resolve_inf_nan(value: &str) -> Option<f64> {
    match value {
        ".inf" | ".Inf" | ".INF" | "+.inf" | "+.Inf" | "+.INF" => Some(f64::INFINITY),
        "-.inf" | "-.Inf" | "-.INF" => Some(f64::NEG_INFINITY),
        ".nan" | ".NaN" | ".NAN" => Some(f64::NAN),
        _ => None,
    }
}

/// Trait for YAML resolvers that handle tag resolution
pub trait Resolver {
    /// Resolve a tag for implicit typing
    fn resolve_tag(&self, value: &str, implicit: bool) -> Option<String>;

    /// Add an implicit resolver pattern
    fn add_implicit_resolver(&mut self, tag: String, pattern: String);

    /// Reset the resolver state
    fn reset(&mut self);
}

/// Basic resolver with standard YAML 1.2 implicit typing
#[derive(Debug)]
pub struct BasicResolver {
    implicit_resolvers: HashMap<String, String>,
}

impl BasicResolver {
    /// Create a new resolver with standard YAML 1.2 resolvers
    pub fn new() -> Self {
        let mut resolver = Self {
            implicit_resolvers: HashMap::new(),
        };

        // Add standard YAML 1.2 implicit resolvers
        resolver.add_standard_resolvers();
        resolver
    }

    fn add_standard_resolvers(&mut self) {
        // Boolean values
        self.implicit_resolvers
            .insert("true".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("True".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("TRUE".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("false".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("False".to_string(), "tag:yaml.org,2002:bool".to_string());
        self.implicit_resolvers
            .insert("FALSE".to_string(), "tag:yaml.org,2002:bool".to_string());

        // Null values
        self.implicit_resolvers
            .insert("null".to_string(), "tag:yaml.org,2002:null".to_string());
        self.implicit_resolvers
            .insert("Null".to_string(), "tag:yaml.org,2002:null".to_string());
        self.implicit_resolvers
            .insert("NULL".to_string(), "tag:yaml.org,2002:null".to_string());
        self.implicit_resolvers
            .insert("~".to_string(), "tag:yaml.org,2002:null".to_string());
    }

    /// Check if a string represents an integer
    pub fn is_int(&self, value: &str) -> bool {
        value.parse::<i64>().is_ok()
    }

    /// Check if a string represents a float
    pub fn is_float(&self, value: &str) -> bool {
        value.parse::<f64>().is_ok()
    }
}

impl Default for BasicResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Resolver for BasicResolver {
    fn resolve_tag(&self, value: &str, implicit: bool) -> Option<String> {
        if !implicit {
            return None;
        }

        // Check explicit mappings first
        if let Some(tag) = self.implicit_resolvers.get(value) {
            return Some(tag.clone());
        }

        // Check numeric types
        if self.is_int(value) {
            return Some("tag:yaml.org,2002:int".to_string());
        }

        if self.is_float(value) {
            return Some("tag:yaml.org,2002:float".to_string());
        }

        // Default to string
        Some("tag:yaml.org,2002:str".to_string())
    }

    fn add_implicit_resolver(&mut self, tag: String, pattern: String) {
        self.implicit_resolvers.insert(pattern, tag);
    }

    fn reset(&mut self) {
        // Keep the standard resolvers, don't clear them
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_scalar_decimal_int() {
        assert_eq!(
            resolve_plain_scalar("42", YamlVersion::V1_2),
            PlainScalarType::Int(42)
        );
        assert_eq!(
            resolve_plain_scalar("-7", YamlVersion::V1_2),
            PlainScalarType::Int(-7)
        );
    }

    #[test]
    fn plain_scalar_float() {
        assert_eq!(
            resolve_plain_scalar("3.14", YamlVersion::V1_2),
            PlainScalarType::Float(3.14)
        );
    }

    #[test]
    fn plain_scalar_bool_1_2_only_true_false() {
        assert_eq!(
            resolve_plain_scalar("true", YamlVersion::V1_2),
            PlainScalarType::Bool(true)
        );
        assert_eq!(
            resolve_plain_scalar("TRUE", YamlVersion::V1_2),
            PlainScalarType::Bool(true)
        );
        assert_eq!(
            resolve_plain_scalar("False", YamlVersion::V1_2),
            PlainScalarType::Bool(false)
        );
    }

    #[test]
    fn plain_scalar_bool_1_2_rejects_yes_no_on_off() {
        for s in ["yes", "no", "on", "off", "Yes", "NO", "On", "OFF"] {
            assert_eq!(
                resolve_plain_scalar(s, YamlVersion::V1_2),
                PlainScalarType::Str,
                "{s:?} should fall through to Str under 1.2"
            );
        }
    }

    #[test]
    fn plain_scalar_bool_1_1_accepts_yes_no_on_off() {
        assert_eq!(
            resolve_plain_scalar("yes", YamlVersion::V1_1),
            PlainScalarType::Bool(true)
        );
        assert_eq!(
            resolve_plain_scalar("no", YamlVersion::V1_1),
            PlainScalarType::Bool(false)
        );
        assert_eq!(
            resolve_plain_scalar("on", YamlVersion::V1_1),
            PlainScalarType::Bool(true)
        );
        assert_eq!(
            resolve_plain_scalar("off", YamlVersion::V1_1),
            PlainScalarType::Bool(false)
        );
    }

    #[test]
    fn plain_scalar_null_any_version() {
        for v in [YamlVersion::V1_1, YamlVersion::V1_2] {
            assert_eq!(resolve_plain_scalar("null", v), PlainScalarType::Null);
            assert_eq!(resolve_plain_scalar("Null", v), PlainScalarType::Null);
            assert_eq!(resolve_plain_scalar("NULL", v), PlainScalarType::Null);
            assert_eq!(resolve_plain_scalar("~", v), PlainScalarType::Null);
        }
    }

    #[test]
    fn plain_scalar_string_fallback() {
        assert_eq!(
            resolve_plain_scalar("hello", YamlVersion::V1_2),
            PlainScalarType::Str
        );
        // YAML 1.2 dropped the `!!value` tag — `=` is a plain string.
        assert_eq!(
            resolve_plain_scalar("=", YamlVersion::V1_2),
            PlainScalarType::Str
        );
    }

    /// YAML 1.1 §10.3.4 — `=` is the indicator for the
    /// `tag:yaml.org,2002:value` (Value) tag. The resolver surfaces
    /// it as a distinct variant so composers can refuse it the way
    /// `ruamel.yaml` typ="safe" / typ="unsafe" do — see
    /// <https://yaml.org/spec/1.1/#id903992>.
    #[test]
    fn plain_scalar_value_tag_1_1() {
        assert_eq!(
            resolve_plain_scalar("=", YamlVersion::V1_1),
            PlainScalarType::Value
        );
    }

    /// The `=` indicator must be the entire scalar — strings that merely
    /// contain `=` (`a=b`, `==`, `= `, ` =`) stay as plain strings even
    /// under 1.1.
    #[test]
    fn plain_scalar_value_tag_1_1_only_bare_equals() {
        for s in ["==", "a=b", "= ", " =", " = "] {
            assert_eq!(
                resolve_plain_scalar(s, YamlVersion::V1_1),
                PlainScalarType::Str,
                "{s:?} should fall through to Str — only bare `=` is the value tag"
            );
        }
    }

    #[test]
    fn plain_scalar_hex_octal_binary_int() {
        // YAML 1.2 Core `0x`/`0o` plus the `0b` binary form (#22).
        assert_eq!(
            resolve_plain_scalar("0xFF", YamlVersion::V1_2),
            PlainScalarType::Int(255)
        );
        assert_eq!(
            resolve_plain_scalar("0xff", YamlVersion::V1_2),
            PlainScalarType::Int(255)
        );
        assert_eq!(
            resolve_plain_scalar("0o17", YamlVersion::V1_2),
            PlainScalarType::Int(15)
        );
        assert_eq!(
            resolve_plain_scalar("0b101", YamlVersion::V1_2),
            PlainScalarType::Int(5)
        );
    }

    #[test]
    fn plain_scalar_malformed_radix_int_is_string() {
        // A recognized prefix with no / invalid digits is not an integer.
        for s in ["0x", "0o8", "0b102", "0xZZ"] {
            assert_eq!(
                resolve_plain_scalar(s, YamlVersion::V1_2),
                PlainScalarType::Str,
                "{s:?} should fall through to Str"
            );
        }
    }

    #[test]
    fn plain_scalar_dotted_inf_nan_are_floats() {
        // §10.2: only the dotted spellings are YAML 1.2 floats (#22).
        assert_eq!(
            resolve_plain_scalar(".inf", YamlVersion::V1_2),
            PlainScalarType::Float(f64::INFINITY)
        );
        assert_eq!(
            resolve_plain_scalar("-.inf", YamlVersion::V1_2),
            PlainScalarType::Float(f64::NEG_INFINITY)
        );
        assert!(matches!(
            resolve_plain_scalar(".nan", YamlVersion::V1_2),
            PlainScalarType::Float(f) if f.is_nan()
        ));
    }

    #[test]
    fn plain_scalar_bare_inf_nan_are_strings() {
        // Bare `inf`/`nan` (no leading dot) are NOT YAML 1.2 floats — they
        // must stay strings, even though Rust's f64 parser would accept
        // them (#22).
        for s in ["inf", "nan", "Inf", "NaN", "infinity"] {
            assert_eq!(
                resolve_plain_scalar(s, YamlVersion::V1_2),
                PlainScalarType::Str,
                "{s:?} should fall through to Str"
            );
        }
    }

    #[test]
    fn test_resolver_creation() {
        let resolver = BasicResolver::new();
        assert!(!resolver.implicit_resolvers.is_empty());
    }

    #[test]
    fn test_boolean_resolution() {
        let resolver = BasicResolver::new();

        assert_eq!(
            resolver.resolve_tag("true", true),
            Some("tag:yaml.org,2002:bool".to_string())
        );
        assert_eq!(
            resolver.resolve_tag("false", true),
            Some("tag:yaml.org,2002:bool".to_string())
        );
    }

    #[test]
    fn test_null_resolution() {
        let resolver = BasicResolver::new();

        assert_eq!(
            resolver.resolve_tag("null", true),
            Some("tag:yaml.org,2002:null".to_string())
        );
        assert_eq!(
            resolver.resolve_tag("~", true),
            Some("tag:yaml.org,2002:null".to_string())
        );
    }

    #[test]
    fn test_numeric_resolution() {
        let resolver = BasicResolver::new();

        assert_eq!(
            resolver.resolve_tag("42", true),
            Some("tag:yaml.org,2002:int".to_string())
        );
        assert_eq!(
            resolver.resolve_tag("3.14", true),
            Some("tag:yaml.org,2002:float".to_string())
        );
    }

    #[test]
    fn test_string_resolution() {
        let resolver = BasicResolver::new();

        assert_eq!(
            resolver.resolve_tag("hello", true),
            Some("tag:yaml.org,2002:str".to_string())
        );
    }

    #[test]
    fn test_explicit_tag_resolution() {
        let resolver = BasicResolver::new();

        // When not implicit, should return None
        assert_eq!(resolver.resolve_tag("true", false), None);
    }

    #[test]
    fn test_custom_resolver() {
        let mut resolver = BasicResolver::new();

        resolver.add_implicit_resolver(
            "tag:example.com,2002:custom".to_string(),
            "CUSTOM".to_string(),
        );

        assert_eq!(
            resolver.resolve_tag("CUSTOM", true),
            Some("tag:example.com,2002:custom".to_string())
        );
    }
}
