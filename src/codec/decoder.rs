//! Tag-based struct decoder for sproto wire format.
//!
//! `StructDecoder` lazily iterates the wire header, yielding `DecodedField`
//! values with typed accessors. It is the core decoding engine shared by the
//! Direct API and the Serde adapter.

use crate::codec::wire::*;
use crate::error::DecodeError;
use crate::types::{Field, FieldType, Sproto, SprotoType};

/// Lazy wire-header decoder for a single sproto struct.
///
/// Call `next_field()` repeatedly to iterate over encoded fields.
pub struct StructDecoder<'a> {
    sproto: &'a Sproto,
    sproto_type: &'a SprotoType,
    data: &'a [u8],
    fn_count: usize,
    header_idx: usize,
    tag: i32,
    data_offset: usize,
}

impl<'a> StructDecoder<'a> {
    /// Create a decoder for the given wire bytes.
    pub fn new(
        sproto: &'a Sproto,
        sproto_type: &'a SprotoType,
        data: &'a [u8],
    ) -> Result<Self, DecodeError> {
        let size = data.len();
        if size < SIZEOF_HEADER {
            return Err(DecodeError::Truncated {
                need: SIZEOF_HEADER,
                have: size,
            });
        }
        let fn_count = read_u16_le(data) as usize;
        let field_part_end = SIZEOF_HEADER + fn_count * SIZEOF_FIELD;
        if size < field_part_end {
            return Err(DecodeError::Truncated {
                need: field_part_end,
                have: size,
            });
        }
        Ok(StructDecoder {
            sproto,
            sproto_type,
            data,
            fn_count,
            header_idx: 0,
            tag: -1,
            data_offset: field_part_end,
        })
    }

    /// Yield the next decoded field, or `None` when all fields are consumed.
    pub fn next_field(&mut self) -> Result<Option<DecodedField<'a>>, DecodeError> {
        let size = self.data.len();
        loop {
            if self.header_idx >= self.fn_count {
                return Ok(None);
            }
            let off = SIZEOF_HEADER + self.header_idx * SIZEOF_FIELD;
            let value = read_u16_le(&self.data[off..]) as i32;
            self.header_idx += 1;
            self.tag += 1;

            if value & 1 != 0 {
                self.tag += value / 2;
                continue;
            }

            let decoded_value = value / 2 - 1;
            let field_data = if decoded_value < 0 {
                if self.data_offset + SIZEOF_LENGTH > size {
                    return Err(DecodeError::Truncated {
                        need: self.data_offset + SIZEOF_LENGTH,
                        have: size,
                    });
                }
                let dsz = read_u32_le(&self.data[self.data_offset..]) as usize;
                if self.data_offset + SIZEOF_LENGTH + dsz > size {
                    return Err(DecodeError::Truncated {
                        need: self.data_offset + SIZEOF_LENGTH + dsz,
                        have: size,
                    });
                }
                let d = &self.data
                    [self.data_offset + SIZEOF_LENGTH..self.data_offset + SIZEOF_LENGTH + dsz];
                self.data_offset += SIZEOF_LENGTH + dsz;
                d
            } else {
                &[]
            };

            let field = match self.sproto_type.find_field_by_tag(self.tag as u16) {
                Some(f) => f,
                None => continue,
            };

            return Ok(Some(DecodedField {
                sproto: self.sproto,
                field,
                inline_value: decoded_value,
                data: field_data,
            }));
        }
    }
}

/// A single decoded field from the wire format.
///
/// Use the typed accessor methods (`as_integer`, `as_string`, etc.) to
/// extract the value in the expected type.
pub struct DecodedField<'a> {
    sproto: &'a Sproto,
    field: &'a Field,
    inline_value: i32,
    data: &'a [u8],
}

impl<'a> DecodedField<'a> {
    /// The tag number of this field.
    #[inline]
    pub fn tag(&self) -> u16 {
        self.field.tag
    }

    /// The schema field definition.
    #[inline]
    pub fn field(&self) -> &'a Field {
        self.field
    }

    /// The raw inline value from the wire header (-1 if data section).
    #[inline]
    pub fn inline_value(&self) -> i32 {
        self.inline_value
    }

    /// The raw data bytes (empty if value is inline).
    #[inline]
    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Decode as an integer value.
    pub fn as_integer(&self) -> Result<i64, DecodeError> {
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
            Err(DecodeError::InvalidData(format!(
                "invalid integer size {} in field '{}'",
                d.len(),
                self.field.name
            )))
        }
    }

    /// Decode as a boolean value.
    pub fn as_bool(&self) -> Result<bool, DecodeError> {
        if self.inline_value >= 0 {
            Ok(self.inline_value != 0)
        } else {
            Ok(self.as_integer()? != 0)
        }
    }

    /// Decode as a double (f64) value.
    pub fn as_double(&self) -> Result<f64, DecodeError> {
        let d = self.data;
        if d.len() == SIZEOF_INT64 {
            let lo = read_u32_le(d) as u64;
            let hi = read_u32_le(&d[SIZEOF_INT32..]) as u64;
            Ok(f64::from_bits(lo | (hi << 32)))
        } else {
            Err(DecodeError::InvalidData(format!(
                "invalid double size {} in field '{}'",
                d.len(),
                self.field.name
            )))
        }
    }

    /// Decode as a UTF-8 string.
    pub fn as_string(&self) -> Result<&'a str, DecodeError> {
        std::str::from_utf8(self.data).map_err(|e| {
            DecodeError::InvalidData(format!(
                "invalid utf-8 in field '{}': {}",
                self.field.name, e
            ))
        })
    }

    /// Return the raw bytes of the data section.
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.data
    }

    /// Decode as a nested struct, returning a sub-decoder.
    pub fn as_struct(&self) -> Result<StructDecoder<'a>, DecodeError> {
        match &self.field.field_type {
            FieldType::Struct(idx) => {
                let sub_type = &self.sproto.types_list[*idx];
                StructDecoder::new(self.sproto, sub_type, self.data)
            }
            _ => Err(DecodeError::InvalidData(format!(
                "field '{}' is not a struct type",
                self.field.name
            ))),
        }
    }

    /// Decode as an integer array.
    pub fn as_integer_array(&self) -> Result<Vec<i64>, DecodeError> {
        let d = self.data;
        if d.is_empty() {
            return Ok(Vec::new());
        }
        let int_len = d[0] as usize;
        if int_len != SIZEOF_INT32 && int_len != SIZEOF_INT64 {
            return Err(DecodeError::InvalidData(format!(
                "invalid integer element size {} in field '{}'",
                int_len, self.field.name
            )));
        }
        let vals = &d[1..];
        let count = vals.len() / int_len;
        let mut result = Vec::with_capacity(count);
        let mut off = 0;
        for _ in 0..count {
            let v = if int_len == SIZEOF_INT32 {
                expand64(read_u32_le(&vals[off..])) as i64
            } else {
                let lo = read_u32_le(&vals[off..]) as u64;
                let hi = read_u32_le(&vals[off + SIZEOF_INT32..]) as u64;
                (lo | (hi << 32)) as i64
            };
            result.push(v);
            off += int_len;
        }
        Ok(result)
    }

    /// Decode as a boolean array.
    pub fn as_bool_array(&self) -> Vec<bool> {
        self.data.iter().map(|&b| b != 0).collect()
    }

    /// Decode as a double array.
    pub fn as_double_array(&self) -> Result<Vec<f64>, DecodeError> {
        let d = self.data;
        if d.is_empty() {
            return Ok(Vec::new());
        }
        let int_len = d[0] as usize;
        if int_len != SIZEOF_INT32 && int_len != SIZEOF_INT64 {
            return Err(DecodeError::InvalidData(format!(
                "invalid double element size {} in field '{}'",
                int_len, self.field.name
            )));
        }
        let vals = &d[1..];
        let count = vals.len() / int_len;
        let mut result = Vec::with_capacity(count);
        let mut off = 0;
        for _ in 0..count {
            let raw = if int_len == SIZEOF_INT32 {
                expand64(read_u32_le(&vals[off..]))
            } else {
                let lo = read_u32_le(&vals[off..]) as u64;
                let hi = read_u32_le(&vals[off + SIZEOF_INT32..]) as u64;
                lo | (hi << 32)
            };
            result.push(f64::from_bits(raw));
            off += int_len;
        }
        Ok(result)
    }

    /// Decode as a string array.
    pub fn as_string_array(&self) -> Result<Vec<&'a str>, DecodeError> {
        let mut result = Vec::new();
        let mut off = 0;
        while off < self.data.len() {
            if off + SIZEOF_LENGTH > self.data.len() {
                return Err(DecodeError::Truncated {
                    need: off + SIZEOF_LENGTH,
                    have: self.data.len(),
                });
            }
            let esz = read_u32_le(&self.data[off..]) as usize;
            let start = off + SIZEOF_LENGTH;
            if start + esz > self.data.len() {
                return Err(DecodeError::Truncated {
                    need: start + esz,
                    have: self.data.len(),
                });
            }
            let s = std::str::from_utf8(&self.data[start..start + esz]).map_err(|e| {
                DecodeError::InvalidData(format!(
                    "invalid utf-8 in array field '{}': {}",
                    self.field.name, e
                ))
            })?;
            result.push(s);
            off = start + esz;
        }
        Ok(result)
    }

    /// Decode as a bytes array.
    pub fn as_bytes_array(&self) -> Result<Vec<&'a [u8]>, DecodeError> {
        let mut result = Vec::new();
        let mut off = 0;
        while off < self.data.len() {
            if off + SIZEOF_LENGTH > self.data.len() {
                return Err(DecodeError::Truncated {
                    need: off + SIZEOF_LENGTH,
                    have: self.data.len(),
                });
            }
            let esz = read_u32_le(&self.data[off..]) as usize;
            let start = off + SIZEOF_LENGTH;
            if start + esz > self.data.len() {
                return Err(DecodeError::Truncated {
                    need: start + esz,
                    have: self.data.len(),
                });
            }
            result.push(&self.data[start..start + esz]);
            off = start + esz;
        }
        Ok(result)
    }

    /// Decode as a struct array iterator.
    pub fn as_struct_iter(&self) -> Result<StructArrayIter<'a>, DecodeError> {
        match &self.field.field_type {
            FieldType::Struct(idx) => {
                let sub_type = &self.sproto.types_list[*idx];
                Ok(StructArrayIter {
                    sproto: self.sproto,
                    sub_type,
                    data: self.data,
                    offset: 0,
                })
            }
            _ => Err(DecodeError::InvalidData(format!(
                "field '{}' is not a struct array",
                self.field.name
            ))),
        }
    }
}

/// Iterator over elements of a struct array.
pub struct StructArrayIter<'a> {
    sproto: &'a Sproto,
    sub_type: &'a SprotoType,
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for StructArrayIter<'a> {
    type Item = Result<StructDecoder<'a>, DecodeError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }
        if self.offset + SIZEOF_LENGTH > self.data.len() {
            return Some(Err(DecodeError::Truncated {
                need: self.offset + SIZEOF_LENGTH,
                have: self.data.len(),
            }));
        }
        let esz = read_u32_le(&self.data[self.offset..]) as usize;
        let start = self.offset + SIZEOF_LENGTH;
        if start + esz > self.data.len() {
            return Some(Err(DecodeError::Truncated {
                need: start + esz,
                have: self.data.len(),
            }));
        }
        let elem_data = &self.data[start..start + esz];
        self.offset = start + esz;
        Some(StructDecoder::new(self.sproto, self.sub_type, elem_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::encoder::StructEncoder;
    use crate::types::{Field, FieldType};

    fn test_schema() -> Sproto {
        let mut s = Sproto::new();
        let person_idx = s.add_type(
            "Person",
            vec![
                Field::new("name", 0, FieldType::String),
                Field::new("age", 1, FieldType::Integer),
                Field::new("active", 2, FieldType::Boolean),
            ],
        );
        s.add_type(
            "Data",
            vec![
                Field::array("numbers", 0, FieldType::Integer),
                Field::new("value", 1, FieldType::Double),
                Field::array("flags", 2, FieldType::Boolean),
            ],
        );
        s.add_type(
            "Team",
            vec![
                Field::new("name", 0, FieldType::String),
                Field::array("members", 1, FieldType::Struct(person_idx)),
            ],
        );
        s.add_type(
            "Nested",
            vec![
                Field::new("person", 0, FieldType::Struct(person_idx)),
                Field::new("count", 1, FieldType::Integer),
            ],
        );
        s
    }

    #[test]
    fn test_roundtrip_primitives() {
        let schema = test_schema();
        let st = schema.get_type("Person").unwrap();
        let mut buf = Vec::new();
        {
            let mut enc = StructEncoder::new(&schema, st, &mut buf);
            enc.set_string(0, "Alice").unwrap();
            enc.set_integer(1, 30).unwrap();
            enc.set_bool(2, true).unwrap();
            enc.finish();
        }
        let mut dec = StructDecoder::new(&schema, st, &buf).unwrap();
        let mut name = None;
        let mut age = None;
        let mut active = None;
        while let Some(f) = dec.next_field().unwrap() {
            match f.tag() {
                0 => name = Some(f.as_string().unwrap().to_owned()),
                1 => age = Some(f.as_integer().unwrap()),
                2 => active = Some(f.as_bool().unwrap()),
                _ => {}
            }
        }
        assert_eq!(name.as_deref(), Some("Alice"));
        assert_eq!(age, Some(30));
        assert_eq!(active, Some(true));
    }

    #[test]
    fn test_roundtrip_nested() {
        let schema = test_schema();
        let st = schema.get_type("Nested").unwrap();
        let mut buf = Vec::new();
        {
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
        }
        let mut dec = StructDecoder::new(&schema, st, &buf).unwrap();
        let mut count = None;
        let mut person_name = None;
        while let Some(f) = dec.next_field().unwrap() {
            match f.tag() {
                0 => {
                    let mut sub = f.as_struct().unwrap();
                    while let Some(sf) = sub.next_field().unwrap() {
                        if sf.tag() == 0 {
                            person_name = Some(sf.as_string().unwrap().to_owned());
                        }
                    }
                }
                1 => count = Some(f.as_integer().unwrap()),
                _ => {}
            }
        }
        assert_eq!(person_name.as_deref(), Some("Bob"));
        assert_eq!(count, Some(42));
    }

    #[test]
    fn test_roundtrip_arrays() {
        let schema = test_schema();
        let st = schema.get_type("Data").unwrap();
        let mut buf = Vec::new();
        {
            let mut enc = StructEncoder::new(&schema, st, &mut buf);
            enc.set_integer_array(0, &[1, 2, 3, 4, 5]).unwrap();
            enc.set_double(1, 3.15).unwrap();
            enc.set_bool_array(2, &[true, false, true]).unwrap();
            enc.finish();
        }
        let mut dec = StructDecoder::new(&schema, st, &buf).unwrap();
        let mut numbers = None;
        let mut value = None;
        let mut flags = None;
        while let Some(f) = dec.next_field().unwrap() {
            match f.tag() {
                0 => numbers = Some(f.as_integer_array().unwrap()),
                1 => value = Some(f.as_double().unwrap()),
                2 => flags = Some(f.as_bool_array()),
                _ => {}
            }
        }
        assert_eq!(numbers, Some(vec![1, 2, 3, 4, 5]));
        assert!((value.unwrap() - 3.15).abs() < 1e-10);
        assert_eq!(flags, Some(vec![true, false, true]));
    }

    #[test]
    fn test_roundtrip_struct_array() {
        let schema = test_schema();
        let st = schema.get_type("Team").unwrap();
        let mut buf = Vec::new();
        {
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
        }
        let mut dec = StructDecoder::new(&schema, st, &buf).unwrap();
        let mut team_name = None;
        let mut member_names = Vec::new();
        while let Some(f) = dec.next_field().unwrap() {
            match f.tag() {
                0 => team_name = Some(f.as_string().unwrap().to_owned()),
                1 => {
                    for elem in f.as_struct_iter().unwrap() {
                        let mut sub = elem.unwrap();
                        while let Some(sf) = sub.next_field().unwrap() {
                            if sf.tag() == 0 {
                                member_names.push(sf.as_string().unwrap().to_owned());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        assert_eq!(team_name.as_deref(), Some("TeamA"));
        assert_eq!(member_names, vec!["Alice", "Bob"]);
    }

    #[test]
    fn test_roundtrip_large_integer() {
        let schema = test_schema();
        let st = schema.get_type("Person").unwrap();
        let mut buf = Vec::new();
        {
            let mut enc = StructEncoder::new(&schema, st, &mut buf);
            enc.set_string(0, "Test").unwrap();
            enc.set_integer(1, i64::MAX).unwrap();
            enc.set_bool(2, false).unwrap();
            enc.finish();
        }
        let mut dec = StructDecoder::new(&schema, st, &buf).unwrap();
        while let Some(f) = dec.next_field().unwrap() {
            if f.tag() == 1 {
                assert_eq!(f.as_integer().unwrap(), i64::MAX);
            }
        }
    }

    #[test]
    fn test_roundtrip_negative_integer() {
        let schema = test_schema();
        let st = schema.get_type("Person").unwrap();
        let mut buf = Vec::new();
        {
            let mut enc = StructEncoder::new(&schema, st, &mut buf);
            enc.set_string(0, "Test").unwrap();
            enc.set_integer(1, -100).unwrap();
            enc.set_bool(2, true).unwrap();
            enc.finish();
        }
        let mut dec = StructDecoder::new(&schema, st, &buf).unwrap();
        while let Some(f) = dec.next_field().unwrap() {
            if f.tag() == 1 {
                assert_eq!(f.as_integer().unwrap(), -100);
            }
        }
    }
}
