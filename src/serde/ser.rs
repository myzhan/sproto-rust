//! Serde serializer for converting Rust types to SprotoValue.

use std::collections::HashMap;

use serde::ser::{self, Serialize};

use super::error::SerdeError;
use crate::value::SprotoValue;

/// Serializer that converts Rust types to SprotoValue.
pub struct SprotoSerializer;

impl SprotoSerializer {
    /// Serialize a value to SprotoValue.
    pub fn serialize<T: Serialize>(value: &T) -> Result<SprotoValue, SerdeError> {
        value.serialize(SprotoSerializer)
    }
}

impl ser::Serializer for SprotoSerializer {
    type Ok = SprotoValue;
    type Error = SerdeError;

    type SerializeSeq = SeqSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = SeqSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Boolean(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Integer(v as i64))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Integer(v as i64))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Integer(v as i64))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Integer(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Integer(v as i64))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Integer(v as i64))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Integer(v as i64))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        // Note: u64 values > i64::MAX will wrap. User should be aware.
        Ok(SprotoValue::Integer(v as i64))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Double(v as f64))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Double(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Str(v.to_string()))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Str(v.to_string()))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Binary(v.to_vec()))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        // None is represented by absence in sproto, but we need a placeholder
        // This will be filtered out by StructSerializer
        Err(SerdeError::Custom(
            "None values should be handled by Option serialization".into(),
        ))
    }

    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        // Unit type serializes as empty struct
        Ok(SprotoValue::Struct(HashMap::new()))
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        // Encode enum unit variants as integers
        Ok(SprotoValue::Integer(variant_index as i64))
    }

    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(SerdeError::UnsupportedType(
            "enum variants with data are not supported by sproto".into(),
        ))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Ok(SeqSerializer {
            elements: Vec::with_capacity(len.unwrap_or(0)),
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(SerdeError::UnsupportedType(
            "enum tuple variants are not supported by sproto".into(),
        ))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Ok(MapSerializer {
            fields: HashMap::with_capacity(len.unwrap_or(0)),
            current_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(StructSerializer {
            fields: HashMap::with_capacity(len),
        })
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(SerdeError::UnsupportedType(
            "enum struct variants are not supported by sproto".into(),
        ))
    }
}

/// Serializer for sequences/arrays.
pub struct SeqSerializer {
    elements: Vec<SprotoValue>,
}

impl ser::SerializeSeq for SeqSerializer {
    type Ok = SprotoValue;
    type Error = SerdeError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        self.elements.push(value.serialize(SprotoSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Array(self.elements))
    }
}

impl ser::SerializeTuple for SeqSerializer {
    type Ok = SprotoValue;
    type Error = SerdeError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SeqSerializer {
    type Ok = SprotoValue;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for SeqSerializer {
    type Ok = SprotoValue;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeSeq::end(self)
    }
}

/// Serializer for maps (string keys only).
pub struct MapSerializer {
    fields: HashMap<String, SprotoValue>,
    current_key: Option<String>,
}

impl ser::SerializeMap for MapSerializer {
    type Ok = SprotoValue;
    type Error = SerdeError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> Result<(), Self::Error> {
        // We need to extract the key as a string
        let key_value = key.serialize(SprotoSerializer)?;
        match key_value {
            SprotoValue::Str(s) => {
                self.current_key = Some(s);
                Ok(())
            }
            _ => Err(SerdeError::UnsupportedType(
                "map keys must be strings".into(),
            )),
        }
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), Self::Error> {
        let key = self
            .current_key
            .take()
            .ok_or_else(|| SerdeError::Custom("serialize_value called before serialize_key".into()))?;
        let val = value.serialize(SprotoSerializer)?;
        self.fields.insert(key, val);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Struct(self.fields))
    }
}

/// Serializer for structs.
pub struct StructSerializer {
    fields: HashMap<String, SprotoValue>,
}

impl ser::SerializeStruct for StructSerializer {
    type Ok = SprotoValue;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        // Try to serialize the value. If it fails with "None" error, skip the field.
        match value.serialize(SprotoSerializer) {
            Ok(val) => {
                self.fields.insert(key.to_string(), val);
                Ok(())
            }
            Err(SerdeError::Custom(msg)) if msg.contains("None values") => {
                // Skip None fields
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(SprotoValue::Struct(self.fields))
    }

    fn skip_field(&mut self, _key: &'static str) -> Result<(), Self::Error> {
        // Field is skipped, nothing to do
        Ok(())
    }
}

impl ser::SerializeStructVariant for StructSerializer {
    type Ok = SprotoValue;
    type Error = SerdeError;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        ser::SerializeStruct::serialize_field(self, key, value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        ser::SerializeStruct::end(self)
    }
}
