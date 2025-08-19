//! Zero-copy YAML value representation using borrowed data
//!
//! This module provides a borrowed version of the Value enum that minimizes
//! allocations by using Cow (Clone-on-Write) for strings and borrowed slices
//! where possible.

use indexmap::IndexMap;
use std::borrow::Cow;
use std::fmt;
use std::hash::{Hash, Hasher};

/// A zero-copy YAML value that borrows data where possible
#[derive(Debug, Clone, PartialEq)]
pub enum BorrowedValue<'a> {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Floating point value
    Float(f64),
    /// String value (borrowed or owned)
    String(Cow<'a, str>),
    /// Sequence (array/list) value
    Sequence(Vec<BorrowedValue<'a>>),
    /// Mapping (dictionary/object) value
    Mapping(IndexMap<BorrowedValue<'a>, BorrowedValue<'a>>),
}

impl<'a> BorrowedValue<'a> {
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

    /// Create a borrowed string value
    pub const fn borrowed_string(s: &'a str) -> Self {
        Self::String(Cow::Borrowed(s))
    }

    /// Create an owned string value
    pub fn owned_string(s: String) -> Self {
        Self::String(Cow::Owned(s))
    }

    /// Create a string value from a Cow
    pub const fn string(s: Cow<'a, str>) -> Self {
        Self::String(s)
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

    /// Try to get this value as a boolean
    pub const fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Try to get this value as an integer
    pub const fn as_int(&self) -> Option<i64> {
        if let Self::Int(i) = self {
            Some(*i)
        } else {
            None
        }
    }

    /// Try to get this value as a float
    pub const fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Int(i) => Some(*i as f64),
            _ => None,
        }
    }

    /// Try to get this value as a string slice
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(s) = self {
            Some(s.as_ref())
        } else {
            None
        }
    }

    /// Try to get this value as a sequence
    pub const fn as_sequence(&self) -> Option<&Vec<BorrowedValue<'a>>> {
        if let Self::Sequence(seq) = self {
            Some(seq)
        } else {
            None
        }
    }

    /// Try to get this value as a mutable sequence
    pub fn as_sequence_mut(&mut self) -> Option<&mut Vec<BorrowedValue<'a>>> {
        if let Self::Sequence(seq) = self {
            Some(seq)
        } else {
            None
        }
    }

    /// Try to get this value as a mapping
    pub const fn as_mapping(&self) -> Option<&IndexMap<BorrowedValue<'a>, BorrowedValue<'a>>> {
        if let Self::Mapping(map) = self {
            Some(map)
        } else {
            None
        }
    }

    /// Try to get this value as a mutable mapping
    pub fn as_mapping_mut(
        &mut self,
    ) -> Option<&mut IndexMap<BorrowedValue<'a>, BorrowedValue<'a>>> {
        if let Self::Mapping(map) = self {
            Some(map)
        } else {
            None
        }
    }

    /// Convert to an owned Value (for when lifetime constraints require it)
    pub fn into_owned(self) -> BorrowedValue<'static> {
        match self {
            Self::Null => BorrowedValue::Null,
            Self::Bool(b) => BorrowedValue::Bool(b),
            Self::Int(i) => BorrowedValue::Int(i),
            Self::Float(f) => BorrowedValue::Float(f),
            Self::String(s) => BorrowedValue::String(Cow::Owned(s.into_owned())),
            Self::Sequence(seq) => {
                BorrowedValue::Sequence(seq.into_iter().map(|v| v.into_owned()).collect())
            }
            Self::Mapping(map) => BorrowedValue::Mapping(
                map.into_iter()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect(),
            ),
        }
    }

    /// Clone only if necessary (for Cow optimization)
    pub fn clone_if_needed(&self) -> Self {
        match self {
            Self::String(Cow::Borrowed(s)) => Self::String(Cow::Borrowed(s)),
            _ => self.clone(),
        }
    }
}

impl<'a> Default for BorrowedValue<'a> {
    fn default() -> Self {
        Self::Null
    }
}

impl<'a> fmt::Display for BorrowedValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{}", b),
            Self::Int(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::String(s) => write!(f, "{}", s),
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

impl<'a> Hash for BorrowedValue<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Null => 0.hash(state),
            Self::Bool(b) => {
                1.hash(state);
                b.hash(state);
            }
            Self::Int(i) => {
                2.hash(state);
                i.hash(state);
            }
            Self::Float(f) => {
                3.hash(state);
                f.to_bits().hash(state);
            }
            Self::String(s) => {
                4.hash(state);
                s.hash(state);
            }
            Self::Sequence(seq) => {
                5.hash(state);
                seq.hash(state);
            }
            Self::Mapping(_) => {
                6.hash(state);
                // Note: IndexMap iteration order is deterministic
                // but we can't hash the map directly
            }
        }
    }
}

impl<'a> Eq for BorrowedValue<'a> {}

// Conversion from owned Value to BorrowedValue
impl<'a> From<crate::Value> for BorrowedValue<'a> {
    fn from(value: crate::Value) -> Self {
        match value {
            crate::Value::Null => Self::Null,
            crate::Value::Bool(b) => Self::Bool(b),
            crate::Value::Int(i) => Self::Int(i),
            crate::Value::Float(f) => Self::Float(f),
            crate::Value::String(s) => Self::String(Cow::Owned(s)),
            crate::Value::Sequence(seq) => {
                Self::Sequence(seq.into_iter().map(Into::into).collect())
            }
            crate::Value::Mapping(map) => {
                Self::Mapping(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
        }
    }
}

// Conversion from BorrowedValue to owned Value
impl From<BorrowedValue<'_>> for crate::Value {
    fn from(value: BorrowedValue<'_>) -> Self {
        match value {
            BorrowedValue::Null => Self::Null,
            BorrowedValue::Bool(b) => Self::Bool(b),
            BorrowedValue::Int(i) => Self::Int(i),
            BorrowedValue::Float(f) => Self::Float(f),
            BorrowedValue::String(s) => Self::String(s.into_owned()),
            BorrowedValue::Sequence(seq) => {
                Self::Sequence(seq.into_iter().map(Into::into).collect())
            }
            BorrowedValue::Mapping(map) => {
                Self::Mapping(map.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_borrowed_string() {
        let s = "hello world";
        let value = BorrowedValue::borrowed_string(s);
        assert_eq!(value.as_str(), Some("hello world"));

        // Verify it's actually borrowed
        if let BorrowedValue::String(cow) = &value {
            assert!(matches!(cow, Cow::Borrowed(_)));
        }
    }

    #[test]
    fn test_owned_string() {
        let value = BorrowedValue::owned_string("hello".to_string());
        assert_eq!(value.as_str(), Some("hello"));

        // Verify it's owned
        if let BorrowedValue::String(cow) = &value {
            assert!(matches!(cow, Cow::Owned(_)));
        }
    }

    #[test]
    fn test_into_owned() {
        let s = "test";
        let borrowed = BorrowedValue::borrowed_string(s);
        let owned: BorrowedValue<'static> = borrowed.into_owned();

        // Verify the owned version is actually owned
        if let BorrowedValue::String(cow) = &owned {
            assert!(matches!(cow, Cow::Owned(_)));
        }
    }

    #[test]
    fn test_zero_copy_mapping() {
        let key = "key";
        let value = "value";

        let map = BorrowedValue::mapping_with(vec![(
            BorrowedValue::borrowed_string(key),
            BorrowedValue::borrowed_string(value),
        )]);

        assert!(map.is_mapping());
        if let BorrowedValue::Mapping(m) = &map {
            assert_eq!(m.len(), 1);
        }
    }
}
