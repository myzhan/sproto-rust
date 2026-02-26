//! Serde deserializer for converting SprotoValue to Rust types.

use std::collections::HashMap;

use serde::de::{self, DeserializeSeed, Visitor};

use super::error::SerdeError;
use crate::value::SprotoValue;

/// Deserializer that converts SprotoValue to Rust types.
pub struct SprotoDeserializer<'de> {
    value: &'de SprotoValue,
}

impl<'de> SprotoDeserializer<'de> {
    /// Create a new deserializer from a SprotoValue.
    pub fn new(value: &'de SprotoValue) -> Self {
        SprotoDeserializer { value }
    }

    /// Deserialize a SprotoValue to the target type.
    pub fn deserialize<T: de::Deserialize<'de>>(value: &'de SprotoValue) -> Result<T, SerdeError> {
        T::deserialize(SprotoDeserializer::new(value))
    }
}

impl<'de> de::Deserializer<'de> for SprotoDeserializer<'de> {
    type Error = SerdeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Integer(v) => visitor.visit_i64(*v),
            SprotoValue::Boolean(v) => visitor.visit_bool(*v),
            SprotoValue::Str(v) => visitor.visit_str(v),
            SprotoValue::Binary(v) => visitor.visit_bytes(v),
            SprotoValue::Double(v) => visitor.visit_f64(*v),
            SprotoValue::Struct(_) => self.deserialize_map(visitor),
            SprotoValue::Array(_) => self.deserialize_seq(visitor),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Boolean(v) => visitor.visit_bool(*v),
            SprotoValue::Integer(v) => visitor.visit_bool(*v != 0),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "bool".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Integer(v) => visitor.visit_i64(*v),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "integer".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_u64(visitor)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Integer(v) => visitor.visit_u64(*v as u64),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "integer".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_f64(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Double(v) => visitor.visit_f64(*v),
            SprotoValue::Integer(v) => visitor.visit_f64(*v as f64),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "double".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Str(s) => {
                let mut chars = s.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) => visitor.visit_char(c),
                    _ => Err(SerdeError::TypeMismatch {
                        field: String::new(),
                        expected: "single character".into(),
                        actual: format!("string of length {}", s.len()),
                    }),
                }
            }
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "char".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Str(s) => visitor.visit_str(s),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "string".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Binary(b) => visitor.visit_bytes(b),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "binary".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Binary(b) => visitor.visit_byte_buf(b.clone()),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "binary".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // If we get here with a value, it's Some
        visitor.visit_some(self)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Array(arr) => visitor.visit_seq(SeqAccess::new(arr)),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "array".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Struct(map) => visitor.visit_map(MapAccess::new(map)),
            _ => Err(SerdeError::TypeMismatch {
                field: String::new(),
                expected: "struct".into(),
                actual: self.value.type_name().into(),
            }),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.value {
            SprotoValue::Integer(v) => visitor.visit_enum(EnumAccess { value: *v }),
            _ => Err(SerdeError::UnsupportedType(
                "enums must be encoded as integers".into(),
            )),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }
}

/// Sequence access for deserializing arrays.
struct SeqAccess<'de> {
    iter: std::slice::Iter<'de, SprotoValue>,
}

impl<'de> SeqAccess<'de> {
    fn new(arr: &'de [SprotoValue]) -> Self {
        SeqAccess { iter: arr.iter() }
    }
}

impl<'de> de::SeqAccess<'de> for SeqAccess<'de> {
    type Error = SerdeError;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Self::Error> {
        match self.iter.next() {
            Some(value) => seed.deserialize(SprotoDeserializer::new(value)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

/// Map access for deserializing structs.
struct MapAccess<'de> {
    iter: std::collections::hash_map::Iter<'de, String, SprotoValue>,
    current_value: Option<&'de SprotoValue>,
}

impl<'de> MapAccess<'de> {
    fn new(map: &'de HashMap<String, SprotoValue>) -> Self {
        MapAccess {
            iter: map.iter(),
            current_value: None,
        }
    }
}

impl<'de> de::MapAccess<'de> for MapAccess<'de> {
    type Error = SerdeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        match self.iter.next() {
            Some((key, value)) => {
                self.current_value = Some(value);
                // Deserialize the key as a string
                seed.deserialize(StrDeserializer(key)).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        let value = self.current_value.take().ok_or_else(|| {
            SerdeError::Custom("next_value_seed called before next_key_seed".into())
        })?;
        seed.deserialize(SprotoDeserializer::new(value))
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

/// Simple deserializer for string keys.
struct StrDeserializer<'a>(&'a str);

impl<'de, 'a> de::Deserializer<'de> for StrDeserializer<'a> {
    type Error = SerdeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_str(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }
}

/// Enum access for deserializing unit variants as integers.
struct EnumAccess {
    value: i64,
}

impl<'de> de::EnumAccess<'de> for EnumAccess {
    type Error = SerdeError;
    type Variant = VariantAccess;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, Self::Variant), Self::Error> {
        let variant = seed.deserialize(U32Deserializer(self.value as u32))?;
        Ok((variant, VariantAccess))
    }
}

/// Simple deserializer for u32 (enum discriminant).
struct U32Deserializer(u32);

impl<'de> de::Deserializer<'de> for U32Deserializer {
    type Error = SerdeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_u32(self.0)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        map struct enum identifier ignored_any
    }
}

/// Variant access for unit enum variants.
struct VariantAccess;

impl<'de> de::VariantAccess<'de> for VariantAccess {
    type Error = SerdeError;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(
        self,
        _seed: T,
    ) -> Result<T::Value, Self::Error> {
        Err(SerdeError::UnsupportedType(
            "newtype variants are not supported".into(),
        ))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(SerdeError::UnsupportedType(
            "tuple variants are not supported".into(),
        ))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(SerdeError::UnsupportedType(
            "struct variants are not supported".into(),
        ))
    }
}
