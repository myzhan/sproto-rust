//! Tag-based struct encoder for sproto wire format.
//!
//! `StructEncoder` is the core encoding engine shared by the Direct API and
//! the Serde adapter. It accepts field values by tag number and assembles
//! the sproto wire header + data section on `finish()`.

use crate::codec::wire::*;
use crate::error::EncodeError;
use crate::types::{FieldType, Sproto, SprotoType};

const MAX_STACK_FIELDS: usize = 32;

#[derive(Clone, Copy)]
pub(crate) enum FieldEntry {
    Inline(u16),
    Data { start: usize, len: usize },
}

/// Tag-based encoder for a single sproto struct.
///
/// Reserves header space on creation, accepts field values via `set_*` methods,
/// then assembles the final wire bytes on `finish()`.
pub struct StructEncoder<'a> {
    pub(crate) sproto: &'a Sproto,
    pub(crate) sproto_type: &'a SprotoType,
    pub(crate) output: &'a mut Vec<u8>,
    output_base: usize,
    stack_entries: [Option<FieldEntry>; MAX_STACK_FIELDS],
    heap_entries: Vec<Option<FieldEntry>>,
    use_heap: bool,
    in_order: bool,
    last_data_tag: i32,
}

impl<'a> StructEncoder<'a> {
    /// Create a new encoder for the given type, appending to `output`.
    pub fn new(sproto: &'a Sproto, sproto_type: &'a SprotoType, output: &'a mut Vec<u8>) -> Self {
        let output_base = output.len();
        let header_sz = SIZEOF_HEADER + sproto_type.maxn * SIZEOF_FIELD;
        output.resize(output_base + header_sz, 0);
        let nfields = sproto_type.fields.len();
        let use_heap = nfields > MAX_STACK_FIELDS;
        StructEncoder {
            sproto,
            sproto_type,
            output,
            output_base,
            stack_entries: [None; MAX_STACK_FIELDS],
            heap_entries: if use_heap {
                vec![None; nfields]
            } else {
                Vec::new()
            },
            use_heap,
            in_order: true,
            last_data_tag: -1,
        }
    }

    #[inline]
    fn resolve_tag(&self, tag: u16) -> Result<usize, EncodeError> {
        self.sproto_type.field_index_by_tag(tag).ok_or_else(|| {
            EncodeError::Other(format!(
                "unknown tag {} in type '{}'",
                tag, self.sproto_type.name
            ))
        })
    }

    #[inline]
    fn set_entry(&mut self, idx: usize, entry: FieldEntry) {
        if self.use_heap {
            self.heap_entries[idx] = Some(entry);
        } else {
            self.stack_entries[idx] = Some(entry);
        }
    }

    #[inline]
    fn track_data_order(&mut self, tag: u16) {
        let t = tag as i32;
        if t <= self.last_data_tag {
            self.in_order = false;
        }
        self.last_data_tag = t;
    }

    /// Encode an integer field.
    pub fn set_integer(&mut self, tag: u16, value: i64) -> Result<(), EncodeError> {
        let idx = self.resolve_tag(tag)?;
        let uint_val = value as u64;
        let u32_val = uint_val as u32;
        if uint_val == u32_val as u64 && u32_val < 0x7fff {
            self.set_entry(idx, FieldEntry::Inline(((u32_val + 1) * 2) as u16));
        } else if (value as i32) as i64 == value {
            let start = self.output.len();
            self.output.resize(start + SIZEOF_LENGTH + SIZEOF_INT32, 0);
            write_u32_le(&mut self.output[start..], SIZEOF_INT32 as u32);
            write_u32_le(&mut self.output[start + SIZEOF_LENGTH..], value as u32);
            self.set_entry(
                idx,
                FieldEntry::Data {
                    start,
                    len: SIZEOF_LENGTH + SIZEOF_INT32,
                },
            );
            self.track_data_order(tag);
        } else {
            let start = self.output.len();
            self.output.resize(start + SIZEOF_LENGTH + SIZEOF_INT64, 0);
            write_u32_le(&mut self.output[start..], SIZEOF_INT64 as u32);
            write_u64_le(&mut self.output[start + SIZEOF_LENGTH..], uint_val);
            self.set_entry(
                idx,
                FieldEntry::Data {
                    start,
                    len: SIZEOF_LENGTH + SIZEOF_INT64,
                },
            );
            self.track_data_order(tag);
        }
        Ok(())
    }

    /// Encode a boolean field (wire-encoded as integer 0/1).
    #[inline]
    pub fn set_bool(&mut self, tag: u16, value: bool) -> Result<(), EncodeError> {
        self.set_integer(tag, i64::from(value))
    }

    /// Encode a double (f64) field.
    pub fn set_double(&mut self, tag: u16, value: f64) -> Result<(), EncodeError> {
        let idx = self.resolve_tag(tag)?;
        let start = self.output.len();
        self.output.resize(start + SIZEOF_LENGTH + SIZEOF_INT64, 0);
        write_u32_le(&mut self.output[start..], SIZEOF_INT64 as u32);
        write_u64_le(&mut self.output[start + SIZEOF_LENGTH..], value.to_bits());
        self.set_entry(
            idx,
            FieldEntry::Data {
                start,
                len: SIZEOF_LENGTH + SIZEOF_INT64,
            },
        );
        self.track_data_order(tag);
        Ok(())
    }

    /// Encode a string field.
    #[inline]
    pub fn set_string(&mut self, tag: u16, value: &str) -> Result<(), EncodeError> {
        self.set_raw_bytes(tag, value.as_bytes())
    }

    /// Encode a binary (bytes) field.
    #[inline]
    pub fn set_bytes(&mut self, tag: u16, value: &[u8]) -> Result<(), EncodeError> {
        self.set_raw_bytes(tag, value)
    }

    fn set_raw_bytes(&mut self, tag: u16, data: &[u8]) -> Result<(), EncodeError> {
        let idx = self.resolve_tag(tag)?;
        let start = self.output.len();
        self.output.reserve(SIZEOF_LENGTH + data.len());
        self.output.resize(start + SIZEOF_LENGTH, 0);
        write_u32_le(&mut self.output[start..], data.len() as u32);
        self.output.extend_from_slice(data);
        let len = SIZEOF_LENGTH + data.len();
        self.set_entry(idx, FieldEntry::Data { start, len });
        self.track_data_order(tag);
        Ok(())
    }

    /// Encode a nested struct field using a closure.
    ///
    /// The closure receives a `StructEncoder` for the sub-type. The sub-encoder
    /// is automatically finished when the closure returns.
    pub fn encode_nested<F>(&mut self, tag: u16, f: F) -> Result<(), EncodeError>
    where
        F: FnOnce(&mut StructEncoder) -> Result<(), EncodeError>,
    {
        let idx = self.resolve_tag(tag)?;
        let field = &self.sproto_type.fields[idx];
        let sub_type_idx = match &field.field_type {
            FieldType::Struct(i) => *i,
            _ => {
                return Err(EncodeError::TypeMismatch {
                    field: field.name.to_string(),
                    expected: "struct".into(),
                    actual: "non-struct".into(),
                })
            }
        };
        let sub_type = &self.sproto.types_list[sub_type_idx];
        let data_start = self.output.len();
        self.output.resize(data_start + SIZEOF_LENGTH, 0);
        {
            let mut sub = StructEncoder::new(self.sproto, sub_type, &mut *self.output);
            f(&mut sub)?;
            sub.finish();
        }
        let encoded_len = self.output.len() - data_start - SIZEOF_LENGTH;
        write_u32_le(&mut self.output[data_start..], encoded_len as u32);
        let data_len = self.output.len() - data_start;
        self.set_entry(
            idx,
            FieldEntry::Data {
                start: data_start,
                len: data_len,
            },
        );
        self.track_data_order(tag);
        Ok(())
    }

    /// Encode an integer array field.
    pub fn set_integer_array(&mut self, tag: u16, values: &[i64]) -> Result<(), EncodeError> {
        let idx = self.resolve_tag(tag)?;
        let start = self.output.len();
        if values.is_empty() {
            self.output.resize(start + SIZEOF_LENGTH, 0);
            write_u32_le(&mut self.output[start..], 0);
        } else {
            let need_64 = values.iter().any(|&v| (v as i32) as i64 != v);
            let isz = if need_64 { SIZEOF_INT64 } else { SIZEOF_INT32 };
            let dlen = 1 + values.len() * isz;
            self.output.resize(start + SIZEOF_LENGTH + dlen, 0);
            write_u32_le(&mut self.output[start..], dlen as u32);
            self.output[start + SIZEOF_LENGTH] = isz as u8;
            let mut off = start + SIZEOF_LENGTH + 1;
            for &v in values {
                if need_64 {
                    write_u64_le(&mut self.output[off..], v as u64);
                    off += SIZEOF_INT64;
                } else {
                    write_u32_le(&mut self.output[off..], v as u32);
                    off += SIZEOF_INT32;
                }
            }
        }
        let len = self.output.len() - start;
        self.set_entry(idx, FieldEntry::Data { start, len });
        self.track_data_order(tag);
        Ok(())
    }

    /// Encode a boolean array field.
    pub fn set_bool_array(&mut self, tag: u16, values: &[bool]) -> Result<(), EncodeError> {
        let idx = self.resolve_tag(tag)?;
        let start = self.output.len();
        self.output.resize(start + SIZEOF_LENGTH + values.len(), 0);
        write_u32_le(&mut self.output[start..], values.len() as u32);
        for (i, &v) in values.iter().enumerate() {
            self.output[start + SIZEOF_LENGTH + i] = u8::from(v);
        }
        let len = self.output.len() - start;
        self.set_entry(idx, FieldEntry::Data { start, len });
        self.track_data_order(tag);
        Ok(())
    }

    /// Encode a double array field.
    pub fn set_double_array(&mut self, tag: u16, values: &[f64]) -> Result<(), EncodeError> {
        let idx = self.resolve_tag(tag)?;
        let start = self.output.len();
        if values.is_empty() {
            self.output.resize(start + SIZEOF_LENGTH, 0);
            write_u32_le(&mut self.output[start..], 0);
        } else {
            let dlen = 1 + values.len() * SIZEOF_INT64;
            self.output.resize(start + SIZEOF_LENGTH + dlen, 0);
            write_u32_le(&mut self.output[start..], dlen as u32);
            self.output[start + SIZEOF_LENGTH] = SIZEOF_INT64 as u8;
            let mut off = start + SIZEOF_LENGTH + 1;
            for &v in values {
                write_u64_le(&mut self.output[off..], v.to_bits());
                off += SIZEOF_INT64;
            }
        }
        let len = self.output.len() - start;
        self.set_entry(idx, FieldEntry::Data { start, len });
        self.track_data_order(tag);
        Ok(())
    }

    /// Encode a string array field.
    pub fn set_string_array<S: AsRef<str>>(
        &mut self,
        tag: u16,
        values: &[S],
    ) -> Result<(), EncodeError> {
        self.set_object_array(tag, values.iter().map(|s| s.as_ref().as_bytes()))
    }

    /// Encode a binary array field.
    pub fn set_bytes_array<B: AsRef<[u8]>>(
        &mut self,
        tag: u16,
        values: &[B],
    ) -> Result<(), EncodeError> {
        self.set_object_array(tag, values.iter().map(|b| b.as_ref()))
    }

    fn set_object_array<'b>(
        &mut self,
        tag: u16,
        values: impl Iterator<Item = &'b [u8]>,
    ) -> Result<(), EncodeError> {
        let idx = self.resolve_tag(tag)?;
        let outer_start = self.output.len();
        self.output.resize(outer_start + SIZEOF_LENGTH, 0);
        for data in values {
            let elem_start = self.output.len();
            self.output.resize(elem_start + SIZEOF_LENGTH, 0);
            write_u32_le(&mut self.output[elem_start..], data.len() as u32);
            self.output.extend_from_slice(data);
        }
        let outer_len = self.output.len() - outer_start - SIZEOF_LENGTH;
        write_u32_le(&mut self.output[outer_start..], outer_len as u32);
        let len = self.output.len() - outer_start;
        self.set_entry(
            idx,
            FieldEntry::Data {
                start: outer_start,
                len,
            },
        );
        self.track_data_order(tag);
        Ok(())
    }

    /// Encode a struct array field using a closure.
    ///
    /// The closure receives a `StructArrayEncoder` to which elements can be added.
    pub fn encode_struct_array<F>(&mut self, tag: u16, f: F) -> Result<(), EncodeError>
    where
        F: FnOnce(&mut StructArrayEncoder) -> Result<(), EncodeError>,
    {
        let idx = self.resolve_tag(tag)?;
        let field = &self.sproto_type.fields[idx];
        let sub_type_idx = match &field.field_type {
            FieldType::Struct(i) => *i,
            _ => {
                return Err(EncodeError::TypeMismatch {
                    field: field.name.to_string(),
                    expected: "struct array".into(),
                    actual: "non-struct".into(),
                })
            }
        };
        let sub_type = &self.sproto.types_list[sub_type_idx];
        let outer_start = self.output.len();
        self.output.resize(outer_start + SIZEOF_LENGTH, 0);
        {
            let mut arr = StructArrayEncoder {
                sproto: self.sproto,
                sub_type,
                output: &mut *self.output,
            };
            f(&mut arr)?;
        }
        let outer_len = self.output.len() - outer_start - SIZEOF_LENGTH;
        write_u32_le(&mut self.output[outer_start..], outer_len as u32);
        let len = self.output.len() - outer_start;
        self.set_entry(
            idx,
            FieldEntry::Data {
                start: outer_start,
                len,
            },
        );
        self.track_data_order(tag);
        Ok(())
    }

    /// Record a field entry by index and tag.
    ///
    /// Combines `set_entry` + `track_data_order` for data entries.
    /// Used by the serde adapter to feed results back into the encoder.
    #[cfg(feature = "serde")]
    pub(crate) fn record_field(&mut self, idx: usize, tag: u16, entry: FieldEntry) {
        if matches!(entry, FieldEntry::Data { .. }) {
            self.track_data_order(tag);
        }
        self.set_entry(idx, entry);
    }

    /// Assemble the wire header + data and finalize the encoded bytes.
    ///
    /// Returns the output buffer so callers (e.g. nested struct serializers)
    /// can append length prefixes after assembly.
    pub fn finish(self) -> &'a mut Vec<u8> {
        if self.in_order {
            self.assemble_inorder()
        } else {
            self.assemble_reorder()
        }
    }

    #[inline]
    fn assemble_inorder(self) -> &'a mut Vec<u8> {
        let header_sz = SIZEOF_HEADER + self.sproto_type.maxn * SIZEOF_FIELD;
        let use_heap = self.use_heap;
        let mut index = 0usize;
        let mut last_tag: i32 = -1;
        for (i, field) in self.sproto_type.fields.iter().enumerate() {
            let entry = if use_heap {
                self.heap_entries[i]
            } else {
                self.stack_entries[i]
            };
            let entry = match entry {
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
                    write_u16_le(&mut self.output[offset..], v);
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
        let use_heap = self.use_heap;
        let mut index = 0usize;
        let mut last_tag: i32 = -1;
        for (i, field) in self.sproto_type.fields.iter().enumerate() {
            let entry = if use_heap {
                self.heap_entries[i]
            } else {
                self.stack_entries[i]
            };
            let entry = match entry {
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
                    write_u16_le(&mut self.output[offset..], v);
                }
                FieldEntry::Data { start, len } => {
                    write_u16_le(&mut self.output[offset..], 0);
                    let rel = start - data_region_start;
                    self.output.extend_from_slice(&saved_data[rel..rel + len]);
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

/// Encoder for struct array elements.
///
/// Each call to `encode_element` appends one length-prefixed encoded struct.
pub struct StructArrayEncoder<'a> {
    sproto: &'a Sproto,
    sub_type: &'a SprotoType,
    output: &'a mut Vec<u8>,
}

impl<'a> StructArrayEncoder<'a> {
    /// Encode one array element using a closure.
    pub fn encode_element<F>(&mut self, f: F) -> Result<(), EncodeError>
    where
        F: FnOnce(&mut StructEncoder) -> Result<(), EncodeError>,
    {
        let elem_start = self.output.len();
        self.output.resize(elem_start + SIZEOF_LENGTH, 0);
        {
            let mut enc = StructEncoder::new(self.sproto, self.sub_type, &mut *self.output);
            f(&mut enc)?;
            enc.finish();
        }
        let elem_len = self.output.len() - elem_start - SIZEOF_LENGTH;
        write_u32_le(&mut self.output[elem_start..], elem_len as u32);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn test_schema() -> Sproto {
        parser::parse(
            r#"
            .Person {
                name 0 : string
                age 1 : integer
                active 2 : boolean
            }
            .Data {
                numbers 0 : *integer
                value 1 : double
                flags 2 : *boolean
            }
            .Team {
                name 0 : string
                members 1 : *Person
            }
            .Nested {
                person 0 : Person
                count 1 : integer
            }
        "#,
        )
        .unwrap()
    }

    #[test]
    fn test_encode_primitives() {
        let schema = test_schema();
        let st = schema.get_type("Person").unwrap();
        let mut buf = Vec::new();
        let mut enc = StructEncoder::new(&schema, st, &mut buf);
        enc.set_string(0, "Alice").unwrap();
        enc.set_integer(1, 30).unwrap();
        enc.set_bool(2, true).unwrap();
        enc.finish();
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_encode_nested() {
        let schema = test_schema();
        let st = schema.get_type("Nested").unwrap();
        let mut buf = Vec::new();
        let mut enc = StructEncoder::new(&schema, st, &mut buf);
        enc.encode_nested(0, |child| {
            child.set_string(0, "Bob")?;
            child.set_integer(1, 25)?;
            child.set_bool(2, false)?;
            Ok(())
        })
        .unwrap();
        enc.set_integer(1, 42).unwrap();
        enc.finish();
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_encode_integer_array() {
        let schema = test_schema();
        let st = schema.get_type("Data").unwrap();
        let mut buf = Vec::new();
        let mut enc = StructEncoder::new(&schema, st, &mut buf);
        enc.set_integer_array(0, &[1, 2, 3, 4, 5]).unwrap();
        enc.set_double(1, 3.15).unwrap();
        enc.finish();
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_encode_struct_array() {
        let schema = test_schema();
        let st = schema.get_type("Team").unwrap();
        let mut buf = Vec::new();
        let mut enc = StructEncoder::new(&schema, st, &mut buf);
        enc.set_string(0, "TeamA").unwrap();
        enc.encode_struct_array(1, |arr| {
            arr.encode_element(|e| {
                e.set_string(0, "Alice")?;
                e.set_integer(1, 30)?;
                e.set_bool(2, true)?;
                Ok(())
            })?;
            arr.encode_element(|e| {
                e.set_string(0, "Bob")?;
                e.set_integer(1, 25)?;
                e.set_bool(2, false)?;
                Ok(())
            })?;
            Ok(())
        })
        .unwrap();
        enc.finish();
        assert!(!buf.is_empty());
    }
}
