//! Direct Serde serializer: Rust struct -> sproto wire format, no SprotoValue intermediate.

use serde::ser::{self, Serialize};

use crate::codec::wire::*;
use crate::types::{Field, FieldType, Sproto, SprotoType};

use super::error::SerdeError;

/// Encode a Rust struct directly to sproto wire format.
pub fn direct_encode<T: Serialize>(
    sproto: &Sproto,
    sproto_type: &SprotoType,
    value: &T,
    output: &mut Vec<u8>,
) -> Result<(), SerdeError> {
    value.serialize(TopLevelSerializer {
        sproto,
        sproto_type,
        output,
    })
}

enum FieldResult {
    Inline(u16),
    DataWritten,
    Skip,
}

#[derive(Clone)]
enum FieldEntry {
    Inline(u16),
    Data { start: usize, len: usize },
}

struct StructCore<'a> {
    sproto: &'a Sproto,
    sproto_type: &'a SprotoType,
    output: &'a mut Vec<u8>,
    output_base: usize,
    entries: Vec<Option<FieldEntry>>,
    in_order: bool,
    last_data_tag: i32,
}

impl<'a> StructCore<'a> {
    fn new(sproto: &'a Sproto, sproto_type: &'a SprotoType, output: &'a mut Vec<u8>) -> Self {
        let output_base = output.len();
        let header_sz = SIZEOF_HEADER + sproto_type.maxn * SIZEOF_FIELD;
        output.resize(output_base + header_sz, 0);
        StructCore {
            sproto,
            sproto_type,
            output,
            output_base,
            entries: vec![None; sproto_type.fields.len()],
            in_order: true,
            last_data_tag: -1,
        }
    }

    fn serialize_field_inner<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), SerdeError> {
        let sproto = self.sproto;
        let sproto_type = self.sproto_type;
        let (idx, field) = match sproto_type.field_index_by_name(key) {
            Some(v) => v,
            None => return Ok(()),
        };
        let data_start = self.output.len();
        let field_ser = FieldValueSerializer {
            sproto,
            field,
            buf: &mut *self.output,
        };
        match value.serialize(field_ser) {
            Ok(FieldResult::Inline(v)) => {
                self.entries[idx] = Some(FieldEntry::Inline(v));
            }
            Ok(FieldResult::DataWritten) => {
                let data_len = self.output.len() - data_start;
                self.entries[idx] = Some(FieldEntry::Data {
                    start: data_start,
                    len: data_len,
                });
                let tag = field.tag as i32;
                if tag <= self.last_data_tag {
                    self.in_order = false;
                }
                self.last_data_tag = tag;
            }
            Ok(FieldResult::Skip) => {
                self.output.truncate(data_start);
            }
            Err(e) => {
                self.output.truncate(data_start);
                return Err(e);
            }
        }
        Ok(())
    }

    fn assemble(self) -> &'a mut Vec<u8> {
        if self.in_order {
            self.assemble_inorder()
        } else {
            self.assemble_reorder()
        }
    }

    #[inline]
    fn assemble_inorder(self) -> &'a mut Vec<u8> {
        let header_sz = SIZEOF_HEADER + self.sproto_type.maxn * SIZEOF_FIELD;
        let mut index = 0usize;
        let mut last_tag: i32 = -1;
        for (i, field) in self.sproto_type.fields.iter().enumerate() {
            let entry = match &self.entries[i] {
                Some(e) => e,
                None => continue,
            };
            let tag_gap = field.tag as i32 - last_tag - 1;
            if tag_gap > 0 {
                let skip = ((tag_gap - 1) * 2 + 1) as u16;
                let offset = self.output_base + SIZEOF_HEADER + SIZEOF_FIELD * index;
                write_u16_le(&mut self.output[offset..], skip);
                index += 1;
            }
            let offset = self.output_base + SIZEOF_HEADER + SIZEOF_FIELD * index;
            match entry {
                FieldEntry::Inline(v) => {
                    write_u16_le(&mut self.output[offset..], *v);
                }
                FieldEntry::Data { .. } => {
                    write_u16_le(&mut self.output[offset..], 0);
                }
            }
            index += 1;
            last_tag = field.tag as i32;
        }
        write_u16_le(&mut self.output[self.output_base..], index as u16);
        let used_header = SIZEOF_HEADER + index * SIZEOF_FIELD;
        let unused = header_sz - used_header;
        if unused > 0 {
            let data_start = self.output_base + header_sz;
            let data_end = self.output.len();
            if data_start < data_end {
                self.output
                    .copy_within(data_start..data_end, data_start - unused);
            }
            self.output.truncate(data_end - unused);
        }
        self.output
    }

    fn assemble_reorder(self) -> &'a mut Vec<u8> {
        let header_sz = SIZEOF_HEADER + self.sproto_type.maxn * SIZEOF_FIELD;
        let data_region_start = self.output_base + header_sz;
        let saved_data: Vec<u8> = self.output[data_region_start..].to_vec();
        self.output.truncate(data_region_start);
        let mut index = 0usize;
        let mut last_tag: i32 = -1;
        for (i, field) in self.sproto_type.fields.iter().enumerate() {
            let entry = match &self.entries[i] {
                Some(e) => e,
                None => continue,
            };
            let tag_gap = field.tag as i32 - last_tag - 1;
            if tag_gap > 0 {
                let skip = ((tag_gap - 1) * 2 + 1) as u16;
                let offset = self.output_base + SIZEOF_HEADER + SIZEOF_FIELD * index;
                write_u16_le(&mut self.output[offset..], skip);
                index += 1;
            }
            let offset = self.output_base + SIZEOF_HEADER + SIZEOF_FIELD * index;
            match entry {
                FieldEntry::Inline(v) => {
                    write_u16_le(&mut self.output[offset..], *v);
                }
                FieldEntry::Data { start, len } => {
                    write_u16_le(&mut self.output[offset..], 0);
                    let rel = *start - data_region_start;
                    self.output.extend_from_slice(&saved_data[rel..rel + *len]);
                }
            }
            index += 1;
            last_tag = field.tag as i32;
        }
        write_u16_le(&mut self.output[self.output_base..], index as u16);
        let used_header = SIZEOF_HEADER + index * SIZEOF_FIELD;
        let unused = header_sz - used_header;
        if unused > 0 {
            let data_start = self.output_base + header_sz;
            let data_end = self.output.len();
            if data_start < data_end {
                self.output
                    .copy_within(data_start..data_end, data_start - unused);
            }
            self.output.truncate(data_end - unused);
        }
        self.output
    }
}

struct TopLevelSerializer<'a> {
    sproto: &'a Sproto,
    sproto_type: &'a SprotoType,
    output: &'a mut Vec<u8>,
}

impl<'a> ser::Serializer for TopLevelSerializer<'a> {
    type Ok = ();
    type Error = SerdeError;
    type SerializeSeq = ser::Impossible<(), SerdeError>;
    type SerializeTuple = ser::Impossible<(), SerdeError>;
    type SerializeTupleStruct = ser::Impossible<(), SerdeError>;
    type SerializeTupleVariant = ser::Impossible<(), SerdeError>;
    type SerializeMap = ser::Impossible<(), SerdeError>;
    type SerializeStruct = DirectStructSerializer<'a>;
    type SerializeStructVariant = ser::Impossible<(), SerdeError>;

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(DirectStructSerializer(StructCore::new(
            self.sproto,
            self.sproto_type,
            self.output,
        )))
    }
    fn serialize_bool(self, _: bool) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_i8(self, _: i8) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_i16(self, _: i16) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_i32(self, _: i32) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_i64(self, _: i64) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_u8(self, _: u8) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_u16(self, _: u16) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_u32(self, _: u32) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_u64(self, _: u64) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_f32(self, _: f32) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_f64(self, _: f64) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_char(self, _: char) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_str(self, _: &str) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_none(self) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_unit(self) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), Self::Error> {
        Err(top_err())
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(top_err())
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(top_err())
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(top_err())
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(top_err())
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(top_err())
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(top_err())
    }
}

#[inline]
fn top_err() -> SerdeError {
    SerdeError::UnsupportedType("top-level value must be a struct".into())
}

struct DirectStructSerializer<'a>(StructCore<'a>);

impl<'a> ser::SerializeStruct for DirectStructSerializer<'a> {
    type Ok = ();
    type Error = SerdeError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.0.serialize_field_inner(key, value)
    }
    fn skip_field(&mut self, _key: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end(self) -> Result<(), Self::Error> {
        self.0.assemble();
        Ok(())
    }
}

struct NestedStructSerializer<'a> {
    core: StructCore<'a>,
    len_pos: usize,
}

impl<'a> ser::SerializeStruct for NestedStructSerializer<'a> {
    type Ok = FieldResult;
    type Error = SerdeError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.core.serialize_field_inner(key, value)
    }
    fn skip_field(&mut self, _key: &'static str) -> Result<(), Self::Error> {
        Ok(())
    }
    fn end(self) -> Result<FieldResult, Self::Error> {
        let len_pos = self.len_pos;
        let output = self.core.assemble();
        let data_start = len_pos + SIZEOF_LENGTH;
        let encoded_len = output.len() - data_start;
        write_u32_le(&mut output[len_pos..], encoded_len as u32);
        Ok(FieldResult::DataWritten)
    }
}

struct FieldValueSerializer<'a> {
    sproto: &'a Sproto,
    field: &'a Field,
    buf: &'a mut Vec<u8>,
}

impl<'a> FieldValueSerializer<'a> {
    fn encode_integer(self, int_val: i64) -> Result<FieldResult, SerdeError> {
        let uint_val = int_val as u64;
        let u32_val = uint_val as u32;
        if uint_val == u32_val as u64 && u32_val < 0x7fff {
            return Ok(FieldResult::Inline(((u32_val + 1) * 2) as u16));
        }
        if (int_val as i32) as i64 == int_val {
            let start = self.buf.len();
            self.buf.resize(start + SIZEOF_LENGTH + SIZEOF_INT32, 0);
            write_u32_le(&mut self.buf[start..], SIZEOF_INT32 as u32);
            write_u32_le(&mut self.buf[start + SIZEOF_LENGTH..], int_val as u32);
        } else {
            let start = self.buf.len();
            self.buf.resize(start + SIZEOF_LENGTH + SIZEOF_INT64, 0);
            write_u32_le(&mut self.buf[start..], SIZEOF_INT64 as u32);
            write_u64_le(&mut self.buf[start + SIZEOF_LENGTH..], uint_val);
        }
        Ok(FieldResult::DataWritten)
    }
    fn encode_double(self, dval: f64) -> Result<FieldResult, SerdeError> {
        let start = self.buf.len();
        self.buf.resize(start + SIZEOF_LENGTH + SIZEOF_INT64, 0);
        write_u32_le(&mut self.buf[start..], SIZEOF_INT64 as u32);
        write_u64_le(&mut self.buf[start + SIZEOF_LENGTH..], dval.to_bits());
        Ok(FieldResult::DataWritten)
    }
    fn encode_string_bytes(self, data: &[u8]) -> Result<FieldResult, SerdeError> {
        let start = self.buf.len();
        self.buf.reserve(SIZEOF_LENGTH + data.len());
        self.buf.resize(start + SIZEOF_LENGTH, 0);
        write_u32_le(&mut self.buf[start..], data.len() as u32);
        self.buf.extend_from_slice(data);
        Ok(FieldResult::DataWritten)
    }
}

impl<'a> ser::Serializer for FieldValueSerializer<'a> {
    type Ok = FieldResult;
    type Error = SerdeError;
    type SerializeSeq = DirectSeqSerializer<'a>;
    type SerializeTuple = DirectSeqSerializer<'a>;
    type SerializeTupleStruct = ser::Impossible<FieldResult, SerdeError>;
    type SerializeTupleVariant = ser::Impossible<FieldResult, SerdeError>;
    type SerializeMap = ser::Impossible<FieldResult, SerdeError>;
    type SerializeStruct = NestedStructSerializer<'a>;
    type SerializeStructVariant = ser::Impossible<FieldResult, SerdeError>;

    fn serialize_bool(self, v: bool) -> Result<FieldResult, SerdeError> {
        self.encode_integer(if v { 1 } else { 0 })
    }
    fn serialize_i8(self, v: i8) -> Result<FieldResult, SerdeError> {
        self.encode_integer(v as i64)
    }
    fn serialize_i16(self, v: i16) -> Result<FieldResult, SerdeError> {
        self.encode_integer(v as i64)
    }
    fn serialize_i32(self, v: i32) -> Result<FieldResult, SerdeError> {
        self.encode_integer(v as i64)
    }
    fn serialize_i64(self, v: i64) -> Result<FieldResult, SerdeError> {
        self.encode_integer(v)
    }
    fn serialize_u8(self, v: u8) -> Result<FieldResult, SerdeError> {
        self.encode_integer(v as i64)
    }
    fn serialize_u16(self, v: u16) -> Result<FieldResult, SerdeError> {
        self.encode_integer(v as i64)
    }
    fn serialize_u32(self, v: u32) -> Result<FieldResult, SerdeError> {
        self.encode_integer(v as i64)
    }
    fn serialize_u64(self, v: u64) -> Result<FieldResult, SerdeError> {
        self.encode_integer(v as i64)
    }
    fn serialize_f32(self, v: f32) -> Result<FieldResult, SerdeError> {
        self.serialize_f64(v as f64)
    }
    fn serialize_f64(self, v: f64) -> Result<FieldResult, SerdeError> {
        if self.field.decimal_precision > 0 {
            let scaled = (v * self.field.decimal_precision as f64).round() as i64;
            self.encode_integer(scaled)
        } else {
            self.encode_double(v)
        }
    }
    fn serialize_char(self, v: char) -> Result<FieldResult, SerdeError> {
        let mut tmp = [0u8; 4];
        self.encode_string_bytes(v.encode_utf8(&mut tmp).as_bytes())
    }
    fn serialize_str(self, v: &str) -> Result<FieldResult, SerdeError> {
        self.encode_string_bytes(v.as_bytes())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<FieldResult, SerdeError> {
        self.encode_string_bytes(v)
    }
    fn serialize_none(self) -> Result<FieldResult, SerdeError> {
        Ok(FieldResult::Skip)
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<FieldResult, SerdeError> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<FieldResult, SerdeError> {
        Err(fv_err())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<FieldResult, SerdeError> {
        Err(fv_err())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<FieldResult, SerdeError> {
        Err(fv_err())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        v: &T,
    ) -> Result<FieldResult, SerdeError> {
        v.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<FieldResult, SerdeError> {
        Err(fv_err())
    }
    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, SerdeError> {
        DirectSeqSerializer::new(self.sproto, self.field, self.buf, len)
    }
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, SerdeError> {
        self.serialize_seq(Some(len))
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, SerdeError> {
        Err(fv_err())
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, SerdeError> {
        Err(fv_err())
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, SerdeError> {
        Err(fv_err())
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, SerdeError> {
        let sub_type = match &self.field.field_type {
            FieldType::Struct(idx) => &self.sproto.types_list[*idx],
            _ => {
                return Err(SerdeError::TypeMismatch {
                    field: self.field.name.to_string(),
                    expected: "struct".into(),
                    actual: "non-struct schema type".into(),
                })
            }
        };
        let len_pos = self.buf.len();
        self.buf.resize(len_pos + SIZEOF_LENGTH, 0);
        Ok(NestedStructSerializer {
            core: StructCore::new(self.sproto, sub_type, self.buf),
            len_pos,
        })
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, SerdeError> {
        Err(fv_err())
    }
}

#[inline]
fn fv_err() -> SerdeError {
    SerdeError::UnsupportedType("unsupported type for sproto field".into())
}

enum DirectSeqSerializer<'a> {
    Integer(IntArrayCollector<'a>),
    Boolean(BoolArrayCollector<'a>),
    Object(ObjectArraySerializer<'a>),
}

impl<'a> DirectSeqSerializer<'a> {
    fn new(
        sproto: &'a Sproto,
        field: &'a Field,
        buf: &'a mut Vec<u8>,
        len: Option<usize>,
    ) -> Result<Self, SerdeError> {
        match &field.field_type {
            FieldType::Integer | FieldType::Double => {
                let is_double = field.field_type == FieldType::Double;
                Ok(DirectSeqSerializer::Integer(IntArrayCollector {
                    field,
                    buf,
                    values: Vec::with_capacity(len.unwrap_or(0)),
                    need_64bit: is_double,
                    is_double,
                }))
            }
            FieldType::Boolean => {
                let p = buf.len();
                buf.resize(p + SIZEOF_LENGTH, 0);
                Ok(DirectSeqSerializer::Boolean(BoolArrayCollector {
                    buf,
                    outer_len_pos: p,
                    count: 0,
                }))
            }
            FieldType::String | FieldType::Binary | FieldType::Struct(_) => {
                let p = buf.len();
                buf.resize(p + SIZEOF_LENGTH, 0);
                Ok(DirectSeqSerializer::Object(ObjectArraySerializer {
                    sproto,
                    field,
                    buf,
                    outer_len_pos: p,
                }))
            }
        }
    }
}

impl<'a> ser::SerializeSeq for DirectSeqSerializer<'a> {
    type Ok = FieldResult;
    type Error = SerdeError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        match self {
            DirectSeqSerializer::Integer(c) => c.serialize_element(value),
            DirectSeqSerializer::Boolean(c) => c.serialize_element(value),
            DirectSeqSerializer::Object(c) => c.serialize_element(value),
        }
    }
    fn end(self) -> Result<FieldResult, SerdeError> {
        match self {
            DirectSeqSerializer::Integer(c) => c.end(),
            DirectSeqSerializer::Boolean(c) => c.end(),
            DirectSeqSerializer::Object(c) => c.end(),
        }
    }
}

impl<'a> ser::SerializeTuple for DirectSeqSerializer<'a> {
    type Ok = FieldResult;
    type Error = SerdeError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        ser::SerializeSeq::serialize_element(self, value)
    }
    fn end(self) -> Result<FieldResult, SerdeError> {
        ser::SerializeSeq::end(self)
    }
}

struct IntArrayCollector<'a> {
    field: &'a Field,
    buf: &'a mut Vec<u8>,
    values: Vec<u64>,
    need_64bit: bool,
    is_double: bool,
}

impl<'a> IntArrayCollector<'a> {
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        let ex = NumericExtractor {
            is_double: self.is_double,
            decimal_precision: self.field.decimal_precision,
        };
        let (raw, n64) = value.serialize(ex)?;
        if n64 {
            self.need_64bit = true;
        }
        self.values.push(raw);
        Ok(())
    }
    fn end(self) -> Result<FieldResult, SerdeError> {
        if self.values.is_empty() {
            let s = self.buf.len();
            self.buf.resize(s + SIZEOF_LENGTH, 0);
            write_u32_le(&mut self.buf[s..], 0);
            return Ok(FieldResult::DataWritten);
        }
        let isz = if self.need_64bit {
            SIZEOF_INT64
        } else {
            SIZEOF_INT32
        };
        let dlen = 1 + self.values.len() * isz;
        let s = self.buf.len();
        self.buf.resize(s + SIZEOF_LENGTH + dlen, 0);
        write_u32_le(&mut self.buf[s..], dlen as u32);
        self.buf[s + SIZEOF_LENGTH] = isz as u8;
        let mut off = s + SIZEOF_LENGTH + 1;
        for &v in &self.values {
            if self.need_64bit {
                write_u64_le(&mut self.buf[off..], v);
                off += SIZEOF_INT64;
            } else {
                write_u32_le(&mut self.buf[off..], v as u32);
                off += SIZEOF_INT32;
            }
        }
        Ok(FieldResult::DataWritten)
    }
}

struct BoolArrayCollector<'a> {
    buf: &'a mut Vec<u8>,
    outer_len_pos: usize,
    count: usize,
}

impl<'a> BoolArrayCollector<'a> {
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        let b = value.serialize(BoolExtractor)?;
        self.buf.push(if b { 1 } else { 0 });
        self.count += 1;
        Ok(())
    }
    fn end(self) -> Result<FieldResult, SerdeError> {
        write_u32_le(&mut self.buf[self.outer_len_pos..], self.count as u32);
        Ok(FieldResult::DataWritten)
    }
}

struct ObjectArraySerializer<'a> {
    sproto: &'a Sproto,
    field: &'a Field,
    buf: &'a mut Vec<u8>,
    outer_len_pos: usize,
}

impl<'a> ObjectArraySerializer<'a> {
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), SerdeError> {
        let elp = self.buf.len();
        self.buf.resize(elp + SIZEOF_LENGTH, 0);
        let es = self.buf.len();
        let ser = ArrayElemSerializer {
            sproto: self.sproto,
            field: self.field,
            buf: &mut *self.buf,
        };
        value.serialize(ser)?;
        let elem_len = (self.buf.len() - es) as u32;
        write_u32_le(&mut self.buf[elp..], elem_len);
        Ok(())
    }
    fn end(self) -> Result<FieldResult, SerdeError> {
        let ds = self.outer_len_pos + SIZEOF_LENGTH;
        let outer_len = (self.buf.len() - ds) as u32;
        write_u32_le(&mut self.buf[self.outer_len_pos..], outer_len);
        Ok(FieldResult::DataWritten)
    }
}

struct ArrayElemSerializer<'a> {
    sproto: &'a Sproto,
    field: &'a Field,
    buf: &'a mut Vec<u8>,
}

impl<'a> ser::Serializer for ArrayElemSerializer<'a> {
    type Ok = ();
    type Error = SerdeError;
    type SerializeSeq = ser::Impossible<(), SerdeError>;
    type SerializeTuple = ser::Impossible<(), SerdeError>;
    type SerializeTupleStruct = ser::Impossible<(), SerdeError>;
    type SerializeTupleVariant = ser::Impossible<(), SerdeError>;
    type SerializeMap = ser::Impossible<(), SerdeError>;
    type SerializeStruct = DirectStructSerializer<'a>;
    type SerializeStructVariant = ser::Impossible<(), SerdeError>;

    fn serialize_str(self, v: &str) -> Result<(), SerdeError> {
        self.buf.extend_from_slice(v.as_bytes());
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<(), SerdeError> {
        self.buf.extend_from_slice(v);
        Ok(())
    }
    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, SerdeError> {
        let st = match &self.field.field_type {
            FieldType::Struct(i) => &self.sproto.types_list[*i],
            _ => {
                return Err(SerdeError::UnsupportedType(
                    "expected struct for array element".into(),
                ))
            }
        };
        Ok(DirectStructSerializer(StructCore::new(
            self.sproto,
            st,
            self.buf,
        )))
    }
    fn serialize_bool(self, _: bool) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_i8(self, _: i8) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_i16(self, _: i16) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_i32(self, _: i32) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_i64(self, _: i64) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_u8(self, _: u8) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_u16(self, _: u16) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_u32(self, _: u32) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_u64(self, _: u64) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_f32(self, _: f32) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_f64(self, _: f64) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_char(self, _: char) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_none(self) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_unit(self) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), SerdeError> {
        Err(elem_err())
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, SerdeError> {
        Err(elem_err())
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, SerdeError> {
        Err(elem_err())
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, SerdeError> {
        Err(elem_err())
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, SerdeError> {
        Err(elem_err())
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, SerdeError> {
        Err(elem_err())
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, SerdeError> {
        Err(elem_err())
    }
}

#[inline]
fn elem_err() -> SerdeError {
    SerdeError::UnsupportedType("unexpected type for array element".into())
}

struct NumericExtractor {
    is_double: bool,
    decimal_precision: u32,
}

impl ser::Serializer for NumericExtractor {
    type Ok = (u64, bool);
    type Error = SerdeError;
    type SerializeSeq = ser::Impossible<(u64, bool), SerdeError>;
    type SerializeTuple = ser::Impossible<(u64, bool), SerdeError>;
    type SerializeTupleStruct = ser::Impossible<(u64, bool), SerdeError>;
    type SerializeTupleVariant = ser::Impossible<(u64, bool), SerdeError>;
    type SerializeMap = ser::Impossible<(u64, bool), SerdeError>;
    type SerializeStruct = ser::Impossible<(u64, bool), SerdeError>;
    type SerializeStructVariant = ser::Impossible<(u64, bool), SerdeError>;

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok((v as u64, (v as i32) as i64 != v))
    }
    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(v as i64)
    }
    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(v as f64)
    }
    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        if self.is_double {
            Ok((v.to_bits(), true))
        } else if self.decimal_precision > 0 {
            let s = (v * self.decimal_precision as f64).round() as i64;
            Ok((s as u64, (s as i32) as i64 != s))
        } else {
            let i = v as i64;
            Ok((i as u64, (i as i32) as i64 != i))
        }
    }
    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok((if v { 1 } else { 0 }, false))
    }
    fn serialize_char(self, _: char) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_str(self, _: &str) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<Self::Ok, Self::Error> {
        Err(num_err())
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(num_err())
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(num_err())
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(num_err())
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(num_err())
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(num_err())
    }
    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Err(num_err())
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(num_err())
    }
}

#[inline]
fn num_err() -> SerdeError {
    SerdeError::UnsupportedType("expected numeric type for array".into())
}

struct BoolExtractor;

impl ser::Serializer for BoolExtractor {
    type Ok = bool;
    type Error = SerdeError;
    type SerializeSeq = ser::Impossible<bool, SerdeError>;
    type SerializeTuple = ser::Impossible<bool, SerdeError>;
    type SerializeTupleStruct = ser::Impossible<bool, SerdeError>;
    type SerializeTupleVariant = ser::Impossible<bool, SerdeError>;
    type SerializeMap = ser::Impossible<bool, SerdeError>;
    type SerializeStruct = ser::Impossible<bool, SerdeError>;
    type SerializeStructVariant = ser::Impossible<bool, SerdeError>;

    fn serialize_bool(self, v: bool) -> Result<bool, SerdeError> {
        Ok(v)
    }
    fn serialize_i8(self, _: i8) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_i16(self, _: i16) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_i32(self, _: i32) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_i64(self, _: i64) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_u8(self, _: u8) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_u16(self, _: u16) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_u32(self, _: u32) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_u64(self, _: u64) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_f32(self, _: f32) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_f64(self, _: f64) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_char(self, _: char) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_str(self, _: &str) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_none(self) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, _: &T) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_unit(self) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: &T,
    ) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<bool, SerdeError> {
        Err(bl_err())
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, SerdeError> {
        Err(bl_err())
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, SerdeError> {
        Err(bl_err())
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, SerdeError> {
        Err(bl_err())
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, SerdeError> {
        Err(bl_err())
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, SerdeError> {
        Err(bl_err())
    }
    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, SerdeError> {
        Err(bl_err())
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, SerdeError> {
        Err(bl_err())
    }
}

#[inline]
fn bl_err() -> SerdeError {
    SerdeError::UnsupportedType("expected boolean for boolean array".into())
}
