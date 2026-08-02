// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `Serialize` / `Deserialize` impls for the public `Value` type.

use crate::Value;
use serde::{
    de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor},
    ser::{Serialize, SerializeMap, SerializeSeq, Serializer},
};
use std::fmt;

impl Serialize for Value {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        match self {
            Value::Null => ser.serialize_unit(),
            Value::Bool(b) => ser.serialize_bool(*b),
            Value::Int(i) => ser.serialize_i64(*i),
            Value::Float(f) => ser.serialize_f64(*f),
            Value::String(s) => ser.serialize_str(s),
            Value::Sequence(sequence) => {
                let mut s = ser.serialize_seq(Some(sequence.len()))?;
                for item in sequence {
                    s.serialize_element(item)?;
                }
                s.end()
            }
            Value::Mapping(map) => {
                let mut m = ser.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    m.serialize_entry(k, v)?;
                }
                m.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_any(ValueVisitor)
    }
}

struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("any YAML-compatible value")
    }

    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        Value::deserialize(d)
    }

    fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
        Ok(Value::Bool(b))
    }

    fn visit_i64<E>(self, i: i64) -> Result<Value, E> {
        Ok(Value::Int(i))
    }
    fn visit_i128<E: serde::de::Error>(self, i: i128) -> Result<Value, E> {
        i64::try_from(i)
            .map(Value::Int)
            .map_err(|_| E::custom("integer out of range for i64"))
    }
    fn visit_u64<E: serde::de::Error>(self, u: u64) -> Result<Value, E> {
        i64::try_from(u)
            .map(Value::Int)
            .map_err(|_| E::custom("integer out of range for i64"))
    }
    fn visit_u128<E: serde::de::Error>(self, u: u128) -> Result<Value, E> {
        i64::try_from(u)
            .map(Value::Int)
            .map_err(|_| E::custom("integer out of range for i64"))
    }

    fn visit_f64<E>(self, f: f64) -> Result<Value, E> {
        Ok(Value::Float(f))
    }

    fn visit_str<E>(self, s: &str) -> Result<Value, E> {
        Ok(Value::String(s.to_owned()))
    }
    fn visit_string<E>(self, s: String) -> Result<Value, E> {
        Ok(Value::String(s))
    }
    fn visit_borrowed_str<E>(self, s: &'de str) -> Result<Value, E> {
        Ok(Value::String(s.to_owned()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(elem) = seq.next_element()? {
            out.push(elem);
        }
        Ok(Value::Sequence(out))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut out = indexmap::IndexMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((k, v)) = map.next_entry()? {
            out.insert(k, v);
        }
        Ok(Value::Mapping(out))
    }
}

#[cfg(test)]
mod ser_tests {
    use crate::Value;
    use indexmap::IndexMap;

    #[test]
    fn value_serializes_through_serde_json() {
        let mut m = IndexMap::new();
        m.insert(Value::String("name".into()), Value::String("rust".into()));
        m.insert(Value::String("ver".into()), Value::Int(11));
        let v = Value::Mapping(m);

        let j = serde_json::to_string(&v).expect("serialize through serde_json");
        assert_eq!(j, r#"{"name":"rust","ver":11}"#);
    }

    #[test]
    fn null_int_float_bool_seq_serialize() {
        assert_eq!(serde_json::to_string(&Value::Null).unwrap(), "null");
        assert_eq!(serde_json::to_string(&Value::Bool(true)).unwrap(), "true");
        assert_eq!(serde_json::to_string(&Value::Int(42)).unwrap(), "42");
        assert_eq!(serde_json::to_string(&Value::Float(1.5)).unwrap(), "1.5");
        let seq = Value::Sequence(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(serde_json::to_string(&seq).unwrap(), "[1,2]");
    }
}

#[cfg(test)]
mod de_tests {
    use crate::Value;

    #[test]
    fn value_deserializes_from_serde_json() {
        let json = r#"{"name":"rust","ver":11,"flags":[true,false]}"#;
        let v: Value = serde_json::from_str(json).expect("parse via serde_json");
        let map = v.as_mapping().expect("mapping");
        assert_eq!(
            map.get(&Value::String("name".into())),
            Some(&Value::String("rust".into()))
        );
        assert_eq!(map.get(&Value::String("ver".into())), Some(&Value::Int(11)));
    }
}
