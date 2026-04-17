//! Error conversion utilities for Lua binding.

use mlua::prelude::*;
use sproto::error::{DecodeError, EncodeError, PackError, RpcError};

/// Convert sproto EncodeError to Lua error
pub fn encode_error_to_lua(err: EncodeError) -> LuaError {
    LuaError::RuntimeError(format!("encode error: {}", err))
}

/// Convert sproto DecodeError to Lua error
pub fn decode_error_to_lua(err: DecodeError) -> LuaError {
    LuaError::RuntimeError(format!("decode error: {}", err))
}

/// Convert sproto PackError to Lua error
pub fn pack_error_to_lua(err: PackError) -> LuaError {
    LuaError::RuntimeError(format!("pack error: {}", err))
}

/// Convert sproto RpcError to Lua error
pub fn rpc_error_to_lua(err: RpcError) -> LuaError {
    LuaError::RuntimeError(format!("rpc error: {}", err))
}
