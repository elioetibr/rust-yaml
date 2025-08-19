//! Optimized YAML value representation with reduced allocations
//!
//! This module provides an optimized Value type that reduces cloning
//! and allocations through careful use of reference counting and
//! copy-on-write semantics.

use indexmap::IndexMap;
use std::fmt;
use std::rc::Rc;

/// An optimized YAML value that minimizes allocations
#[derive(Debug, Clone, PartialEq)]
pub enum OptimizedValue {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// String value (using Rc for cheap cloning)
    String(Rc<String>),
    /// Sequence value (using Rc for cheap cloning)
    Sequence(Rc<Vec<OptimizedValue>>),
    /// Mapping value (using Rc for cheap cloning)
    Mapping(Rc<IndexMap<OptimizedValue, OptimizedValue>>),
}

impl OptimizedValue {
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
        Self::String(Rc::new(s.into()))
    }

    /// Create an empty sequence
    pub fn sequence() -> Self {
        Self::Sequence(Rc::new(Vec::new()))
    }

    /// Create a sequence with values
    pub fn sequence_with(values: Vec<Self>) -> Self {
        Self::Sequence(Rc::new(values))
    }

    /// Create an empty mapping
    pub fn mapping() -> Self {
        Self::Mapping(Rc::new(IndexMap::new()))
    }

    /// Create a mapping with key-value pairs
    pub fn mapping_with(pairs: Vec<(Self, Self)>) -> Self {
        let mut map = IndexMap::new();
        for (key, value) in pairs {
            map.insert(key, value);
        }
        Self::Mapping(Rc::new(map))
    }

    /// Get a reference to the string if this is a string value
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(s) = self {
            Some(s.as_str())
        } else {
            None
        }
    }

    /// Get a reference to the sequence if this is a sequence value
    pub fn as_sequence(&self) -> Option<&[OptimizedValue]> {
        if let Self::Sequence(seq) = self {
            Some(seq.as_slice())
        } else {
            None
        }
    }

    /// Get a reference to the mapping if this is a mapping value
    pub fn as_mapping(&self) -> Option<&IndexMap<OptimizedValue, OptimizedValue>> {
        if let Self::Mapping(map) = self {
            Some(map.as_ref())
        } else {
            None
        }
    }

    /// Convert from regular Value
    pub fn from_value(value: crate::Value) -> Self {
        match value {
            crate::Value::Null => Self::Null,
            crate::Value::Bool(b) => Self::Bool(b),
            crate::Value::Int(i) => Self::Int(i),
            crate::Value::Float(f) => Self::Float(f),
            crate::Value::String(s) => Self::String(Rc::new(s)),
            crate::Value::Sequence(seq) => {
                Self::Sequence(Rc::new(seq.into_iter().map(Self::from_value).collect()))
            }
            crate::Value::Mapping(map) => Self::Mapping(Rc::new(
                map.into_iter()
                    .map(|(k, v)| (Self::from_value(k), Self::from_value(v)))
                    .collect(),
            )),
        }
    }

    /// Convert to regular Value
    pub fn to_value(&self) -> crate::Value {
        match self {
            Self::Null => crate::Value::Null,
            Self::Bool(b) => crate::Value::Bool(*b),
            Self::Int(i) => crate::Value::Int(*i),
            Self::Float(f) => crate::Value::Float(*f),
            Self::String(s) => crate::Value::String((**s).clone()),
            Self::Sequence(seq) => {
                crate::Value::Sequence(seq.iter().map(|v| v.to_value()).collect())
            }
            Self::Mapping(map) => crate::Value::Mapping(
                map.iter()
                    .map(|(k, v)| (k.to_value(), v.to_value()))
                    .collect(),
            ),
        }
    }
}

impl fmt::Display for OptimizedValue {
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

impl std::hash::Hash for OptimizedValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
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
                seq.len().hash(state);
                for item in seq.iter() {
                    item.hash(state);
                }
            }
            Self::Mapping(_) => {
                6.hash(state);
                // Mapping order is deterministic but we can't hash directly
            }
        }
    }
}

impl Eq for OptimizedValue {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rc_string_cloning() {
        let value1 = OptimizedValue::string("hello world");
        let value2 = value1.clone();

        // Both should point to the same Rc
        if let (OptimizedValue::String(s1), OptimizedValue::String(s2)) = (&value1, &value2) {
            assert!(Rc::ptr_eq(s1, s2));
        }
    }

    #[test]
    fn test_rc_sequence_cloning() {
        let value1 =
            OptimizedValue::sequence_with(vec![OptimizedValue::int(1), OptimizedValue::int(2)]);
        let value2 = value1.clone();

        // Both should point to the same Rc
        if let (OptimizedValue::Sequence(s1), OptimizedValue::Sequence(s2)) = (&value1, &value2) {
            assert!(Rc::ptr_eq(s1, s2));
        }
    }

    #[test]
    fn test_conversion_roundtrip() {
        let original = crate::Value::Mapping(indexmap::indexmap! {
            crate::Value::String("key".to_string()) => crate::Value::Int(42),
        });

        let optimized = OptimizedValue::from_value(original.clone());
        let converted_back = optimized.to_value();

        assert_eq!(original, converted_back);
    }
}
