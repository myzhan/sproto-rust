//! Lua userdata implementations for Sproto types.

use mlua::prelude::*;
use sproto::{codec, parser, binary_schema, rpc, Sproto};
use std::sync::Arc;
use std::cell::RefCell;

use crate::conversion::{sproto_value_to_lua, table_to_sproto_value};
use crate::error::{decode_error_to_lua, encode_error_to_lua, parse_error_to_lua, rpc_error_to_lua};

/// Wrapper for Sproto schema object.
pub struct SprotoUserData {
    pub inner: Arc<Sproto>,
}

impl LuaUserData for SprotoUserData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // encode(type_name, table) -> string
        methods.add_method("encode", |lua, this, (type_name, value): (String, LuaTable)| {
            let sproto_type = this
                .inner
                .get_type(&type_name)
                .ok_or_else(|| LuaError::RuntimeError(format!("unknown type: {}", type_name)))?;

            let sproto_value = table_to_sproto_value(lua, value)?;
            let bytes = codec::encode(&this.inner, sproto_type, &sproto_value)
                .map_err(encode_error_to_lua)?;

            lua.create_string(&bytes)
        });

        // decode(type_name, data) -> table
        methods.add_method("decode", |lua, this, (type_name, data): (String, LuaString)| {
            let sproto_type = this
                .inner
                .get_type(&type_name)
                .ok_or_else(|| LuaError::RuntimeError(format!("unknown type: {}", type_name)))?;

            let sproto_value = codec::decode(&this.inner, sproto_type, &data.as_bytes())
                .map_err(decode_error_to_lua)?;

            sproto_value_to_lua(lua, &sproto_value)
        });

        // get_type(name) -> table | nil
        methods.add_method("get_type", |lua, this, name: String| {
            match this.inner.get_type(&name) {
                Some(sproto_type) => {
                    let table = lua.create_table()?;
                    table.set("name", sproto_type.name.as_str())?;
                    
                    let fields_table = lua.create_table()?;
                    for (i, field) in sproto_type.fields.iter().enumerate() {
                        let field_table = lua.create_table()?;
                        field_table.set("name", field.name.as_str())?;
                        field_table.set("tag", field.tag)?;
                        field_table.set("is_array", field.is_array)?;
                        fields_table.set(i + 1, field_table)?;
                    }
                    table.set("fields", fields_table)?;
                    
                    Ok(LuaValue::Table(table))
                }
                None => Ok(LuaValue::Nil),
            }
        });

        // get_protocol(name) -> table | nil
        methods.add_method("get_protocol", |lua, this, name: String| {
            match this.inner.get_protocol(&name) {
                Some(proto) => {
                    let table = lua.create_table()?;
                    table.set("name", proto.name.as_str())?;
                    table.set("tag", proto.tag)?;
                    table.set("confirm", proto.confirm)?;
                    
                    if let Some(req_idx) = proto.request {
                        if let Some(req_type) = this.inner.types_list.get(req_idx) {
                            table.set("request", req_type.name.as_str())?;
                        }
                    }
                    if let Some(resp_idx) = proto.response {
                        if let Some(resp_type) = this.inner.types_list.get(resp_idx) {
                            table.set("response", resp_type.name.as_str())?;
                        }
                    }
                    
                    Ok(LuaValue::Table(table))
                }
                None => Ok(LuaValue::Nil),
            }
        });

        // host(package_name) -> Host
        methods.add_method("host", |_lua, this, package_name: String| {
            let host = rpc::Host::new((*this.inner).clone(), &package_name)
                .map_err(rpc_error_to_lua)?;
            Ok(HostUserData {
                inner: RefCell::new(host),
            })
        });
    }
}

/// Wrapper for RPC Host.
pub struct HostUserData {
    pub inner: RefCell<rpc::Host>,
}

impl LuaUserData for HostUserData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // dispatch(packed_data) -> table
        methods.add_method("dispatch", |lua, this, data: LuaString| {
            let result = this
                .inner
                .borrow_mut()
                .dispatch(&data.as_bytes())
                .map_err(rpc_error_to_lua)?;

            match result {
                rpc::DispatchResult::Request {
                    name,
                    message,
                    responder,
                    ud,
                } => {
                    let table = lua.create_table()?;
                    table.set("type", "request")?;
                    table.set("name", name.as_str())?;
                    table.set("message", sproto_value_to_lua(lua, &message)?)?;
                    
                    if let Some(resp) = responder {
                        table.set("session", resp.session())?;
                        // Store responder for later use
                        table.set("__responder", ResponderUserData { inner: resp })?;
                    }
                    if let Some(u) = ud {
                        table.set("ud", u)?;
                    }
                    
                    Ok(table)
                }
                rpc::DispatchResult::Response { session, message, ud } => {
                    let table = lua.create_table()?;
                    table.set("type", "response")?;
                    table.set("session", session as i64)?;
                    
                    if let Some(msg) = message {
                        table.set("message", sproto_value_to_lua(lua, &msg)?)?;
                    }
                    if let Some(u) = ud {
                        table.set("ud", u)?;
                    }
                    
                    Ok(table)
                }
            }
        });

        // attach(remote_sproto) -> Sender
        methods.add_method("attach", |_lua, this, remote: LuaUserDataRef<SprotoUserData>| {
            let sender = this.inner.borrow().attach((*remote.inner).clone());
            Ok(SenderUserData {
                inner: RefCell::new(sender),
            })
        });

        // register_session(session, response_type_idx)
        methods.add_method("register_session", |_lua, this, (session, type_idx): (u64, Option<usize>)| {
            this.inner.borrow_mut().register_session(session, type_idx);
            Ok(())
        });
    }
}

/// Wrapper for RPC RequestSender.
pub struct SenderUserData {
    pub inner: RefCell<rpc::RequestSender>,
}

impl LuaUserData for SenderUserData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // request(protocol_name, message, session?, ud?) -> string
        methods.add_method(
            "request",
            |lua, this, (protocol_name, message, session, ud): (String, LuaTable, Option<u64>, Option<i64>)| {
                let sproto_value = table_to_sproto_value(lua, message)?;
                let data = this
                    .inner
                    .borrow_mut()
                    .request(&protocol_name, &sproto_value, session, ud)
                    .map_err(rpc_error_to_lua)?;

                lua.create_string(&data)
            },
        );
    }
}

/// Wrapper for RPC Responder.
pub struct ResponderUserData {
    pub inner: Box<rpc::Responder>,
}

impl LuaUserData for ResponderUserData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // respond(message, ud?) -> string
        methods.add_method("respond", |lua, this, (message, ud): (LuaTable, Option<i64>)| {
            let sproto_value = table_to_sproto_value(lua, message)?;
            let data = this
                .inner
                .respond(&sproto_value, ud)
                .map_err(rpc_error_to_lua)?;

            lua.create_string(&data)
        });
    }
}

/// Parse schema text and create Sproto userdata.
pub fn lua_parse(_lua: &Lua, schema_text: String) -> LuaResult<SprotoUserData> {
    let sproto = parser::parse(&schema_text).map_err(parse_error_to_lua)?;
    Ok(SprotoUserData {
        inner: Arc::new(sproto),
    })
}

/// Load binary schema and create Sproto userdata.
pub fn lua_load_binary(_lua: &Lua, data: LuaString) -> LuaResult<SprotoUserData> {
    let sproto = binary_schema::load_binary(&data.as_bytes()).map_err(decode_error_to_lua)?;
    Ok(SprotoUserData {
        inner: Arc::new(sproto),
    })
}
