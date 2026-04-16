//! Direct Serde deserializer: sproto wire format -> Rust struct, no SprotoValue intermediate.

use serde::de::{self, DeserializeSeed, Visitor};

use crate::codec::wire::*;
use crate::types::{Field, FieldType, Sproto, SprotoType};

use super::error::SerdeError;

/// Decode sproto wire bytes directly into a Rust struct.
pub fn direct_decode<'de, T: de::Deserialize<'de>>(
    sproto: &'de Sproto,
    sproto_type: &'de SprotoType,
    data: &'de [u8],
) -> Result<T, SerdeError> {
    let deser = DirectDeserializer::new(sproto, sproto_type, data)?;
    T::deserialize(deser)
}

// ── Parsed field from wire header ───────────────────────────────────────

struct ParsedField<'de> {
    field: &'de Field,
    /// If >= 0, value is inline (decoded_value). If < 0, data is in data slice.
    inline_value: i32,
    /// Slice of field data (excluding length prefix). Empty if inline.
    data: &'de [u8],
}

// ── Top-level deserializer ──────────────────────────────────────────────

struct DirectDeserializer<'de> {
    sproto: &'de Sproto,
    parsed: Vec<ParsedField<'de>>,
}

impl<'de> DirectDeserializer<'de> {
    fn new(
        sproto: &'de Sproto,
        sproto_type: &'de SprotoType,
        data: &'de [u8],
    ) -> Result<Self, SerdeError> {
        let parsed = parse_wire_fields(sproto_type, data)?;
        Ok(DirectDeserializer { sproto, parsed })
    }
}

/// Parse wire header into a list of ParsedField entries.
fn parse_wire_fields<'de>(
    sproto_type: &'de SprotoType,
    data: &'de [u8],
) -> Result<Vec<ParsedField<'de>>, SerdeError> {
    let size = data.len();
    if size < SIZEOF_HEADER {
        return Err(SerdeError::Custom(format!("truncated header: need {}, have {}", SIZEOF_HEADER, size)));
    }
    let fn_count = read_u16_le(&data[0..]) as usize;
    let field_part_end = SIZEOF_HEADER + fn_count * SIZEOF_FIELD;
    if size < field_part_end {
        return Err(SerdeError::Custom(format!("truncated fields: need {}, have {}", field_part_end, size)));
    }
    let field_part = &data[SIZEOF_HEADER..field_part_end];
    let mut data_offset = field_part_end;
    let mut tag: i32 = -1;
    let mut result = Vec::with_capacity(fn_count);

    for i in 0..fn_count {
        let value = read_u16_le(&field_part[i * SIZEOF_FIELD..]) as i32;
        tag += 1;
        if value & 1 != 0 {
            tag += value / 2;
            continue;
        }
        let decoded_value = value / 2 - 1;
        let mut field_data: &[u8] = &[];
        if decoded_value < 0 {
            if data_offset + SIZEOF_LENGTH > size {
                return Err(SerdeError::Custom("truncated data length".into()));
            }
            let dsz = read_u32_le(&data[data_offset..]) as usize;
            if data_offset + SIZEOF_LENGTH + dsz > size {
                return Err(SerdeError::Custom("truncated data".into()));
            }
            field_data = &data[data_offset + SIZEOF_LENGTH..data_offset + SIZEOF_LENGTH + dsz];
            data_offset += SIZEOF_LENGTH + dsz;
        }
        let field = match sproto_type.find_field_by_tag(tag as u16) {
            Some(f) => f,
            None => continue,
        };
        result.push(ParsedField { field, inline_value: decoded_value, data: field_data });
    }
    Ok(result)
}

impl<'de> de::Deserializer<'de> for DirectDeserializer<'de> {
    type Error = SerdeError;

    fn deserialize_struct<V: Visitor<'de>>(
        self, _name: &'static str, _fields: &'static [&'static str], visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_map(WireMapAccess {
            sproto: self.sproto,
            parsed: self.parsed,
            index: 0,
            current: None,
        })
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(WireMapAccess {
            sproto: self.sproto,
            parsed: self.parsed,
            index: 0,
            current: None,
        })
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes
        byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct
        enum identifier ignored_any
    }
}

// ── WireMapAccess ───────────────────────────────────────────────────────

struct WireMapAccess<'de> {
    sproto: &'de Sproto,
    parsed: Vec<ParsedField<'de>>,
    index: usize,
    current: Option<usize>,
}

impl<'de> de::MapAccess<'de> for WireMapAccess<'de> {
    type Error = SerdeError;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error> {
        if self.index >= self.parsed.len() {
            return Ok(None);
        }
        self.current = Some(self.index);
        let name = &self.parsed[self.index].field.name;
        self.index += 1;
        seed.deserialize(StrDeserializer(&name)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Self::Error> {
        let idx = self.current.take().ok_or_else(|| SerdeError::Custom("value before key".into()))?;
        let pf = &self.parsed[idx];
        let deser = FieldValueDeserializer {
            sproto: self.sproto,
            field: pf.field,
            inline_value: pf.inline_value,
            data: pf.data,
        };
        seed.deserialize(deser)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.parsed.len() - self.index)
    }
}

// ── FieldValueDeserializer ──────────────────────────────────────────────

struct FieldValueDeserializer<'de> {
    sproto: &'de Sproto,
    field: &'de Field,
    inline_value: i32,
    data: &'de [u8],
}

impl<'de> FieldValueDeserializer<'de> {
    fn decode_integer(&self) -> Result<i64, SerdeError> {
        if self.inline_value >= 0 {
            return Ok(self.inline_value as i64);
        }
        let d = self.data;
        if d.len() == SIZEOF_INT32 {
            Ok(expand64(read_u32_le(d)) as i64)
        } else if d.len() == SIZEOF_INT64 {
            let lo = read_u32_le(d) as u64;
            let hi = read_u32_le(&d[SIZEOF_INT32..]) as u64;
            Ok((lo | (hi << 32)) as i64)
        } else {
            Err(SerdeError::Custom(format!("invalid integer size {}", d.len())))
        }
    }

    fn decode_double(&self) -> Result<f64, SerdeError> {
        let d = self.data;
        if d.len() == SIZEOF_INT64 {
            let lo = read_u32_le(d) as u64;
            let hi = read_u32_le(&d[SIZEOF_INT32..]) as u64;
            Ok(f64::from_bits(lo | (hi << 32)))
        } else {
            Err(SerdeError::Custom(format!("invalid double size {}", d.len())))
        }
    }
}

impl<'de> de::Deserializer<'de> for FieldValueDeserializer<'de> {
    type Error = SerdeError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        if self.field.is_array {
            return self.deserialize_seq(visitor);
        }
        match &self.field.field_type {
            FieldType::Integer => visitor.visit_i64(self.decode_integer()?),
            FieldType::Boolean => {
                let v = if self.inline_value >= 0 { self.inline_value != 0 } else { self.data.first().copied().unwrap_or(0) != 0 };
                visitor.visit_bool(v)
            }
            FieldType::Double => visitor.visit_f64(self.decode_double()?),
            FieldType::String => {
                let s = std::str::from_utf8(self.data).map_err(|e| SerdeError::Custom(format!("invalid utf8: {}", e)))?;
                visitor.visit_borrowed_str(s)
            }
            FieldType::Binary => visitor.visit_borrowed_bytes(self.data),
            FieldType::Struct(idx) => {
                let sub_type = &self.sproto.types_list[*idx];
                let sub = DirectDeserializer::new(self.sproto, sub_type, self.data)?;
                sub.deserialize_any(visitor)
            }
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let v = if self.inline_value >= 0 { self.inline_value != 0 } else { self.decode_integer()? != 0 };
        visitor.visit_bool(v)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_i64(self.decode_integer()?) }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_i64(self.decode_integer()?) }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_i64(self.decode_integer()?) }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_i64(self.decode_integer()?) }
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_u64(self.decode_integer()? as u64) }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_u64(self.decode_integer()? as u64) }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_u64(self.decode_integer()? as u64) }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_u64(self.decode_integer()? as u64) }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_f64(self.decode_double()?) }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_f64(self.decode_double()?) }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let s = std::str::from_utf8(self.data).map_err(|e| SerdeError::Custom(format!("invalid utf8: {}", e)))?;
        visitor.visit_borrowed_str(s)
    }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.deserialize_str(visitor) }
    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_borrowed_bytes(self.data) }
    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_byte_buf(self.data.to_vec()) }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_unit() }
    fn deserialize_unit_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_unit() }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let d = self.data;
        match &self.field.field_type {
            FieldType::Integer | FieldType::Double => {
                if d.is_empty() { return visitor.visit_seq(EmptySeq); }
                let int_len = d[0] as usize;
                let vals = &d[1..];
                visitor.visit_seq(NumArrayAccess { field: self.field, data: vals, int_len, offset: 0 })
            }
            FieldType::Boolean => {
                visitor.visit_seq(BoolArrayAccess { data: d, offset: 0 })
            }
            FieldType::String | FieldType::Binary | FieldType::Struct(_) => {
                visitor.visit_seq(ObjectArrayAccess { sproto: self.sproto, field: self.field, data: d, offset: 0 })
            }
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error> { self.deserialize_seq(visitor) }
    fn deserialize_tuple_struct<V: Visitor<'de>>(self, _: &'static str, _: usize, visitor: V) -> Result<V::Value, Self::Error> { self.deserialize_seq(visitor) }

    fn deserialize_struct<V: Visitor<'de>>(self, _name: &'static str, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> {
        let sub_type = match &self.field.field_type {
            FieldType::Struct(idx) => &self.sproto.types_list[*idx],
            _ => return Err(SerdeError::TypeMismatch { field: self.field.name.to_string(), expected: "struct".into(), actual: "non-struct".into() }),
        };
        let sub = DirectDeserializer::new(self.sproto, sub_type, self.data)?;
        sub.deserialize_struct(_name, _fields, visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.deserialize_any(visitor) }

    fn deserialize_enum<V: Visitor<'de>>(self, _: &'static str, _: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error> {
        let v = self.decode_integer()?;
        visitor.visit_enum(EnumAccess(v))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { self.deserialize_str(visitor) }
    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_unit() }
    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let s = std::str::from_utf8(self.data).map_err(|e| SerdeError::Custom(format!("invalid utf8: {}", e)))?;
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(SerdeError::Custom(format!("expected single char, got len {}", s.len()))),
        }
    }
}

// ── Array access types ──────────────────────────────────────────────────

struct EmptySeq;
impl<'de> de::SeqAccess<'de> for EmptySeq {
    type Error = SerdeError;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, _: T) -> Result<Option<T::Value>, Self::Error> { Ok(None) }
    fn size_hint(&self) -> Option<usize> { Some(0) }
}

struct NumArrayAccess<'de> { field: &'de Field, data: &'de [u8], int_len: usize, offset: usize }

impl<'de> de::SeqAccess<'de> for NumArrayAccess<'de> {
    type Error = SerdeError;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
        if self.offset >= self.data.len() { return Ok(None); }
        let chunk = &self.data[self.offset..];
        self.offset += self.int_len;
        let is_double = self.field.field_type == FieldType::Double;
        if self.int_len == SIZEOF_INT32 {
            let raw = expand64(read_u32_le(chunk));
            if is_double {
                seed.deserialize(F64Deser(f64::from_bits(raw))).map(Some)
            } else {
                seed.deserialize(I64Deser(raw as i64)).map(Some)
            }
        } else {
            let lo = read_u32_le(chunk) as u64;
            let hi = read_u32_le(&chunk[SIZEOF_INT32..]) as u64;
            let raw = lo | (hi << 32);
            if is_double {
                seed.deserialize(F64Deser(f64::from_bits(raw))).map(Some)
            } else {
                seed.deserialize(I64Deser(raw as i64)).map(Some)
            }
        }
    }
    fn size_hint(&self) -> Option<usize> {
        if self.int_len == 0 { Some(0) } else { Some((self.data.len() - self.offset) / self.int_len) }
    }
}

struct BoolArrayAccess<'de> { data: &'de [u8], offset: usize }

impl<'de> de::SeqAccess<'de> for BoolArrayAccess<'de> {
    type Error = SerdeError;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
        if self.offset >= self.data.len() { return Ok(None); }
        let v = self.data[self.offset] != 0;
        self.offset += 1;
        seed.deserialize(BoolDeser(v)).map(Some)
    }
    fn size_hint(&self) -> Option<usize> { Some(self.data.len() - self.offset) }
}

struct ObjectArrayAccess<'de> { sproto: &'de Sproto, field: &'de Field, data: &'de [u8], offset: usize }

impl<'de> de::SeqAccess<'de> for ObjectArrayAccess<'de> {
    type Error = SerdeError;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error> {
        if self.offset >= self.data.len() { return Ok(None); }
        let d = &self.data[self.offset..];
        if d.len() < SIZEOF_LENGTH {
            return Err(SerdeError::Custom("truncated object array element".into()));
        }
        let esz = read_u32_le(d) as usize;
        let elem = &d[SIZEOF_LENGTH..SIZEOF_LENGTH + esz];
        self.offset += SIZEOF_LENGTH + esz;
        match &self.field.field_type {
            FieldType::String => {
                let s = std::str::from_utf8(elem).map_err(|e| SerdeError::Custom(format!("invalid utf8: {}", e)))?;
                seed.deserialize(StrValDeser(s)).map(Some)
            }
            FieldType::Binary => {
                seed.deserialize(BytesDeser(elem)).map(Some)
            }
            FieldType::Struct(idx) => {
                let st = &self.sproto.types_list[*idx];
                let sub = DirectDeserializer::new(self.sproto, st, elem)?;
                seed.deserialize(sub).map(Some)
            }
            _ => Err(SerdeError::Custom("unexpected array element type".into())),
        }
    }
}

// ── Primitive deserializers ─────────────────────────────────────────────

struct I64Deser(i64);
impl<'de> de::Deserializer<'de> for I64Deser {
    type Error = SerdeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_i64(self.0) }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct F64Deser(f64);
impl<'de> de::Deserializer<'de> for F64Deser {
    type Error = SerdeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_f64(self.0) }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct BoolDeser(bool);
impl<'de> de::Deserializer<'de> for BoolDeser {
    type Error = SerdeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_bool(self.0) }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct StrDeserializer<'a>(&'a str);
impl<'de, 'a> de::Deserializer<'de> for StrDeserializer<'a> {
    type Error = SerdeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_str(self.0) }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct StrValDeser<'a>(&'a str);
impl<'de, 'a> de::Deserializer<'de> for StrValDeser<'a> {
    type Error = SerdeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_str(self.0) }
    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_string(self.0.to_owned()) }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct BytesDeser<'a>(&'a [u8]);
impl<'de, 'a> de::Deserializer<'de> for BytesDeser<'a> {
    type Error = SerdeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_bytes(self.0) }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

// ── Enum support ────────────────────────────────────────────────────────

struct EnumAccess(i64);
impl<'de> de::EnumAccess<'de> for EnumAccess {
    type Error = SerdeError;
    type Variant = UnitVariant;
    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error> {
        let v = seed.deserialize(U32Deser(self.0 as u32))?;
        Ok((v, UnitVariant))
    }
}

struct U32Deser(u32);
impl<'de> de::Deserializer<'de> for U32Deser {
    type Error = SerdeError;
    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { visitor.visit_u32(self.0) }
    serde::forward_to_deserialize_any! { bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any }
}

struct UnitVariant;
impl<'de> de::VariantAccess<'de> for UnitVariant {
    type Error = SerdeError;
    fn unit_variant(self) -> Result<(), Self::Error> { Ok(()) }
    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, _: T) -> Result<T::Value, Self::Error> { Err(SerdeError::UnsupportedType("newtype variant".into())) }
    fn tuple_variant<V: Visitor<'de>>(self, _: usize, _: V) -> Result<V::Value, Self::Error> { Err(SerdeError::UnsupportedType("tuple variant".into())) }
    fn struct_variant<V: Visitor<'de>>(self, _: &'static [&'static str], _: V) -> Result<V::Value, Self::Error> { Err(SerdeError::UnsupportedType("struct variant".into())) }
}
