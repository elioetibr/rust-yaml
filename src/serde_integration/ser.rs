// SPDX-FileCopyrightText: Rust Yaml contributors
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! serde::Serializer that builds a `Value` tree.
//!
//! The public `to_string` / `to_writer` entry points (Task 8) will serialize
//! `T` into a `Value` via this serializer, then delegate to the existing
//! `dump_str` pipeline.

use crate::{Error, Value, Yaml};
use indexmap::IndexMap;
use serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};

/// Serializer that emits a `Value`. Stateless; one per call.
pub struct ValueSerializer;

impl Serializer for ValueSerializer {
    type Ok = Value;
    type Error = Error;

    type SerializeSeq = SeqBuilder;
    type SerializeTuple = SeqBuilder;
    type SerializeTupleStruct = SeqBuilder;
    type SerializeTupleVariant = TupleVariantBuilder;
    type SerializeMap = MapBuilder;
    type SerializeStruct = MapBuilder;
    type SerializeStructVariant = StructVariantBuilder;

    fn serialize_bool(self, v: bool) -> Result<Value, Error> {
        Ok(Value::Bool(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Value, Error> {
        Ok(Value::Int(i64::from(v)))
    }

    fn serialize_i16(self, v: i16) -> Result<Value, Error> {
        Ok(Value::Int(i64::from(v)))
    }

    fn serialize_i32(self, v: i32) -> Result<Value, Error> {
        Ok(Value::Int(i64::from(v)))
    }

    fn serialize_i64(self, v: i64) -> Result<Value, Error> {
        Ok(Value::Int(v))
    }

    fn serialize_i128(self, v: i128) -> Result<Value, Error> {
        i64::try_from(v)
            .map(Value::Int)
            .map_err(|_| <Error as serde::ser::Error>::custom("integer out of range for i64"))
    }

    fn serialize_u8(self, v: u8) -> Result<Value, Error> {
        Ok(Value::Int(i64::from(v)))
    }

    fn serialize_u16(self, v: u16) -> Result<Value, Error> {
        Ok(Value::Int(i64::from(v)))
    }

    fn serialize_u32(self, v: u32) -> Result<Value, Error> {
        Ok(Value::Int(i64::from(v)))
    }

    fn serialize_u64(self, v: u64) -> Result<Value, Error> {
        i64::try_from(v)
            .map(Value::Int)
            .map_err(|_| <Error as serde::ser::Error>::custom("u64 above i64::MAX"))
    }

    fn serialize_u128(self, v: u128) -> Result<Value, Error> {
        i64::try_from(v)
            .map(Value::Int)
            .map_err(|_| <Error as serde::ser::Error>::custom("integer out of range for i64"))
    }

    fn serialize_f32(self, v: f32) -> Result<Value, Error> {
        Ok(Value::Float(f64::from(v)))
    }

    fn serialize_f64(self, v: f64) -> Result<Value, Error> {
        Ok(Value::Float(v))
    }

    fn serialize_char(self, c: char) -> Result<Value, Error> {
        Ok(Value::String(c.to_string()))
    }

    fn serialize_str(self, s: &str) -> Result<Value, Error> {
        Ok(Value::String(s.to_owned()))
    }

    fn serialize_bytes(self, b: &[u8]) -> Result<Value, Error> {
        // v1.1.0: bytes as Sequence<Int>. !!binary tagged form deferred.
        Ok(Value::Sequence(
            b.iter().map(|x| Value::Int(i64::from(*x))).collect(),
        ))
    }

    fn serialize_none(self) -> Result<Value, Error> {
        Ok(Value::Null)
    }

    fn serialize_some<T: ?Sized + Serialize>(self, v: &T) -> Result<Value, Error> {
        v.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Value, Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Value, Error> {
        Ok(Value::String(variant.to_owned()))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<Value, Error> {
        v.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        v: &T,
    ) -> Result<Value, Error> {
        let mut m = IndexMap::new();
        m.insert(
            Value::String(variant.to_owned()),
            v.serialize(ValueSerializer)?,
        );
        Ok(Value::Mapping(m))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Error> {
        Ok(SeqBuilder::with_capacity(len.unwrap_or(0)))
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Error> {
        Ok(SeqBuilder::with_capacity(len))
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Error> {
        Ok(SeqBuilder::with_capacity(len))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Error> {
        Ok(TupleVariantBuilder {
            variant,
            items: Vec::with_capacity(len),
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Error> {
        Ok(MapBuilder::with_capacity(len.unwrap_or(0)))
    }

    fn serialize_struct(self, _: &'static str, len: usize) -> Result<Self::SerializeStruct, Error> {
        Ok(MapBuilder::with_capacity(len))
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Error> {
        Ok(StructVariantBuilder {
            variant,
            fields: IndexMap::with_capacity(len),
        })
    }
}

/// Accumulates elements for sequences, tuples, and tuple structs.
pub struct SeqBuilder {
    items: Vec<Value>,
}

impl SeqBuilder {
    fn with_capacity(n: usize) -> Self {
        Self {
            items: Vec::with_capacity(n),
        }
    }
}

impl SerializeSeq for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        self.items.push(v.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Sequence(self.items))
    }
}

impl SerializeTuple for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        self.items.push(v.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Sequence(self.items))
    }
}

impl SerializeTupleStruct for SeqBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        self.items.push(v.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Sequence(self.items))
    }
}

/// Accumulates elements for an externally-tagged tuple variant.
pub struct TupleVariantBuilder {
    variant: &'static str,
    items: Vec<Value>,
}

impl SerializeTupleVariant for TupleVariantBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        self.items.push(v.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        let mut m = IndexMap::with_capacity(1);
        m.insert(
            Value::String(self.variant.to_owned()),
            Value::Sequence(self.items),
        );
        Ok(Value::Mapping(m))
    }
}

/// Accumulates key-value pairs for maps and structs.
pub struct MapBuilder {
    map: IndexMap<Value, Value>,
    next_key: Option<Value>,
}

impl MapBuilder {
    fn with_capacity(n: usize) -> Self {
        Self {
            map: IndexMap::with_capacity(n),
            next_key: None,
        }
    }
}

impl SerializeMap for MapBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, k: &T) -> Result<(), Error> {
        self.next_key = Some(k.serialize(ValueSerializer)?);
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, v: &T) -> Result<(), Error> {
        let key = self
            .next_key
            .take()
            .ok_or_else(|| <Error as serde::ser::Error>::custom("serialize_value without key"))?;
        self.map.insert(key, v.serialize(ValueSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Mapping(self.map))
    }
}

impl SerializeStruct for MapBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        name: &'static str,
        v: &T,
    ) -> Result<(), Error> {
        self.map.insert(
            Value::String(name.to_owned()),
            v.serialize(ValueSerializer)?,
        );
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Mapping(self.map))
    }
}

/// Accumulates fields for an externally-tagged struct variant.
pub struct StructVariantBuilder {
    variant: &'static str,
    fields: IndexMap<Value, Value>,
}

impl SerializeStructVariant for StructVariantBuilder {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        name: &'static str,
        v: &T,
    ) -> Result<(), Error> {
        self.fields.insert(
            Value::String(name.to_owned()),
            v.serialize(ValueSerializer)?,
        );
        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        let mut outer = IndexMap::with_capacity(1);
        outer.insert(
            Value::String(self.variant.to_owned()),
            Value::Mapping(self.fields),
        );
        Ok(Value::Mapping(outer))
    }
}

use std::io::Write;

/// Serialize `T` to a YAML string.
///
/// Source-compatible with `serde_yaml::to_string`.
///
/// # Errors
///
/// Returns an error if `T`'s `Serialize` impl fails or if YAML emission fails.
pub fn to_string<T: ?Sized + Serialize>(v: &T) -> Result<String, Error> {
    let val = v.serialize(ValueSerializer)?;
    Yaml::new().dump_str(&val)
}

/// Serialize `T` into the given writer as YAML.
///
/// Source-compatible with `serde_yaml::to_writer`.
///
/// # Errors
///
/// Returns an error if serialization fails or if writing to `w` fails.
pub fn to_writer<W: Write, T: ?Sized + Serialize>(mut w: W, v: &T) -> Result<(), Error> {
    let s = to_string(v)?;
    write_all_or_io_error(&mut w, s.as_bytes())
}

fn write_all_or_io_error<W: Write>(w: &mut W, bytes: &[u8]) -> Result<(), Error> {
    w.write_all(bytes).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use serde::Serialize;

    fn ser<T: Serialize>(v: T) -> Value {
        v.serialize(ValueSerializer).expect("serialize")
    }

    #[test]
    fn primitives_round_trip_to_value() {
        assert_eq!(ser(()), Value::Null);
        assert_eq!(ser(Option::<i32>::None), Value::Null);
        assert_eq!(ser(true), Value::Bool(true));
        assert_eq!(ser(7i32), Value::Int(7));
        assert_eq!(ser(7u32), Value::Int(7));
        assert_eq!(ser(1.5f32), Value::Float(1.5));
        assert_eq!(ser("hi"), Value::String("hi".into()));
        assert_eq!(ser('x'), Value::String("x".into()));
    }

    #[test]
    fn u64_above_i64_max_errors() {
        let huge: u64 = (i64::MAX as u64) + 1;
        assert!(huge.serialize(ValueSerializer).is_err());
    }

    #[test]
    fn vec_and_tuple_serialize_to_sequence() {
        assert_eq!(
            ser(vec![1i32, 2, 3]),
            Value::Sequence(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
        assert_eq!(
            ser((10u8, "x", true)),
            Value::Sequence(vec![
                Value::Int(10),
                Value::String("x".into()),
                Value::Bool(true)
            ])
        );
    }

    #[test]
    fn tuple_variant_serializes_to_externally_tagged_map() {
        #[derive(serde::Serialize)]
        enum E {
            Pair(i32, i32),
        }
        let v = ser(E::Pair(1, 2));
        let m = v.as_mapping().expect("mapping");
        assert_eq!(
            m.get(&Value::String("Pair".into())),
            Some(&Value::Sequence(vec![Value::Int(1), Value::Int(2)]))
        );
    }

    #[test]
    fn struct_serializes_to_mapping_preserving_field_order() {
        #[derive(serde::Serialize)]
        struct Cfg {
            name: String,
            version: u32,
            enabled: bool,
        }
        let v = ser(Cfg {
            name: "rust".into(),
            version: 11,
            enabled: true,
        });
        let m = v.as_mapping().expect("mapping");
        let keys: Vec<&Value> = m.keys().collect();
        assert_eq!(
            keys,
            vec![
                &Value::String("name".into()),
                &Value::String("version".into()),
                &Value::String("enabled".into())
            ]
        );
        assert_eq!(
            m.get(&Value::String("version".into())),
            Some(&Value::Int(11))
        );
    }

    #[test]
    fn struct_variant_serializes_to_externally_tagged_map() {
        #[derive(serde::Serialize)]
        enum E {
            Point { x: i32, y: i32 },
        }
        let v = ser(E::Point { x: 3, y: 4 });
        let m = v.as_mapping().expect("outer map");
        let inner = m
            .get(&Value::String("Point".into()))
            .expect("Point variant")
            .as_mapping()
            .expect("inner map");
        assert_eq!(inner.get(&Value::String("x".into())), Some(&Value::Int(3)));
        assert_eq!(inner.get(&Value::String("y".into())), Some(&Value::Int(4)));
    }

    #[test]
    fn to_string_emits_yaml_for_struct() {
        #[derive(serde::Serialize)]
        struct Cfg {
            name: String,
            version: u32,
        }
        let yaml = to_string(&Cfg {
            name: "rust".into(),
            version: 11,
        })
        .expect("to_string");
        assert!(yaml.contains("name: rust"));
        assert!(yaml.contains("version: 11"));
    }

    #[test]
    fn to_writer_writes_same_output_as_to_string() {
        let v = vec![1i32, 2, 3];
        let s = to_string(&v).unwrap();
        let mut buf = Vec::<u8>::new();
        to_writer(&mut buf, &v).unwrap();
        assert_eq!(std::str::from_utf8(&buf).unwrap(), s);
    }
}
