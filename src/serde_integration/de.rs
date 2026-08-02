// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! serde::Deserializer that drives serde from a [`Value`].
//!
//! `from_str` / `from_slice` / `from_reader` parse YAML to `Value` via the
//! existing `load_str` pipeline, then walk the tree with this deserializer.

use crate::{Error, Value, Yaml};
use serde::de::{
    self, DeserializeOwned, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor,
};
use std::io::Read;

/// Borrowed deserializer over an existing `Value`.
pub struct ValueDeserializer<'a> {
    value: &'a Value,
}

impl<'a> ValueDeserializer<'a> {
    /// Wrap a `Value` reference for deserialization.
    #[must_use]
    pub fn new(value: &'a Value) -> Self {
        Self { value }
    }
}

impl<'de, 'a> Deserializer<'de> for ValueDeserializer<'a> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::Int(i) => visitor.visit_i64(*i),
            Value::Float(f) => visitor.visit_f64(*f),
            Value::String(s) => visitor.visit_str(s),
            Value::Sequence(seq) => visitor.visit_seq(SeqAccessImpl { iter: seq.iter() }),
            Value::Mapping(map) => visitor.visit_map(MapAccessImpl {
                iter: map.iter(),
                next_value: None,
            }),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple tuple_struct
        map struct identifier ignored_any
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.value {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.value {
            // Unit variant: a bare string naming the variant.
            Value::String(s) => {
                let de = de::value::StrDeserializer::<Error>::new(s.as_str());
                visitor.visit_enum(de)
            }

            // Tuple / struct / newtype variant: a single-entry mapping whose key
            // is the variant name and whose value carries the payload.
            Value::Mapping(map) if map.len() == 1 => {
                let (k, v) = if let Some(entry) = map.iter().next() {
                    entry
                } else {
                    return Err(<Error as de::Error>::custom(
                        "internal: len==1 but no entry",
                    ));
                };
                let name = match k {
                    Value::String(s) => s.as_str(),
                    _ => {
                        return Err(<Error as de::Error>::custom(
                            "enum variant key must be a string",
                        ));
                    }
                };
                visitor.visit_enum(EnumAccessImpl {
                    variant: name,
                    value: v,
                })
            }

            other => Err(<Error as de::Error>::custom(format!(
                "expected enum (string or single-entry mapping), got {other:?}"
            ))),
        }
    }
}

struct SeqAccessImpl<'a> {
    iter: std::slice::Iter<'a, Value>,
}

impl<'de, 'a> SeqAccess<'de> for SeqAccessImpl<'a> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        match self.iter.next() {
            Some(v) => seed.deserialize(ValueDeserializer::new(v)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct MapAccessImpl<'a> {
    iter: indexmap::map::Iter<'a, Value, Value>,
    next_value: Option<&'a Value>,
}

impl<'de, 'a> MapAccess<'de> for MapAccessImpl<'a> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match self.iter.next() {
            Some((k, v)) => {
                self.next_value = Some(v);
                seed.deserialize(ValueDeserializer::new(k)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        let v = self
            .next_value
            .take()
            .ok_or_else(|| <Error as de::Error>::custom("next_value before next_key"))?;
        seed.deserialize(ValueDeserializer::new(v))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct EnumAccessImpl<'a> {
    variant: &'a str,
    value: &'a Value,
}

impl<'de, 'a> serde::de::EnumAccess<'de> for EnumAccessImpl<'a> {
    type Error = Error;
    type Variant = VariantAccessImpl<'a>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Error> {
        let de = de::value::StrDeserializer::<Error>::new(self.variant);
        let name: V::Value = seed.deserialize(de)?;
        Ok((name, VariantAccessImpl { value: self.value }))
    }
}

struct VariantAccessImpl<'a> {
    value: &'a Value,
}

impl<'de, 'a> serde::de::VariantAccess<'de> for VariantAccessImpl<'a> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.value {
            Value::Null => Ok(()),
            _ => Err(<Error as de::Error>::custom(
                "unit variant must have Null payload",
            )),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(ValueDeserializer::new(self.value))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Error> {
        ValueDeserializer::new(self.value).deserialize_seq(visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        ValueDeserializer::new(self.value).deserialize_map(visitor)
    }
}

/// Parse YAML from a string into `T`.
///
/// # Errors
///
/// Returns an error if the YAML fails to parse or if `T`'s `Deserialize`
/// impl rejects the resulting structure.
pub fn from_str<T: DeserializeOwned>(s: &str) -> Result<T, Error> {
    let value = Yaml::new().load_str(s)?;
    T::deserialize(ValueDeserializer::new(&value))
}

/// Parse YAML from a byte slice into `T`.
///
/// # Errors
///
/// Returns an error if the bytes are not valid UTF-8, if the YAML fails to
/// parse, or if `T`'s `Deserialize` impl rejects the resulting structure.
pub fn from_slice<T: DeserializeOwned>(b: &[u8]) -> Result<T, Error> {
    let s = std::str::from_utf8(b).map_err(Error::from)?;
    from_str(s)
}

/// Parse YAML from a reader into `T`.
///
/// # Errors
///
/// Returns an error if reading fails, if the YAML fails to parse, or if
/// `T`'s `Deserialize` impl rejects the resulting structure.
pub fn from_reader<R: Read, T: DeserializeOwned>(mut r: R) -> Result<T, Error> {
    let mut buf = String::new();
    r.read_to_string(&mut buf).map_err(Error::from)?;
    from_str(&buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)]
    fn from_str_parses_primitives() {
        assert!(from_str::<bool>("true").unwrap());
        assert_eq!(from_str::<i64>("42").unwrap(), 42i64);
        assert_eq!(from_str::<f64>("1.5").unwrap(), 1.5f64);
        assert_eq!(from_str::<String>("hello").unwrap(), "hello".to_string());
        assert_eq!(from_str::<Option<i32>>("null").unwrap(), None);
        assert_eq!(from_str::<Option<i32>>("7").unwrap(), Some(7));
    }

    #[test]
    fn vec_of_int_round_trips() {
        let v: Vec<i32> = from_str("- 1\n- 2\n- 3\n").unwrap();
        assert_eq!(v, vec![1, 2, 3]);
    }

    #[test]
    fn nested_seq_round_trips() {
        let v: Vec<Vec<i32>> = from_str("- - 1\n  - 2\n- - 3\n  - 4\n").unwrap();
        assert_eq!(v, vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn struct_round_trips_through_from_str() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        struct Cfg {
            name: String,
            version: u32,
            enabled: bool,
        }
        let cfg: Cfg = from_str("name: rust\nversion: 11\nenabled: true\n").unwrap();
        assert_eq!(
            cfg,
            Cfg {
                name: "rust".into(),
                version: 11,
                enabled: true
            }
        );
    }

    #[test]
    fn hashmap_round_trips_through_from_str() {
        use std::collections::HashMap;
        let m: HashMap<String, i32> = from_str("a: 1\nb: 2\n").unwrap();
        assert_eq!(m.get("a"), Some(&1));
        assert_eq!(m.get("b"), Some(&2));
    }

    #[test]
    fn from_slice_and_from_reader_match_from_str() {
        let input = "name: rust\nversion: 11\n";
        let bytes = input.as_bytes();
        let from_s: indexmap::IndexMap<String, serde_yaml::Value> = from_str(input).unwrap();
        let from_b: indexmap::IndexMap<String, serde_yaml::Value> = from_slice(bytes).unwrap();
        let from_r: indexmap::IndexMap<String, serde_yaml::Value> =
            from_reader(std::io::Cursor::new(input)).unwrap();
        assert_eq!(from_s, from_b);
        assert_eq!(from_s, from_r);
    }

    #[test]
    fn unit_variant_deserializes_from_string() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        enum Color {
            Red,
            Green,
            Blue,
        }
        let c: Color = from_str("Red").unwrap();
        assert_eq!(c, Color::Red);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn tuple_variant_deserializes_from_tagged_map() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        enum Shape {
            Circle(f64),
            Rect(f64, f64),
        }
        let c: Shape = from_str("Circle: 1.5\n").unwrap();
        assert_eq!(c, Shape::Circle(1.5));
        let r: Shape = from_str("Rect:\n  - 2.0\n  - 3.0\n").unwrap();
        assert_eq!(r, Shape::Rect(2.0, 3.0));
    }

    #[test]
    fn struct_variant_deserializes_from_tagged_map() {
        #[derive(serde::Deserialize, Debug, PartialEq)]
        enum Msg {
            Point { x: i32, y: i32 },
        }
        let p: Msg = from_str("Point:\n  x: 1\n  y: 2\n").unwrap();
        assert_eq!(p, Msg::Point { x: 1, y: 2 });
    }
}
