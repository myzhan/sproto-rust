//! Lua-native sproto codec: encode/decode directly between Lua tables and wire format.
//!
//! This replaces the SprotoValue-based path with direct Lua table ↔ wire conversion.

use mlua::prelude::*;
use sproto::codec::wire::*;
use sproto::types::{FieldType, Sproto, SprotoType, Field};

// ---------------------------------------------------------------------------
// Encoder: LuaTable → wire bytes
// ---------------------------------------------------------------------------

/// Encode a Lua table into sproto wire format according to the given type schema.
pub fn lua_encode(
    lua: &Lua,
    sproto: &Sproto,
    sproto_type: &SprotoType,
    table: &LuaTable,
) -> LuaResult<Vec<u8>> {
    let mut buf = Vec::with_capacity(256);
    lua_encode_into(lua, sproto, sproto_type, table, &mut buf)?;
    Ok(buf)
}

fn lua_encode_into(
    lua: &Lua,
    sproto: &Sproto,
    sproto_type: &SprotoType,
    table: &LuaTable,
    output: &mut Vec<u8>,
) -> LuaResult<()> {
    let header_start = output.len();
    let header_sz = SIZEOF_HEADER + sproto_type.maxn * SIZEOF_FIELD;
    output.resize(header_start + header_sz, 0);

    let mut index = 0usize;
    let mut last_tag: i32 = -1;

    for field in &sproto_type.fields {
        let val: LuaValue = table.get(field.name.as_ref())?;
        if val == LuaValue::Nil {
            continue;
        }

        let mut inline_value: u16 = 0;
        let has_data;

        if field.is_array {
            let arr = match &val {
                LuaValue::Table(t) => t,
                _ => {
                    return Err(LuaError::RuntimeError(format!(
                        "field '{}': expected table (array), got {}",
                        field.name,
                        val.type_name()
                    )));
                }
            };
            encode_lua_array(lua, sproto, field, arr, output)?;
            has_data = true;
        } else {
            match encode_lua_field(lua, sproto, field, &val, output)? {
                FieldEncoded::Inline(v) => {
                    inline_value = v;
                    has_data = true;
                }
                FieldEncoded::DataWritten => {
                    has_data = true;
                }
            }
        }

        if has_data {
            let tag_gap = field.tag as i32 - last_tag - 1;
            if tag_gap > 0 {
                let skip = ((tag_gap - 1) * 2 + 1) as u16;
                let offset = header_start + SIZEOF_HEADER + SIZEOF_FIELD * index;
                write_u16_le(&mut output[offset..], skip);
                index += 1;
            }

            let offset = header_start + SIZEOF_HEADER + SIZEOF_FIELD * index;
            write_u16_le(&mut output[offset..], inline_value);
            index += 1;
            last_tag = field.tag as i32;
        }
    }

    write_u16_le(&mut output[header_start..], index as u16);

    // Compact: shift data part to fill unused header slots
    let used_header = SIZEOF_HEADER + index * SIZEOF_FIELD;
    let unused = header_sz - used_header;
    if unused > 0 {
        let data_start = header_start + header_sz;
        let data_end = output.len();
        if data_start < data_end {
            output.copy_within(data_start..data_end, data_start - unused);
        }
        output.truncate(data_end - unused);
    }

    Ok(())
}

enum FieldEncoded {
    Inline(u16),
    DataWritten,
}

fn encode_lua_field(
    lua: &Lua,
    sproto: &Sproto,
    field: &Field,
    val: &LuaValue,
    buf: &mut Vec<u8>,
) -> LuaResult<FieldEncoded> {
    match &field.field_type {
        FieldType::Integer | FieldType::Boolean => {
            let int_val = lua_val_to_i64(val, &field.name)?;

            let int_val = if field.decimal_precision > 0 {
                if let LuaValue::Number(n) = val {
                    (*n * field.decimal_precision as f64).round() as i64
                } else {
                    int_val
                }
            } else {
                int_val
            };

            let uint_val = int_val as u64;
            let u32_val = uint_val as u32;
            if uint_val == u32_val as u64 && u32_val < 0x7fff {
                return Ok(FieldEncoded::Inline(((u32_val + 1) * 2) as u16));
            }

            let i32_check = int_val as i32;
            if i32_check as i64 == int_val {
                let start = buf.len();
                buf.resize(start + SIZEOF_LENGTH + SIZEOF_INT32, 0);
                write_u32_le(&mut buf[start..], SIZEOF_INT32 as u32);
                write_u32_le(&mut buf[start + SIZEOF_LENGTH..], int_val as u32);
            } else {
                let start = buf.len();
                buf.resize(start + SIZEOF_LENGTH + SIZEOF_INT64, 0);
                write_u32_le(&mut buf[start..], SIZEOF_INT64 as u32);
                write_u64_le(&mut buf[start + SIZEOF_LENGTH..], uint_val);
            }
            Ok(FieldEncoded::DataWritten)
        }
        FieldType::Double => {
            let dval = lua_val_to_f64(val, &field.name)?;
            let bits = dval.to_bits();
            let start = buf.len();
            buf.resize(start + SIZEOF_LENGTH + SIZEOF_INT64, 0);
            write_u32_le(&mut buf[start..], SIZEOF_INT64 as u32);
            write_u64_le(&mut buf[start + SIZEOF_LENGTH..], bits);
            Ok(FieldEncoded::DataWritten)
        }
        FieldType::String | FieldType::Binary => {
            let bytes = lua_val_to_bytes(val, &field.name)?;
            write_length_prefixed_bytes(buf, &bytes);
            Ok(FieldEncoded::DataWritten)
        }
        FieldType::Struct(type_idx) => {
            let sub_table = match val {
                LuaValue::Table(t) => t,
                _ => {
                    return Err(LuaError::RuntimeError(format!(
                        "field '{}': expected table, got {}",
                        field.name,
                        val.type_name()
                    )));
                }
            };
            let sub_type = &sproto.types_list[*type_idx];
            let len_pos = buf.len();
            buf.resize(len_pos + SIZEOF_LENGTH, 0);
            let data_start = buf.len();
            lua_encode_into(lua, sproto, sub_type, sub_table, buf)?;
            let encoded_len = buf.len() - data_start;
            write_u32_le(&mut buf[len_pos..], encoded_len as u32);
            Ok(FieldEncoded::DataWritten)
        }
    }
}

fn encode_lua_array(
    lua: &Lua,
    sproto: &Sproto,
    field: &Field,
    arr: &LuaTable,
    buf: &mut Vec<u8>,
) -> LuaResult<()> {
    let len = arr.len()? as usize;
    if len == 0 {
        let start = buf.len();
        buf.extend_from_slice(&[0u8; SIZEOF_LENGTH]);
        write_u32_le(&mut buf[start..], 0);
        return Ok(());
    }

    match &field.field_type {
        FieldType::Integer | FieldType::Double => {
            encode_lua_integer_array(field, arr, len, buf)
        }
        FieldType::Boolean => encode_lua_boolean_array(arr, len, buf),
        FieldType::String | FieldType::Binary | FieldType::Struct(_) => {
            encode_lua_object_array(lua, sproto, field, arr, len, buf)
        }
    }
}

fn encode_lua_integer_array(
    field: &Field,
    arr: &LuaTable,
    len: usize,
    buf: &mut Vec<u8>,
) -> LuaResult<()> {
    let is_double = field.field_type == FieldType::Double;
    let mut values: Vec<u64> = Vec::with_capacity(len);
    let mut need_64bit = is_double;

    for i in 1..=len {
        let val: LuaValue = arr.get(i as i64)?;
        if is_double {
            let dval = lua_val_to_f64(&val, &field.name)?;
            values.push(dval.to_bits());
        } else {
            let ival = lua_val_to_i64(&val, &field.name)?;
            let ival = if field.decimal_precision > 0 {
                if let LuaValue::Number(n) = &val {
                    (*n * field.decimal_precision as f64).round() as i64
                } else {
                    ival
                }
            } else {
                ival
            };
            if (ival as i32) as i64 != ival {
                need_64bit = true;
            }
            values.push(ival as u64);
        }
    }

    let int_size = if need_64bit { SIZEOF_INT64 } else { SIZEOF_INT32 };
    let data_len = 1 + values.len() * int_size;

    let start = buf.len();
    buf.resize(start + SIZEOF_LENGTH + data_len, 0);
    write_u32_le(&mut buf[start..], data_len as u32);
    buf[start + SIZEOF_LENGTH] = int_size as u8;

    let mut offset = start + SIZEOF_LENGTH + 1;
    for &v in &values {
        if need_64bit {
            write_u64_le(&mut buf[offset..], v);
            offset += SIZEOF_INT64;
        } else {
            write_u32_le(&mut buf[offset..], v as u32);
            offset += SIZEOF_INT32;
        }
    }

    Ok(())
}

fn encode_lua_boolean_array(arr: &LuaTable, len: usize, buf: &mut Vec<u8>) -> LuaResult<()> {
    let start = buf.len();
    buf.resize(start + SIZEOF_LENGTH + len, 0);
    write_u32_le(&mut buf[start..], len as u32);

    for i in 1..=len {
        let val: bool = arr.get(i as i64)?;
        buf[start + SIZEOF_LENGTH + i - 1] = if val { 1 } else { 0 };
    }

    Ok(())
}

fn encode_lua_object_array(
    lua: &Lua,
    sproto: &Sproto,
    field: &Field,
    arr: &LuaTable,
    len: usize,
    buf: &mut Vec<u8>,
) -> LuaResult<()> {
    let outer_len_pos = buf.len();
    buf.resize(outer_len_pos + SIZEOF_LENGTH, 0);
    let outer_data_start = buf.len();

    for i in 1..=len {
        let val: LuaValue = arr.get(i as i64)?;
        let elem_len_pos = buf.len();
        buf.resize(elem_len_pos + SIZEOF_LENGTH, 0);
        let elem_start = buf.len();

        match &field.field_type {
            FieldType::String | FieldType::Binary => {
                let bytes = lua_val_to_bytes(&val, &field.name)?;
                buf.extend_from_slice(&bytes);
            }
            FieldType::Struct(type_idx) => {
                let sub_table = match &val {
                    LuaValue::Table(t) => t,
                    _ => {
                        return Err(LuaError::RuntimeError(format!(
                            "field '{}': array element expected table, got {}",
                            field.name,
                            val.type_name()
                        )));
                    }
                };
                let sub_type = &sproto.types_list[*type_idx];
                lua_encode_into(lua, sproto, sub_type, sub_table, buf)?;
            }
            _ => unreachable!(),
        }

        let elem_len = buf.len() - elem_start;
        write_u32_le(&mut buf[elem_len_pos..], elem_len as u32);
    }

    let outer_len = buf.len() - outer_data_start;
    write_u32_le(&mut buf[outer_len_pos..], outer_len as u32);

    Ok(())
}

// ---------------------------------------------------------------------------
// Decoder: wire bytes → LuaTable
// ---------------------------------------------------------------------------

/// Decode sproto wire format into a Lua table according to the given type schema.
pub fn lua_decode(
    lua: &Lua,
    sproto: &Sproto,
    sproto_type: &SprotoType,
    data: &[u8],
) -> LuaResult<LuaTable> {
    let size = data.len();
    if size < SIZEOF_HEADER {
        return Err(LuaError::RuntimeError(format!(
            "truncated data: need {} bytes, have {}",
            SIZEOF_HEADER, size
        )));
    }

    let fn_count = read_u16_le(&data[0..]) as usize;
    let field_part_end = SIZEOF_HEADER + fn_count * SIZEOF_FIELD;
    if size < field_part_end {
        return Err(LuaError::RuntimeError(format!(
            "truncated data: need {} bytes, have {}",
            field_part_end, size
        )));
    }

    let field_part = &data[SIZEOF_HEADER..field_part_end];
    let mut data_offset = field_part_end;
    let result = lua.create_table()?;

    let mut tag: i32 = -1;

    for i in 0..fn_count {
        let value = read_u16_le(&field_part[i * SIZEOF_FIELD..]) as i32;
        tag += 1;

        if value & 1 != 0 {
            tag += value / 2;
            continue;
        }

        let decoded_value = value / 2 - 1;

        let current_data_start = data_offset;
        if decoded_value < 0 {
            if data_offset + SIZEOF_LENGTH > size {
                return Err(LuaError::RuntimeError(format!(
                    "truncated data at offset {}",
                    data_offset
                )));
            }
            let dsz = read_u32_le(&data[data_offset..]) as usize;
            if data_offset + SIZEOF_LENGTH + dsz > size {
                return Err(LuaError::RuntimeError(format!(
                    "truncated data at offset {}",
                    data_offset
                )));
            }
            data_offset += SIZEOF_LENGTH + dsz;
        }

        let field = match sproto_type.find_field_by_tag(tag as u16) {
            Some(f) => f,
            None => continue,
        };

        if decoded_value < 0 {
            let field_data = &data[current_data_start..data_offset];
            if field.is_array {
                let val = decode_lua_array(lua, sproto, field, field_data)?;
                result.set(field.name.as_ref(), val)?;
            } else {
                let val = decode_lua_field_data(lua, sproto, field, field_data)?;
                result.set(field.name.as_ref(), val)?;
            }
        } else {
            let val = decode_lua_inline(lua, field, decoded_value as u64)?;
            result.set(field.name.as_ref(), val)?;
        }
    }

    Ok(result)
}

fn decode_lua_inline(
    _lua: &Lua,
    field: &Field,
    value: u64,
) -> LuaResult<LuaValue> {
    match &field.field_type {
        FieldType::Integer => Ok(LuaValue::Integer(value as i64)),
        FieldType::Boolean => Ok(LuaValue::Boolean(value != 0)),
        _ => Err(LuaError::RuntimeError(format!(
            "field '{}' type {:?} cannot have inline value",
            field.name, field.field_type
        ))),
    }
}

fn decode_lua_field_data(
    lua: &Lua,
    sproto: &Sproto,
    field: &Field,
    data: &[u8],
) -> LuaResult<LuaValue> {
    let sz = read_u32_le(&data[0..]) as usize;
    let content = &data[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];

    match &field.field_type {
        FieldType::Integer | FieldType::Double => {
            if sz == SIZEOF_INT32 {
                let v = expand64(read_u32_le(content));
                if field.field_type == FieldType::Double {
                    return Err(LuaError::RuntimeError(format!(
                        "double field '{}' has 4-byte data, expected 8",
                        field.name
                    )));
                }
                Ok(LuaValue::Integer(v as i64))
            } else if sz == SIZEOF_INT64 {
                let v = read_u64_le(content);
                if field.field_type == FieldType::Double {
                    Ok(LuaValue::Number(f64::from_bits(v)))
                } else {
                    Ok(LuaValue::Integer(v as i64))
                }
            } else {
                Err(LuaError::RuntimeError(format!(
                    "field '{}' has invalid integer size {}",
                    field.name, sz
                )))
            }
        }
        FieldType::String => {
            let s = lua.create_string(content)?;
            Ok(LuaValue::String(s))
        }
        FieldType::Binary => {
            let s = lua.create_string(content)?;
            Ok(LuaValue::String(s))
        }
        FieldType::Boolean => Err(LuaError::RuntimeError(format!(
            "boolean field '{}' in data part",
            field.name
        ))),
        FieldType::Struct(type_idx) => {
            let sub_type = &sproto.types_list[*type_idx];
            let table = lua_decode(lua, sproto, sub_type, content)?;
            Ok(LuaValue::Table(table))
        }
    }
}

fn decode_lua_array(
    lua: &Lua,
    sproto: &Sproto,
    field: &Field,
    data: &[u8],
) -> LuaResult<LuaValue> {
    let sz = read_u32_le(&data[0..]) as usize;
    if sz == 0 {
        let table = lua.create_table()?;
        return Ok(LuaValue::Table(table));
    }
    let content = &data[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];

    match &field.field_type {
        FieldType::Integer | FieldType::Double => decode_lua_integer_array(lua, field, content),
        FieldType::Boolean => decode_lua_boolean_array(lua, content),
        FieldType::String | FieldType::Binary | FieldType::Struct(_) => {
            decode_lua_object_array(lua, sproto, field, content)
        }
    }
}

fn decode_lua_integer_array(
    lua: &Lua,
    field: &Field,
    content: &[u8],
) -> LuaResult<LuaValue> {
    if content.is_empty() {
        let table = lua.create_table()?;
        return Ok(LuaValue::Table(table));
    }

    let int_len = content[0] as usize;
    let values_data = &content[1..];

    if int_len != SIZEOF_INT32 && int_len != SIZEOF_INT64 {
        return Err(LuaError::RuntimeError(format!(
            "integer array has invalid element size {}",
            int_len
        )));
    }

    if values_data.len() % int_len != 0 {
        return Err(LuaError::RuntimeError(format!(
            "integer array data not aligned to element size {}",
            int_len
        )));
    }

    let count = values_data.len() / int_len;
    let is_double = field.field_type == FieldType::Double;
    let table = lua.create_table_with_capacity(count, 0)?;

    for i in 0..count {
        let offset = i * int_len;
        if int_len == SIZEOF_INT32 {
            let v = expand64(read_u32_le(&values_data[offset..]));
            if is_double {
                table.set((i + 1) as i64, f64::from_bits(v))?;
            } else {
                table.set((i + 1) as i64, v as i64)?;
            }
        } else {
            let v = read_u64_le(&values_data[offset..]);
            if is_double {
                table.set((i + 1) as i64, f64::from_bits(v))?;
            } else {
                table.set((i + 1) as i64, v as i64)?;
            }
        }
    }

    Ok(LuaValue::Table(table))
}

fn decode_lua_boolean_array(lua: &Lua, content: &[u8]) -> LuaResult<LuaValue> {
    let table = lua.create_table_with_capacity(content.len(), 0)?;
    for (i, &b) in content.iter().enumerate() {
        table.set((i + 1) as i64, b != 0)?;
    }
    Ok(LuaValue::Table(table))
}

fn decode_lua_object_array(
    lua: &Lua,
    sproto: &Sproto,
    field: &Field,
    mut content: &[u8],
) -> LuaResult<LuaValue> {
    let table = lua.create_table()?;
    let mut idx = 1i64;

    while !content.is_empty() {
        if content.len() < SIZEOF_LENGTH {
            return Err(LuaError::RuntimeError(
                "truncated object array element".into(),
            ));
        }
        let elem_sz = read_u32_le(content) as usize;
        let elem_data = &content[SIZEOF_LENGTH..SIZEOF_LENGTH + elem_sz];

        match &field.field_type {
            FieldType::String | FieldType::Binary => {
                let s = lua.create_string(elem_data)?;
                table.set(idx, s)?;
            }
            FieldType::Struct(type_idx) => {
                let sub_type = &sproto.types_list[*type_idx];
                let sub_table = lua_decode(lua, sproto, sub_type, elem_data)?;
                table.set(idx, sub_table)?;
            }
            _ => unreachable!(),
        }

        idx += 1;
        content = &content[SIZEOF_LENGTH + elem_sz..];
    }

    Ok(LuaValue::Table(table))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[inline]
fn write_length_prefixed_bytes(buf: &mut Vec<u8>, data: &[u8]) {
    let start = buf.len();
    buf.reserve(SIZEOF_LENGTH + data.len());
    buf.resize(start + SIZEOF_LENGTH, 0);
    write_u32_le(&mut buf[start..], data.len() as u32);
    buf.extend_from_slice(data);
}

fn lua_val_to_i64(val: &LuaValue, field_name: &str) -> LuaResult<i64> {
    match val {
        LuaValue::Integer(i) => Ok(*i),
        LuaValue::Number(n) => Ok(*n as i64),
        LuaValue::Boolean(b) => Ok(if *b { 1 } else { 0 }),
        _ => Err(LuaError::RuntimeError(format!(
            "field '{}': expected integer, got {}",
            field_name,
            val.type_name()
        ))),
    }
}

fn lua_val_to_f64(val: &LuaValue, field_name: &str) -> LuaResult<f64> {
    match val {
        LuaValue::Number(n) => Ok(*n),
        LuaValue::Integer(i) => Ok(*i as f64),
        _ => Err(LuaError::RuntimeError(format!(
            "field '{}': expected number, got {}",
            field_name,
            val.type_name()
        ))),
    }
}

fn lua_val_to_bytes(val: &LuaValue, field_name: &str) -> LuaResult<Vec<u8>> {
    match val {
        LuaValue::String(s) => Ok(s.as_bytes().to_vec()),
        _ => Err(LuaError::RuntimeError(format!(
            "field '{}': expected string, got {}",
            field_name,
            val.type_name()
        ))),
    }
}
