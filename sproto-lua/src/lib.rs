//! Lua bindings for sproto-rust.
//!
//! This crate provides Lua 5.4 bindings for the sproto serialization library.
//!
//! Schemas are loaded from binary format using `sproto.load_binary(data)`.

mod error;
mod lua_codec;
mod userdata;

use mlua::prelude::*;
use sproto::pack;

use error::pack_error_to_lua;
use userdata::lua_load_binary;

/// Pack data (zero-byte compression).
fn lua_pack(lua: &Lua, data: LuaString) -> LuaResult<LuaString> {
    let packed = pack::pack(&data.as_bytes());
    lua.create_string(&packed)
}

/// Unpack data (decompress).
fn lua_unpack(lua: &Lua, data: LuaString) -> LuaResult<LuaString> {
    let unpacked = pack::unpack(&data.as_bytes()).map_err(pack_error_to_lua)?;
    lua.create_string(&unpacked)
}

/// Lua module entry point.
#[mlua::lua_module]
fn sproto_lua(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;

    // Global functions
    exports.set(
        "load_binary",
        lua.create_function(|lua, data: LuaString| lua_load_binary(lua, data))?,
    )?;

    exports.set(
        "pack",
        lua.create_function(|lua, data: LuaString| lua_pack(lua, data))?,
    )?;

    exports.set(
        "unpack",
        lua.create_function(|lua, data: LuaString| lua_unpack(lua, data))?,
    )?;

    // Version info
    exports.set("_VERSION", "0.1.0")?;

    Ok(exports)
}
