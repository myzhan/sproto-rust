use crate::error::EncodeError;
use crate::types::{Field, FieldType, Sproto, SprotoType};
use crate::value::SprotoValue;

use super::wire::*;

/// Encode a `SprotoValue::Struct` according to a `SprotoType` schema.
///
/// Returns the encoded binary data matching the sproto wire protocol.
pub fn encode(
    sproto: &Sproto,
    sproto_type: &SprotoType,
    value: &SprotoValue,
) -> Result<Vec<u8>, EncodeError> {
    let fields_map = match value {
        SprotoValue::Struct(map) => map,
        _ => {
            return Err(EncodeError::TypeMismatch {
                field: sproto_type.name.clone(),
                expected: "struct".into(),
                actual: value.type_name().into(),
            });
        }
    };

    // Pre-allocate buffer: header + maxn * field descriptors + estimated data
    let header_sz = SIZEOF_HEADER + sproto_type.maxn * SIZEOF_FIELD;
    let mut header = vec![0u8; header_sz];
    let mut data_part: Vec<u8> = Vec::new();

    let mut index = 0usize; // current field descriptor index
    let mut last_tag: i32 = -1;

    for field in &sproto_type.fields {
        let val = fields_map.get(&field.name);

        // Skip nil fields
        let val = match val {
            Some(v) => v,
            None => continue,
        };

        let mut inline_value: u16 = 0; // 0 means data is in data part
        let has_data;

        if field.is_array {
            let arr = match val {
                SprotoValue::Array(a) => a,
                _ => {
                    return Err(EncodeError::TypeMismatch {
                        field: field.name.clone(),
                        expected: "array".into(),
                        actual: val.type_name().into(),
                    });
                }
            };
            let encoded = encode_array(sproto, field, arr)?;
            if encoded.is_empty() {
                continue; // empty array, skip
            }
            data_part.extend_from_slice(&encoded);
            has_data = true;
        } else {
            match encode_field(sproto, field, val)? {
                EncodedField::Inline(v) => {
                    inline_value = v;
                    has_data = true;
                }
                EncodedField::Data(d) => {
                    data_part.extend_from_slice(&d);
                    has_data = true;
                }
                // nil fields were already filtered above
            }
        }

        if has_data {
            // Handle tag gap: insert skip marker
            let tag_gap = field.tag as i32 - last_tag - 1;
            if tag_gap > 0 {
                let skip = ((tag_gap - 1) * 2 + 1) as u16;
                let offset = SIZEOF_HEADER + SIZEOF_FIELD * index;
                write_u16_le(&mut header[offset..], skip);
                index += 1;
            }

            // Write field descriptor
            let offset = SIZEOF_HEADER + SIZEOF_FIELD * index;
            write_u16_le(&mut header[offset..], inline_value);
            index += 1;
            last_tag = field.tag as i32;
        }
    }

    // Write field count
    write_u16_le(&mut header[0..], index as u16);

    // Compact: only keep the used header portion
    let used_header = SIZEOF_HEADER + index * SIZEOF_FIELD;
    header.truncate(used_header);
    header.extend_from_slice(&data_part);

    Ok(header)
}

/// Result of encoding a single non-array field.
enum EncodedField {
    /// Value encoded inline in the field descriptor.
    Inline(u16),
    /// Value encoded in the data part (with length prefix).
    Data(Vec<u8>),
}

fn encode_field(
    sproto: &Sproto,
    field: &Field,
    value: &SprotoValue,
) -> Result<EncodedField, EncodeError> {
    match &field.field_type {
        FieldType::Integer | FieldType::Boolean => {
            let int_val = match value {
                SprotoValue::Integer(v) => *v,
                SprotoValue::Boolean(v) => {
                    if *v {
                        1i64
                    } else {
                        0i64
                    }
                }
                _ => {
                    return Err(EncodeError::TypeMismatch {
                        field: field.name.clone(),
                        expected: "integer or boolean".into(),
                        actual: value.type_name().into(),
                    });
                }
            };

            // Apply decimal precision: only convert Double to scaled integer.
            // Integer values are assumed to already be in wire (scaled) form.
            let int_val = if field.decimal_precision > 0 {
                match value {
                    SprotoValue::Double(v) => {
                        (*v * field.decimal_precision as f64).round() as i64
                    }
                    _ => int_val,
                }
            } else {
                int_val
            };

            let uint_val = int_val as u64;

            // Try inline for small positive values
            // The C code checks: u32 < 0x7fff, and uses (u32+1)*2
            let u32_val = uint_val as u32;
            if uint_val == u32_val as u64 && u32_val < 0x7fff {
                return Ok(EncodedField::Inline(((u32_val + 1) * 2) as u16));
            }

            // Check if fits in 32 bits
            let i32_check = int_val as i32;
            if i32_check as i64 == int_val {
                let mut buf = vec![0u8; SIZEOF_LENGTH + SIZEOF_INT32];
                write_u32_le(&mut buf[0..], SIZEOF_INT32 as u32);
                write_u32_le(&mut buf[SIZEOF_LENGTH..], int_val as u32);
                Ok(EncodedField::Data(buf))
            } else {
                let mut buf = vec![0u8; SIZEOF_LENGTH + SIZEOF_INT64];
                write_u32_le(&mut buf[0..], SIZEOF_INT64 as u32);
                write_u64_le(&mut buf[SIZEOF_LENGTH..], uint_val);
                Ok(EncodedField::Data(buf))
            }
        }
        FieldType::Double => {
            let dval = match value {
                SprotoValue::Double(v) => *v,
                SprotoValue::Integer(v) => *v as f64,
                _ => {
                    return Err(EncodeError::TypeMismatch {
                        field: field.name.clone(),
                        expected: "double".into(),
                        actual: value.type_name().into(),
                    });
                }
            };
            let bits = dval.to_bits();
            let mut buf = vec![0u8; SIZEOF_LENGTH + SIZEOF_INT64];
            write_u32_le(&mut buf[0..], SIZEOF_INT64 as u32);
            write_u64_le(&mut buf[SIZEOF_LENGTH..], bits);
            Ok(EncodedField::Data(buf))
        }
        FieldType::String => {
            let s = match value {
                SprotoValue::Str(s) => s.as_bytes(),
                _ => {
                    return Err(EncodeError::TypeMismatch {
                        field: field.name.clone(),
                        expected: "string".into(),
                        actual: value.type_name().into(),
                    });
                }
            };
            let mut buf = vec![0u8; SIZEOF_LENGTH + s.len()];
            write_u32_le(&mut buf[0..], s.len() as u32);
            buf[SIZEOF_LENGTH..].copy_from_slice(s);
            Ok(EncodedField::Data(buf))
        }
        FieldType::Binary => {
            let b = match value {
                SprotoValue::Binary(b) => b.as_slice(),
                _ => {
                    return Err(EncodeError::TypeMismatch {
                        field: field.name.clone(),
                        expected: "binary".into(),
                        actual: value.type_name().into(),
                    });
                }
            };
            let mut buf = vec![0u8; SIZEOF_LENGTH + b.len()];
            write_u32_le(&mut buf[0..], b.len() as u32);
            buf[SIZEOF_LENGTH..].copy_from_slice(b);
            Ok(EncodedField::Data(buf))
        }
        FieldType::Struct(type_idx) => {
            let sub_type = &sproto.types_list[*type_idx];
            let encoded = encode(sproto, sub_type, value)?;
            let mut buf = vec![0u8; SIZEOF_LENGTH + encoded.len()];
            write_u32_le(&mut buf[0..], encoded.len() as u32);
            buf[SIZEOF_LENGTH..].copy_from_slice(&encoded);
            Ok(EncodedField::Data(buf))
        }
    }
}

fn encode_array(
    sproto: &Sproto,
    field: &Field,
    arr: &[SprotoValue],
) -> Result<Vec<u8>, EncodeError> {
    if arr.is_empty() {
        // Empty array: encode as 4-byte zero length
        let mut buf = vec![0u8; SIZEOF_LENGTH];
        write_u32_le(&mut buf[0..], 0);
        return Ok(buf);
    }

    let base_type = &field.field_type;
    match base_type {
        FieldType::Integer | FieldType::Double => {
            encode_integer_array(field, arr)
        }
        FieldType::Boolean => {
            encode_boolean_array(arr)
        }
        FieldType::String | FieldType::Binary | FieldType::Struct(_) => {
            encode_object_array(sproto, field, arr)
        }
    }
}

fn encode_integer_array(
    field: &Field,
    arr: &[SprotoValue],
) -> Result<Vec<u8>, EncodeError> {
    let is_double = field.field_type == FieldType::Double;

    // Collect all values
    let mut values: Vec<u64> = Vec::with_capacity(arr.len());
    let mut need_64bit = is_double;

    for val in arr {
        if is_double {
            let dval = match val {
                SprotoValue::Double(v) => *v,
                SprotoValue::Integer(v) => *v as f64,
                _ => {
                    return Err(EncodeError::TypeMismatch {
                        field: field.name.clone(),
                        expected: "double".into(),
                        actual: val.type_name().into(),
                    });
                }
            };
            values.push(dval.to_bits());
            need_64bit = true;
        } else {
            let ival = match val {
                SprotoValue::Integer(v) => *v,
                SprotoValue::Double(v) => {
                    if field.decimal_precision > 0 {
                        (*v * field.decimal_precision as f64).round() as i64
                    } else {
                        *v as i64
                    }
                }
                _ => {
                    return Err(EncodeError::TypeMismatch {
                        field: field.name.clone(),
                        expected: "integer".into(),
                        actual: val.type_name().into(),
                    });
                }
            };

            let uval = ival as u64;
            // Check if fits in i32
            if (ival as i32) as i64 != ival {
                need_64bit = true;
            }
            values.push(uval);
        }
    }

    let int_size = if need_64bit { SIZEOF_INT64 } else { SIZEOF_INT32 };
    let data_len = 1 + values.len() * int_size; // 1 byte marker + values
    let mut buf = vec![0u8; SIZEOF_LENGTH + data_len];
    write_u32_le(&mut buf[0..], data_len as u32);
    buf[SIZEOF_LENGTH] = int_size as u8;

    let mut offset = SIZEOF_LENGTH + 1;
    for &v in &values {
        if need_64bit {
            if !is_double && (v as i32 as i64 as u64 == v || v >> 32 == 0 || v >> 32 == 0xFFFFFFFF) {
                // Sign-extend 32-bit to 64-bit
                write_u64_le(&mut buf[offset..], v);
            } else {
                write_u64_le(&mut buf[offset..], v);
            }
            offset += SIZEOF_INT64;
        } else {
            write_u32_le(&mut buf[offset..], v as u32);
            offset += SIZEOF_INT32;
        }
    }

    Ok(buf)
}

fn encode_boolean_array(arr: &[SprotoValue]) -> Result<Vec<u8>, EncodeError> {
    let data_len = arr.len();
    let mut buf = vec![0u8; SIZEOF_LENGTH + data_len];
    write_u32_le(&mut buf[0..], data_len as u32);

    for (i, val) in arr.iter().enumerate() {
        let bval = match val {
            SprotoValue::Boolean(v) => *v,
            _ => {
                return Err(EncodeError::TypeMismatch {
                    field: "array element".into(),
                    expected: "boolean".into(),
                    actual: val.type_name().into(),
                });
            }
        };
        buf[SIZEOF_LENGTH + i] = if bval { 1 } else { 0 };
    }

    Ok(buf)
}

fn encode_object_array(
    sproto: &Sproto,
    field: &Field,
    arr: &[SprotoValue],
) -> Result<Vec<u8>, EncodeError> {
    let mut inner = Vec::new();

    for val in arr {
        let encoded = match &field.field_type {
            FieldType::String => {
                let s = match val {
                    SprotoValue::Str(s) => s.as_bytes().to_vec(),
                    _ => {
                        return Err(EncodeError::TypeMismatch {
                            field: field.name.clone(),
                            expected: "string".into(),
                            actual: val.type_name().into(),
                        });
                    }
                };
                s
            }
            FieldType::Binary => {
                match val {
                    SprotoValue::Binary(b) => b.clone(),
                    _ => {
                        return Err(EncodeError::TypeMismatch {
                            field: field.name.clone(),
                            expected: "binary".into(),
                            actual: val.type_name().into(),
                        });
                    }
                }
            }
            FieldType::Struct(type_idx) => {
                let sub_type = &sproto.types_list[*type_idx];
                encode(sproto, sub_type, val)?
            }
            _ => unreachable!(),
        };

        // Each element is prefixed with its length
        let mut elem_buf = vec![0u8; SIZEOF_LENGTH + encoded.len()];
        write_u32_le(&mut elem_buf[0..], encoded.len() as u32);
        elem_buf[SIZEOF_LENGTH..].copy_from_slice(&encoded);
        inner.extend_from_slice(&elem_buf);
    }

    // Outer length prefix
    let mut buf = vec![0u8; SIZEOF_LENGTH + inner.len()];
    write_u32_le(&mut buf[0..], inner.len() as u32);
    buf[SIZEOF_LENGTH..].copy_from_slice(&inner);

    Ok(buf)
}
