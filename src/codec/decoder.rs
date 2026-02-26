use std::collections::HashMap;

use crate::error::DecodeError;
use crate::types::{Field, FieldType, Sproto, SprotoType};
use crate::value::SprotoValue;

use super::wire::*;

/// Decode binary data into a `SprotoValue::Struct` according to a `SprotoType` schema.
pub fn decode(
    sproto: &Sproto,
    sproto_type: &SprotoType,
    data: &[u8],
) -> Result<SprotoValue, DecodeError> {
    let size = data.len();
    if size < SIZEOF_HEADER {
        return Err(DecodeError::Truncated {
            need: SIZEOF_HEADER,
            have: size,
        });
    }

    let fn_count = read_u16_le(&data[0..]) as usize;
    let field_part_end = SIZEOF_HEADER + fn_count * SIZEOF_FIELD;
    if size < field_part_end {
        return Err(DecodeError::Truncated {
            need: field_part_end,
            have: size,
        });
    }

    let field_part = &data[SIZEOF_HEADER..field_part_end];
    let mut data_offset = field_part_end;
    let mut result: HashMap<String, SprotoValue> = HashMap::new();

    let mut tag: i32 = -1;

    for i in 0..fn_count {
        let value = read_u16_le(&field_part[i * SIZEOF_FIELD..]) as i32;
        tag += 1;

        if value & 1 != 0 {
            // Odd value: skip tag
            tag += value / 2;
            continue;
        }

        let decoded_value = value / 2 - 1;

        // If decoded_value < 0, the data is in the data part
        let current_data_start = data_offset;
        if decoded_value < 0 {
            if data_offset + SIZEOF_LENGTH > size {
                return Err(DecodeError::Truncated {
                    need: data_offset + SIZEOF_LENGTH,
                    have: size,
                });
            }
            let dsz = read_u32_le(&data[data_offset..]) as usize;
            if data_offset + SIZEOF_LENGTH + dsz > size {
                return Err(DecodeError::Truncated {
                    need: data_offset + SIZEOF_LENGTH + dsz,
                    have: size,
                });
            }
            data_offset += SIZEOF_LENGTH + dsz;
        }

        // Find field in schema
        let field = sproto_type.find_field_by_tag(tag as u16);
        let field = match field {
            Some(f) => f,
            None => continue, // Unknown tag, skip for forward compatibility
        };

        if decoded_value < 0 {
            // Data is in data part
            let field_data = &data[current_data_start..data_offset];

            if field.is_array {
                let val = decode_array(sproto, field, field_data)?;
                result.insert(field.name.clone(), val);
            } else {
                let val = decode_field_from_data(sproto, field, field_data)?;
                result.insert(field.name.clone(), val);
            }
        } else {
            // Inline value
            let val = decode_inline_value(field, decoded_value as u64)?;
            result.insert(field.name.clone(), val);
        }
    }

    Ok(SprotoValue::Struct(result))
}

fn decode_inline_value(field: &Field, value: u64) -> Result<SprotoValue, DecodeError> {
    match &field.field_type {
        FieldType::Integer => Ok(SprotoValue::Integer(value as i64)),
        FieldType::Boolean => Ok(SprotoValue::Boolean(value != 0)),
        _ => Err(DecodeError::InvalidData(format!(
            "field '{}' type {:?} cannot have inline value",
            field.name, field.field_type
        ))),
    }
}

fn decode_field_from_data(
    sproto: &Sproto,
    field: &Field,
    data: &[u8],
) -> Result<SprotoValue, DecodeError> {
    let sz = read_u32_le(&data[0..]) as usize;
    let content = &data[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];

    match &field.field_type {
        FieldType::Integer | FieldType::Double => {
            if sz == SIZEOF_INT32 {
                let v = expand64(read_u32_le(content));
                if field.field_type == FieldType::Double {
                    // For double stored in data part, it's always 8 bytes
                    return Err(DecodeError::InvalidData(format!(
                        "double field '{}' has 4-byte data, expected 8",
                        field.name
                    )));
                }
                Ok(SprotoValue::Integer(v as i64))
            } else if sz == SIZEOF_INT64 {
                let low = read_u32_le(content) as u64;
                let hi = read_u32_le(&content[SIZEOF_INT32..]) as u64;
                let v = low | (hi << 32);
                if field.field_type == FieldType::Double {
                    Ok(SprotoValue::Double(f64::from_bits(v)))
                } else {
                    Ok(SprotoValue::Integer(v as i64))
                }
            } else {
                Err(DecodeError::InvalidData(format!(
                    "integer/double field '{}' has invalid size {}",
                    field.name, sz
                )))
            }
        }
        FieldType::String => {
            let s = String::from_utf8(content.to_vec()).map_err(|e| DecodeError::InvalidUtf8 {
                field: field.name.clone(),
                source: e,
            })?;
            Ok(SprotoValue::Str(s))
        }
        FieldType::Binary => Ok(SprotoValue::Binary(content.to_vec())),
        FieldType::Boolean => {
            // Booleans shouldn't be in data part for non-arrays
            Err(DecodeError::InvalidData(format!(
                "boolean field '{}' in data part",
                field.name
            )))
        }
        FieldType::Struct(type_idx) => {
            let sub_type = &sproto.types_list[*type_idx];
            decode(sproto, sub_type, content)
        }
    }
}

fn decode_array(
    sproto: &Sproto,
    field: &Field,
    data: &[u8],
) -> Result<SprotoValue, DecodeError> {
    let sz = read_u32_le(&data[0..]) as usize;
    if sz == 0 {
        return Ok(SprotoValue::Array(Vec::new()));
    }
    let content = &data[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];

    let base_type = &field.field_type;
    match base_type {
        FieldType::Integer | FieldType::Double => {
            decode_integer_array(field, content)
        }
        FieldType::Boolean => decode_boolean_array(content),
        FieldType::String | FieldType::Binary | FieldType::Struct(_) => {
            decode_object_array(sproto, field, content)
        }
    }
}

fn decode_integer_array(
    field: &Field,
    content: &[u8],
) -> Result<SprotoValue, DecodeError> {
    if content.is_empty() {
        return Ok(SprotoValue::Array(Vec::new()));
    }

    let int_len = content[0] as usize;
    let values_data = &content[1..];

    if int_len != SIZEOF_INT32 && int_len != SIZEOF_INT64 {
        return Err(DecodeError::InvalidData(format!(
            "integer array has invalid element size {}",
            int_len
        )));
    }

    if values_data.len() % int_len != 0 {
        return Err(DecodeError::InvalidData(format!(
            "integer array data length {} not divisible by element size {}",
            values_data.len(),
            int_len
        )));
    }

    let count = values_data.len() / int_len;
    let mut arr = Vec::with_capacity(count);
    let is_double = field.field_type == FieldType::Double;

    for i in 0..count {
        let offset = i * int_len;
        if int_len == SIZEOF_INT32 {
            let v = expand64(read_u32_le(&values_data[offset..]));
            if is_double {
                arr.push(SprotoValue::Double(f64::from_bits(v)));
            } else {
                arr.push(SprotoValue::Integer(v as i64));
            }
        } else {
            let low = read_u32_le(&values_data[offset..]) as u64;
            let hi = read_u32_le(&values_data[offset + SIZEOF_INT32..]) as u64;
            let v = low | (hi << 32);
            if is_double {
                arr.push(SprotoValue::Double(f64::from_bits(v)));
            } else {
                arr.push(SprotoValue::Integer(v as i64));
            }
        }
    }

    Ok(SprotoValue::Array(arr))
}

fn decode_boolean_array(content: &[u8]) -> Result<SprotoValue, DecodeError> {
    let arr: Vec<SprotoValue> = content
        .iter()
        .map(|&b| SprotoValue::Boolean(b != 0))
        .collect();
    Ok(SprotoValue::Array(arr))
}

fn decode_object_array(
    sproto: &Sproto,
    field: &Field,
    mut content: &[u8],
) -> Result<SprotoValue, DecodeError> {
    let mut arr = Vec::new();

    while !content.is_empty() {
        if content.len() < SIZEOF_LENGTH {
            return Err(DecodeError::InvalidData(
                "truncated object array element".into(),
            ));
        }
        let elem_sz = read_u32_le(content) as usize;
        let elem_data = &content[SIZEOF_LENGTH..SIZEOF_LENGTH + elem_sz];

        let val = match &field.field_type {
            FieldType::String => {
                let s = String::from_utf8(elem_data.to_vec()).map_err(|e| {
                    DecodeError::InvalidUtf8 {
                        field: field.name.clone(),
                        source: e,
                    }
                })?;
                SprotoValue::Str(s)
            }
            FieldType::Binary => SprotoValue::Binary(elem_data.to_vec()),
            FieldType::Struct(type_idx) => {
                let sub_type = &sproto.types_list[*type_idx];
                decode(sproto, sub_type, elem_data)?
            }
            _ => unreachable!(),
        };

        arr.push(val);
        content = &content[SIZEOF_LENGTH + elem_sz..];
    }

    Ok(SprotoValue::Array(arr))
}
