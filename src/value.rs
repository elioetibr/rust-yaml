//! YAML value representation

use crate::scanner::QuoteStyle;
use indexmap::IndexMap;
use std::fmt;
use std::hash::{Hash, Hasher};

/// Comments associated with a YAML value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comments {
    /// Comments that appear before this value
    pub leading: Vec<String>,
    /// Comment that appears on the same line as this value
    pub trailing: Option<String>,
    /// Comments that appear inside collections (between items)
    pub inner: Vec<String>,
}

impl Comments {
    /// Create empty comments
    pub const fn new() -> Self {
        Self {
            leading: Vec::new(),
            trailing: None,
            inner: Vec::new(),
        }
    }

    /// Check if there are any comments
    pub const fn is_empty(&self) -> bool {
        self.leading.is_empty() && self.trailing.is_none() && self.inner.is_empty()
    }

    /// Add a leading comment
    pub fn add_leading<S: Into<String>>(&mut self, comment: S) {
        self.leading.push(comment.into());
    }

    /// Set the trailing comment
    pub fn set_trailing<S: Into<String>>(&mut self, comment: S) {
        self.trailing = Some(comment.into());
    }

    /// Add an inner comment
    pub fn add_inner<S: Into<String>>(&mut self, comment: S) {
        self.inner.push(comment.into());
    }
}

impl Default for Comments {
    fn default() -> Self {
        Self::new()
    }
}

/// Indentation style used in YAML documents
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndentStyle {
    /// Spaces with specified width (2, 4, 8, etc.)
    Spaces(usize),
    /// Tab characters
    Tabs,
}

impl Default for IndentStyle {
    fn default() -> Self {
        Self::Spaces(2)
    }
}

/// Style information for YAML values to preserve formatting during round-trips
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    /// Quote style for string values
    pub quote_style: Option<QuoteStyle>,
    /// Indentation style for the document
    pub indent_style: Option<IndentStyle>,
}

impl Style {
    /// Create empty style information
    pub const fn new() -> Self {
        Self {
            quote_style: None,
            indent_style: None,
        }
    }

    /// Create style with quote style
    pub const fn with_quote_style(quote_style: QuoteStyle) -> Self {
        Self {
            quote_style: Some(quote_style),
            indent_style: None,
        }
    }

    /// Create style with indentation style
    pub const fn with_indent_style(indent_style: IndentStyle) -> Self {
        Self {
            quote_style: None,
            indent_style: Some(indent_style),
        }
    }

    /// Create style with both quote and indent styles
    pub const fn with_styles(quote_style: QuoteStyle, indent_style: IndentStyle) -> Self {
        Self {
            quote_style: Some(quote_style),
            indent_style: Some(indent_style),
        }
    }

    /// Check if there is any style information
    pub const fn is_empty(&self) -> bool {
        self.quote_style.is_none() && self.indent_style.is_none()
    }
}

impl Default for Style {
    fn default() -> Self {
        Self::new()
    }
}

/// A YAML value with optional comments and style for round-trip preservation
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentedValue {
    /// The actual YAML value
    pub value: Value,
    /// Comments associated with this value
    pub comments: Comments,
    /// Style information for formatting preservation
    pub style: Style,
}

impl CommentedValue {
    /// Create a new commented value
    pub const fn new(value: Value) -> Self {
        Self {
            value,
            comments: Comments::new(),
            style: Style::new(),
        }
    }

    /// Create a commented value with leading comments
    pub fn with_leading_comments(value: Value, comments: Vec<String>) -> Self {
        let mut commented = Self::new(value);
        commented.comments.leading = comments;
        commented
    }

    /// Create a commented value with a trailing comment
    pub fn with_trailing_comment(value: Value, comment: String) -> Self {
        let mut commented = Self::new(value);
        commented.comments.trailing = Some(comment);
        commented
    }

    /// Add a leading comment
    pub fn add_leading_comment<S: Into<String>>(&mut self, comment: S) {
        self.comments.add_leading(comment);
    }

    /// Set a trailing comment
    pub fn set_trailing_comment<S: Into<String>>(&mut self, comment: S) {
        self.comments.set_trailing(comment);
    }

    /// Check if this value has any comments
    pub const fn has_comments(&self) -> bool {
        !self.comments.is_empty()
    }

    /// Create a commented value with quote style
    pub const fn with_quote_style(value: Value, quote_style: QuoteStyle) -> Self {
        Self {
            value,
            comments: Comments::new(),
            style: Style::with_quote_style(quote_style),
        }
    }

    /// Set the quote style
    pub const fn set_quote_style(&mut self, quote_style: QuoteStyle) {
        self.style.quote_style = Some(quote_style);
    }

    /// Get the quote style
    pub const fn quote_style(&self) -> Option<&QuoteStyle> {
        self.style.quote_style.as_ref()
    }

    /// Check if this value has style information
    pub const fn has_style(&self) -> bool {
        !self.style.is_empty()
    }

    /// Set the indentation style
    pub const fn set_indent_style(&mut self, indent_style: IndentStyle) {
        self.style.indent_style = Some(indent_style);
    }

    /// Get the indentation style
    pub const fn indent_style(&self) -> Option<&IndentStyle> {
        self.style.indent_style.as_ref()
    }

    /// Create a commented value with indentation style
    pub const fn with_indent_style(value: Value, indent_style: IndentStyle) -> Self {
        Self {
            value,
            comments: Comments::new(),
            style: Style::with_indent_style(indent_style),
        }
    }
}

impl From<Value> for CommentedValue {
    fn from(value: Value) -> Self {
        Self::new(value)
    }
}

/// A YAML value with all possible types
#[derive(Debug, Clone)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Floating point value
    Float(f64),
    /// String value
    String(String),
    /// Sequence (array/list) value
    Sequence(Vec<Value>),
    /// Mapping (dictionary/object) value
    Mapping(IndexMap<Value, Value>),
}

impl Value {
    /// Create a null value
    pub const fn null() -> Self {
        Self::Null
    }

    /// Create a boolean value
    pub const fn bool(b: bool) -> Self {
        Self::Bool(b)
    }

    /// Create an integer value
    pub const fn int(i: i64) -> Self {
        Self::Int(i)
    }

    /// Create a float value
    pub const fn float(f: f64) -> Self {
        Self::Float(f)
    }

    /// Create a string value
    pub fn string(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }

    /// Create an empty sequence
    pub const fn sequence() -> Self {
        Self::Sequence(Vec::new())
    }

    /// Create a sequence with values
    pub const fn sequence_with(values: Vec<Self>) -> Self {
        Self::Sequence(values)
    }

    /// Create an empty mapping
    pub fn mapping() -> Self {
        Self::Mapping(IndexMap::new())
    }

    /// Create a mapping with key-value pairs
    pub fn mapping_with(pairs: Vec<(Self, Self)>) -> Self {
        let mut map = IndexMap::new();
        for (key, value) in pairs {
            map.insert(key, value);
        }
        Self::Mapping(map)
    }

    /// Get the type name of this value
    pub const fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Int(_) => "int",
            Self::Float(_) => "float",
            Self::String(_) => "string",
            Self::Sequence(_) => "sequence",
            Self::Mapping(_) => "mapping",
        }
    }

    /// Check if this value is null
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Check if this value is a boolean
    pub const fn is_bool(&self) -> bool {
        matches!(self, Self::Bool(_))
    }

    /// Check if this value is an integer
    pub const fn is_int(&self) -> bool {
        matches!(self, Self::Int(_))
    }

    /// Check if this value is a float
    pub const fn is_float(&self) -> bool {
        matches!(self, Self::Float(_))
    }

    /// Check if this value is a string
    pub const fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    /// Check if this value is a sequence
    pub const fn is_sequence(&self) -> bool {
        matches!(self, Self::Sequence(_))
    }

    /// Check if this value is a mapping
    pub const fn is_mapping(&self) -> bool {
        matches!(self, Self::Mapping(_))
    }

    /// Check if this value is a number (int or float)
    pub const fn is_number(&self) -> bool {
        matches!(self, Self::Int(_) | Self::Float(_))
    }

    /// Get the length of sequences and mappings, None for scalars
    pub fn len(&self) -> Option<usize> {
        match self {
            Self::Sequence(seq) => Some(seq.len()),
            Self::Mapping(map) => Some(map.len()),
            _ => None,
        }
    }

    /// Check if sequences, mappings, or strings are empty
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Sequence(seq) => seq.is_empty(),
            Self::Mapping(map) => map.is_empty(),
            Self::String(s) => s.is_empty(),
            _ => false,
        }
    }

    /// Get this value as a boolean, if possible
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Get this value as an integer, if possible
    pub const fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// Get this value as a float, if possible
    pub const fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Get this value as a string reference, if possible
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get this value as a sequence reference, if possible
    pub const fn as_sequence(&self) -> Option<&Vec<Self>> {
        match self {
            Self::Sequence(seq) => Some(seq),
            _ => None,
        }
    }

    /// Get this value as a mutable sequence reference, if possible
    pub const fn as_sequence_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Self::Sequence(seq) => Some(seq),
            _ => None,
        }
    }

    /// Get this value as a mapping reference, if possible
    pub const fn as_mapping(&self) -> Option<&IndexMap<Self, Self>> {
        match self {
            Self::Mapping(map) => Some(map),
            _ => None,
        }
    }

    /// Get this value as a mutable mapping reference, if possible
    pub const fn as_mapping_mut(&mut self) -> Option<&mut IndexMap<Self, Self>> {
        match self {
            Self::Mapping(map) => Some(map),
            _ => None,
        }
    }

    /// Index into a sequence or mapping
    pub fn get(&self, index: &Self) -> Option<&Self> {
        match (self, index) {
            (Self::Sequence(seq), Self::Int(i)) => {
                if *i >= 0 && (*i as usize) < seq.len() {
                    seq.get(*i as usize)
                } else {
                    None
                }
            }
            (Self::Mapping(map), key) => map.get(key),
            _ => None,
        }
    }

    /// Convenience method to get a value by string key
    pub fn get_str(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Mapping(map) => map.get(&Self::String(key.to_string())),
            _ => None,
        }
    }

    /// Get a value by numeric index (for sequences)
    pub fn get_index(&self, index: usize) -> Option<&Self> {
        match self {
            Self::Sequence(seq) => seq.get(index),
            _ => None,
        }
    }

    /// Mutably index into a sequence or mapping
    pub fn get_mut(&mut self, index: &Self) -> Option<&mut Self> {
        match (self, index) {
            (Self::Sequence(seq), Self::Int(i)) => {
                if *i >= 0 && (*i as usize) < seq.len() {
                    seq.get_mut(*i as usize)
                } else {
                    None
                }
            }
            (Self::Mapping(map), key) => map.get_mut(key),
            _ => None,
        }
    }
}

// Implement PartialEq manually to handle NaN in floats
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => {
                // Special handling for NaN - all NaN values are considered equal for consistency with Hash
                if a.is_nan() && b.is_nan() {
                    true
                } else {
                    a == b
                }
            }
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Sequence(a), Value::Sequence(b)) => a == b,
            (Value::Mapping(a), Value::Mapping(b)) => a == b,
            _ => false,
        }
    }
}

// Implement Eq - safe because we handle NaN consistently
impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Null => 0u8.hash(state),
            Self::Bool(b) => {
                1u8.hash(state);
                b.hash(state);
            }
            Self::Int(i) => {
                2u8.hash(state);
                i.hash(state);
            }
            Self::Float(f) => {
                3u8.hash(state);
                // Handle NaN and negative zero
                if f.is_nan() {
                    u64::MAX.hash(state);
                } else if *f == 0.0 {
                    0u64.hash(state);
                } else {
                    f.to_bits().hash(state);
                }
            }
            Self::String(s) => {
                4u8.hash(state);
                s.hash(state);
            }
            Self::Sequence(seq) => {
                5u8.hash(state);
                seq.hash(state);
            }
            Self::Mapping(map) => {
                6u8.hash(state);
                // Hash all key-value pairs
                for (k, v) in map.iter() {
                    k.hash(state);
                    v.hash(state);
                }
            }
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Int(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::String(s) => write!(f, "\"{}\"", s),
            Self::Sequence(seq) => {
                write!(f, "[")?;
                for (i, item) in seq.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Self::Mapping(map) => {
                write!(f, "{{")?;
                for (i, (key, value)) in map.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", key, value)?;
                }
                write!(f, "}}")
            }
        }
    }
}

// Conversions from primitive types
impl From<()> for Value {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Self::Int(i)
    }
}

impl From<i32> for Value {
    fn from(i: i32) -> Self {
        Self::Int(i64::from(i))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Self::Float(f64::from(f))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<Vec<Self>> for Value {
    fn from(seq: Vec<Self>) -> Self {
        Self::Sequence(seq)
    }
}

impl From<IndexMap<Self, Self>> for Value {
    fn from(map: IndexMap<Self, Self>) -> Self {
        Self::Mapping(map)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Null => serializer.serialize_none(),
            Self::Bool(b) => serializer.serialize_bool(*b),
            Self::Int(i) => serializer.serialize_i64(*i),
            Self::Float(f) => serializer.serialize_f64(*f),
            Self::String(s) => serializer.serialize_str(s),
            Self::Sequence(seq) => seq.serialize(serializer),
            Self::Mapping(map) => map.serialize(serializer),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_creation() {
        assert_eq!(Value::null(), Value::Null);
        assert_eq!(Value::bool(true), Value::Bool(true));
        assert_eq!(Value::int(42), Value::Int(42));
        assert_eq!(Value::float(3.14), Value::Float(3.14));
        assert_eq!(Value::string("hello"), Value::String("hello".to_string()));
    }

    #[test]
    fn test_value_type_checks() {
        let null_val = Value::null();
        let bool_val = Value::bool(true);
        let int_val = Value::int(42);
        let string_val = Value::string("test");

        assert!(null_val.is_null());
        assert!(bool_val.is_bool());
        assert!(int_val.is_int());
        assert!(string_val.is_string());

        assert!(!null_val.is_bool());
        assert!(!bool_val.is_int());
    }

    #[test]
    fn test_value_conversions() {
        let bool_val = Value::bool(true);
        let int_val = Value::int(42);
        let float_val = Value::float(3.14);
        let string_val = Value::string("hello");

        assert_eq!(bool_val.as_bool(), Some(true));
        assert_eq!(int_val.as_int(), Some(42));
        assert_eq!(float_val.as_float(), Some(3.14));
        assert_eq!(string_val.as_str(), Some("hello"));

        assert_eq!(bool_val.as_int(), None);
        assert_eq!(int_val.as_bool(), None);
    }

    #[test]
    fn test_sequence_operations() {
        let mut seq = Value::sequence();
        if let Value::Sequence(ref mut v) = seq {
            v.push(Value::int(1));
            v.push(Value::string("hello"));
        }

        assert!(seq.is_sequence());
        assert_eq!(seq.as_sequence().unwrap().len(), 2);

        let index = Value::int(0);
        assert_eq!(seq.get(&index), Some(&Value::int(1)));
    }

    #[test]
    fn test_mapping_operations() {
        let mut map = Value::mapping();
        if let Value::Mapping(ref mut m) = map {
            m.insert(Value::string("key"), Value::int(42));
            m.insert(Value::string("name"), Value::string("test"));
        }

        assert!(map.is_mapping());
        assert_eq!(map.as_mapping().unwrap().len(), 2);

        let key = Value::string("key");
        assert_eq!(map.get(&key), Some(&Value::int(42)));
    }

    #[test]
    fn test_value_display() {
        assert_eq!(format!("{}", Value::null()), "null");
        assert_eq!(format!("{}", Value::bool(true)), "true");
        assert_eq!(format!("{}", Value::int(42)), "42");
        assert_eq!(format!("{}", Value::string("hello")), "\"hello\"");
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::int(42), Value::int(42));
        assert_eq!(Value::float(3.14), Value::float(3.14));
        assert_ne!(Value::int(42), Value::float(42.0));

        // Test NaN handling - NaN values are considered equal for consistency with Hash
        let nan1 = Value::float(f64::NAN);
        let nan2 = Value::float(f64::NAN);
        assert_eq!(nan1, nan2); // NaN == NaN for consistency with Hash implementation
    }

    #[test]
    fn test_conversions_from_primitives() {
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(42i32), Value::Int(42));
        assert_eq!(Value::from(42i64), Value::Int(42));
        assert_eq!(Value::from(3.14f64), Value::Float(3.14));
        assert_eq!(Value::from("hello"), Value::String("hello".to_string()));
        assert_eq!(
            Value::from("hello".to_string()),
            Value::String("hello".to_string())
        );
    }
}
