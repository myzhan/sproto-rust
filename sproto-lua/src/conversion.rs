//! Type conversion between Lua values and SprotoValue.

use mlua::prelude::*;
use sproto::value::SprotoValue;
use std::collections::HashMap;

/// Convert a Lua value to SprotoValue.
pub fn lua_to_sproto_value(lua: &Lua, value: LuaValue) -> LuaResult<SprotoValue> {
    match value {
        LuaValue::Nil => {
            // Nil is not directly representable in SprotoValue
            // Return an empty struct as a placeholder (will be handled by caller)
            Err(LuaError::RuntimeError("nil value not allowed".into()))
        }
        LuaValue::Boolean(b) => Ok(SprotoValue::Boolean(b)),
        LuaValue::Integer(i) => Ok(SprotoValue::Integer(i)),
        LuaValue::Number(n) => {
            // Check if it's actually an integer
            if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                Ok(SprotoValue::Integer(n as i64))
            } else {
                Ok(SprotoValue::Double(n))
            }
        }
        LuaValue::String(s) => {
            // Try to convert to UTF-8 string, otherwise treat as binary
            match s.to_str() {
                Ok(str_val) => Ok(SprotoValue::Str(str_val.to_string())),
                Err(_) => Ok(SprotoValue::Binary(s.as_bytes().to_vec())),
            }
        }
        LuaValue::Table(t) => table_to_sproto_value(lua, t),
        _ => Err(LuaError::RuntimeError(format!(
            "unsupported Lua type: {:?}",
            value.type_name()
        ))),
    }
}

/// Convert a Lua table to SprotoValue (array or struct).
pub fn table_to_sproto_value(lua: &Lua, table: LuaTable) -> LuaResult<SprotoValue> {
    // Determine if table is an array (1-indexed sequential integers)
    let is_array = is_lua_array(&table)?;

    if is_array {
        // Convert to Array
        let len = table.len()? as i64;
        let mut arr = Vec::with_capacity(len as usize);
        for i in 1..=len {
            let val: LuaValue = table.get(i)?;
            if val == LuaValue::Nil {
                break;
            }
            arr.push(lua_to_sproto_value(lua, val)?);
        }
        Ok(SprotoValue::Array(arr))
    } else {
        // Convert to Struct
        let mut map = HashMap::new();
        for pair in table.pairs::<LuaValue, LuaValue>() {
            let (key, val) = pair?;
            // Skip nil values
            if val == LuaValue::Nil {
                continue;
            }
            // Get string key
            let key_str = match key {
                LuaValue::String(s) => s.to_str()?.to_string(),
                LuaValue::Integer(i) => i.to_string(),
                _ => {
                    return Err(LuaError::RuntimeError(format!(
                        "table key must be string or integer, got {:?}",
                        key.type_name()
                    )))
                }
            };
            map.insert(key_str, lua_to_sproto_value(lua, val)?);
        }
        Ok(SprotoValue::Struct(map))
    }
}

/// Check if a Lua table is an array (1-indexed sequential integers).
fn is_lua_array(table: &LuaTable) -> LuaResult<bool> {
    let len = table.len()? as i64;
    if len == 0 {
        // Empty table - check if it has any string keys
        for pair in table.pairs::<LuaValue, LuaValue>() {
            let (key, _) = pair?;
            match key {
                LuaValue::String(_) => return Ok(false),
                _ => {}
            }
        }
        return Ok(true); // Empty table treated as empty array
    }

    // Check if all keys are sequential integers from 1 to len
    let mut count = 0;
    for pair in table.pairs::<LuaValue, LuaValue>() {
        let (key, _) = pair?;
        match key {
            LuaValue::Integer(i) if i >= 1 && i <= len => {
                count += 1;
            }
            LuaValue::Integer(_) => return Ok(false), // Out of range integer
            LuaValue::String(_) => return Ok(false),  // Has string key
            _ => return Ok(false),
        }
    }

    Ok(count == len)
}

/// Convert SprotoValue to Lua value.
pub fn sproto_value_to_lua(lua: &Lua, value: &SprotoValue) -> LuaResult<LuaValue> {
    match value {
        SprotoValue::Integer(i) => Ok(LuaValue::Integer(*i)),
        SprotoValue::Boolean(b) => Ok(LuaValue::Boolean(*b)),
        SprotoValue::Str(s) => Ok(LuaValue::String(lua.create_string(s)?)),
        SprotoValue::Binary(b) => Ok(LuaValue::String(lua.create_string(b)?)),
        SprotoValue::Double(d) => Ok(LuaValue::Number(*d)),
        SprotoValue::Struct(map) => {
            let table = lua.create_table()?;
            for (key, val) in map {
                table.set(key.as_str(), sproto_value_to_lua(lua, val)?)?;
            }
            Ok(LuaValue::Table(table))
        }
        SprotoValue::Array(arr) => {
            let table = lua.create_table()?;
            for (i, val) in arr.iter().enumerate() {
                // Lua arrays are 1-indexed
                table.set((i + 1) as i64, sproto_value_to_lua(lua, val)?)?;
            }
            Ok(LuaValue::Table(table))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_conversion() {
        let lua = Lua::new();
        let value = lua_to_sproto_value(&lua, LuaValue::Integer(42)).unwrap();
        assert!(matches!(value, SprotoValue::Integer(42)));
    }

    #[test]
    fn test_boolean_conversion() {
        let lua = Lua::new();
        let value = lua_to_sproto_value(&lua, LuaValue::Boolean(true)).unwrap();
        assert!(matches!(value, SprotoValue::Boolean(true)));
    }
}
