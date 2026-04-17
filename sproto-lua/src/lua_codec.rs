//! Lua-native sproto codec: encode/decode directly between Lua tables and wire format.
//!
//! This module is a thin wrapper around the shared codec engine
//! (`StructEncoder` / `StructDecoder`), handling only `LuaValue <-> Rust`
//! type conversion while delegating all wire-format work to the engine.

use mlua::prelude::*;
use sproto::codec::decoder::{DecodedField, StructDecoder};
use sproto::codec::encoder::StructEncoder;
use sproto::error::EncodeError;
use sproto::types::{Field, FieldType, Sproto, SprotoType};

use crate::error::{decode_error_to_lua, encode_error_to_lua};

// ---------------------------------------------------------------------------
// Encoder: LuaTable -> wire bytes
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
    let mut enc = StructEncoder::new(sproto, sproto_type, output);
    lua_fill_encoder(lua, sproto, sproto_type, table, &mut enc).map_err(encode_error_to_lua)?;
    enc.finish();
    Ok(())
}

/// Fill a `StructEncoder` with values from a Lua table.
///
/// Returns `EncodeError` so the same function can be used inside
/// `encode_nested` / `encode_struct_array` closures.
fn lua_fill_encoder(
    lua: &Lua,
    sproto: &Sproto,
    sproto_type: &SprotoType,
    table: &LuaTable,
    enc: &mut StructEncoder,
) -> Result<(), EncodeError> {
    for field in &sproto_type.fields {
        let val: LuaValue = table
            .get(field.name.as_ref())
            .map_err(|e| EncodeError::Other(e.to_string()))?;
        if val == LuaValue::Nil {
            continue;
        }

        if field.is_array {
            encode_array_field(lua, sproto, field, &val, enc)?;
        } else {
            encode_scalar_field(lua, sproto, field, &val, enc)?;
        }
    }
    Ok(())
}

fn encode_scalar_field(
    lua: &Lua,
    sproto: &Sproto,
    field: &Field,
    val: &LuaValue,
    enc: &mut StructEncoder,
) -> Result<(), EncodeError> {
    match &field.field_type {
        FieldType::Integer | FieldType::Boolean => {
            let int_val = lua_to_i64(val, field)?;
            enc.set_integer(field.tag, int_val)?;
        }
        FieldType::Double => {
            let dval = lua_to_f64(val, &field.name)?;
            enc.set_double(field.tag, dval)?;
        }
        FieldType::String | FieldType::Binary => {
            let bytes = lua_to_bytes(val, &field.name)?;
            enc.set_bytes(field.tag, &bytes)?;
        }
        FieldType::Struct(type_idx) => {
            let sub_table = lua_to_table(val, &field.name)?;
            let sub_type = &sproto.types_list[*type_idx];
            enc.encode_nested(field.tag, |sub_enc| {
                lua_fill_encoder(lua, sproto, sub_type, sub_table, sub_enc)
            })?;
        }
    }
    Ok(())
}

fn encode_array_field(
    lua: &Lua,
    sproto: &Sproto,
    field: &Field,
    val: &LuaValue,
    enc: &mut StructEncoder,
) -> Result<(), EncodeError> {
    let arr = lua_to_table(val, &field.name)?;
    let len = arr.len().map_err(|e| EncodeError::Other(e.to_string()))? as usize;

    match &field.field_type {
        FieldType::Integer => {
            let mut values = Vec::with_capacity(len);
            for i in 1..=len {
                let v: LuaValue = arr
                    .get(i as i64)
                    .map_err(|e| EncodeError::Other(e.to_string()))?;
                values.push(lua_to_i64(&v, field)?);
            }
            enc.set_integer_array(field.tag, &values)?;
        }
        FieldType::Double => {
            let mut values = Vec::with_capacity(len);
            for i in 1..=len {
                let v: LuaValue = arr
                    .get(i as i64)
                    .map_err(|e| EncodeError::Other(e.to_string()))?;
                values.push(lua_to_f64(&v, &field.name)?);
            }
            enc.set_double_array(field.tag, &values)?;
        }
        FieldType::Boolean => {
            let mut values = Vec::with_capacity(len);
            for i in 1..=len {
                let v: bool = arr
                    .get(i as i64)
                    .map_err(|e| EncodeError::Other(e.to_string()))?;
                values.push(v);
            }
            enc.set_bool_array(field.tag, &values)?;
        }
        FieldType::String | FieldType::Binary => {
            let mut bytes_vec: Vec<Vec<u8>> = Vec::with_capacity(len);
            for i in 1..=len {
                let v: LuaValue = arr
                    .get(i as i64)
                    .map_err(|e| EncodeError::Other(e.to_string()))?;
                bytes_vec.push(lua_to_bytes(&v, &field.name)?);
            }
            enc.set_bytes_array(field.tag, &bytes_vec)?;
        }
        FieldType::Struct(type_idx) => {
            let sub_type = &sproto.types_list[*type_idx];
            enc.encode_struct_array(field.tag, |arr_enc| {
                for i in 1..=len {
                    let v: LuaValue = arr
                        .get(i as i64)
                        .map_err(|e| EncodeError::Other(e.to_string()))?;
                    let sub_table = lua_to_table(&v, &field.name)?;
                    arr_enc.encode_element(|elem_enc| {
                        lua_fill_encoder(lua, sproto, sub_type, sub_table, elem_enc)
                    })?;
                }
                Ok(())
            })?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Decoder: wire bytes -> LuaTable
// ---------------------------------------------------------------------------

/// Decode sproto wire format into a Lua table according to the given type schema.
pub fn lua_decode(
    lua: &Lua,
    sproto: &Sproto,
    sproto_type: &SprotoType,
    data: &[u8],
) -> LuaResult<LuaTable> {
    let dec = StructDecoder::new(sproto, sproto_type, data).map_err(decode_error_to_lua)?;
    lua_decode_fields(lua, dec)
}

fn lua_decode_fields(lua: &Lua, mut dec: StructDecoder) -> LuaResult<LuaTable> {
    let result = lua.create_table()?;
    while let Some(f) = dec.next_field().map_err(decode_error_to_lua)? {
        let field_ref = f.field();
        let lua_val = if field_ref.is_array {
            decode_array_field(lua, &f)?
        } else {
            decode_scalar_field(lua, &f)?
        };
        result.set(field_ref.name.as_ref(), lua_val)?;
    }
    Ok(result)
}

fn decode_scalar_field(lua: &Lua, f: &DecodedField) -> LuaResult<LuaValue> {
    match &f.field().field_type {
        FieldType::Integer => {
            let v = f.as_integer().map_err(decode_error_to_lua)?;
            Ok(LuaValue::Integer(v))
        }
        FieldType::Boolean => {
            let v = f.as_bool().map_err(decode_error_to_lua)?;
            Ok(LuaValue::Boolean(v))
        }
        FieldType::Double => {
            let v = f.as_double().map_err(decode_error_to_lua)?;
            Ok(LuaValue::Number(v))
        }
        FieldType::String | FieldType::Binary => {
            let s = lua.create_string(f.as_bytes())?;
            Ok(LuaValue::String(s))
        }
        FieldType::Struct(_) => {
            let sub_dec = f.as_struct().map_err(decode_error_to_lua)?;
            let table = lua_decode_fields(lua, sub_dec)?;
            Ok(LuaValue::Table(table))
        }
    }
}

fn decode_array_field(lua: &Lua, f: &DecodedField) -> LuaResult<LuaValue> {
    match &f.field().field_type {
        FieldType::Integer => {
            let values = f.as_integer_array().map_err(decode_error_to_lua)?;
            let table = lua.create_table_with_capacity(values.len(), 0)?;
            for (i, &v) in values.iter().enumerate() {
                table.set((i + 1) as i64, v)?;
            }
            Ok(LuaValue::Table(table))
        }
        FieldType::Double => {
            let values = f.as_double_array().map_err(decode_error_to_lua)?;
            let table = lua.create_table_with_capacity(values.len(), 0)?;
            for (i, &v) in values.iter().enumerate() {
                table.set((i + 1) as i64, v)?;
            }
            Ok(LuaValue::Table(table))
        }
        FieldType::Boolean => {
            let values = f.as_bool_array();
            let table = lua.create_table_with_capacity(values.len(), 0)?;
            for (i, &v) in values.iter().enumerate() {
                table.set((i + 1) as i64, v)?;
            }
            Ok(LuaValue::Table(table))
        }
        FieldType::String | FieldType::Binary => {
            let values = f.as_bytes_array().map_err(decode_error_to_lua)?;
            let table = lua.create_table_with_capacity(values.len(), 0)?;
            for (i, v) in values.iter().enumerate() {
                let s = lua.create_string(v)?;
                table.set((i + 1) as i64, s)?;
            }
            Ok(LuaValue::Table(table))
        }
        FieldType::Struct(_) => {
            let iter = f.as_struct_iter().map_err(decode_error_to_lua)?;
            let table = lua.create_table()?;
            let mut idx = 1i64;
            for elem_result in iter {
                let sub_dec = elem_result.map_err(decode_error_to_lua)?;
                let sub_table = lua_decode_fields(lua, sub_dec)?;
                table.set(idx, sub_table)?;
                idx += 1;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

// ---------------------------------------------------------------------------
// LuaValue -> Rust type conversion helpers
// ---------------------------------------------------------------------------

/// Convert a Lua value to i64, applying `decimal_precision` when the value is
/// a float and the field declares a non-zero precision.
fn lua_to_i64(val: &LuaValue, field: &Field) -> Result<i64, EncodeError> {
    match val {
        LuaValue::Integer(i) => Ok(*i),
        LuaValue::Number(n) => {
            if field.decimal_precision > 0 {
                Ok((*n * field.decimal_precision as f64).round() as i64)
            } else {
                Ok(*n as i64)
            }
        }
        LuaValue::Boolean(b) => Ok(if *b { 1 } else { 0 }),
        _ => Err(EncodeError::TypeMismatch {
            field: field.name.to_string(),
            expected: "integer".into(),
            actual: val.type_name().into(),
        }),
    }
}

fn lua_to_f64(val: &LuaValue, field_name: &str) -> Result<f64, EncodeError> {
    match val {
        LuaValue::Number(n) => Ok(*n),
        LuaValue::Integer(i) => Ok(*i as f64),
        _ => Err(EncodeError::TypeMismatch {
            field: field_name.to_string(),
            expected: "number".into(),
            actual: val.type_name().into(),
        }),
    }
}

fn lua_to_bytes(val: &LuaValue, field_name: &str) -> Result<Vec<u8>, EncodeError> {
    match val {
        LuaValue::String(s) => Ok(s.as_bytes().to_vec()),
        _ => Err(EncodeError::TypeMismatch {
            field: field_name.to_string(),
            expected: "string".into(),
            actual: val.type_name().into(),
        }),
    }
}

fn lua_to_table<'a>(val: &'a LuaValue, field_name: &str) -> Result<&'a LuaTable, EncodeError> {
    match val {
        LuaValue::Table(t) => Ok(t),
        _ => Err(EncodeError::TypeMismatch {
            field: field_name.to_string(),
            expected: "table".into(),
            actual: val.type_name().into(),
        }),
    }
}
