//! Lua userdata implementations for Sproto types.

use mlua::prelude::*;
use sproto::{binary_schema, rpc, Sproto};
use std::cell::RefCell;
use std::sync::Arc;

use crate::error::{decode_error_to_lua, rpc_error_to_lua};
use crate::lua_codec;

/// Wrapper for Sproto schema object.
pub struct SprotoUserData {
    pub inner: Arc<Sproto>,
}

impl LuaUserData for SprotoUserData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        // encode(type_name, table) -> string
        methods.add_method(
            "encode",
            |lua, this, (type_name, value): (String, LuaTable)| {
                let sproto_type = this.inner.get_type(&type_name).ok_or_else(|| {
                    LuaError::RuntimeError(format!("unknown type: {}", type_name))
                })?;

                let bytes = lua_codec::lua_encode(lua, &this.inner, sproto_type, &value)?;
                lua.create_string(&bytes)
            },
        );

        // decode(type_name, data) -> table
        methods.add_method(
            "decode",
            |lua, this, (type_name, data): (String, LuaString)| {
                let sproto_type = this.inner.get_type(&type_name).ok_or_else(|| {
                    LuaError::RuntimeError(format!("unknown type: {}", type_name))
                })?;

                let table = lua_codec::lua_decode(lua, &this.inner, sproto_type, &data.as_bytes())?;
                Ok(LuaValue::Table(table))
            },
        );

        // get_type(name) -> table | nil
        methods.add_method("get_type", |lua, this, name: String| {
            match this.inner.get_type(&name) {
                Some(sproto_type) => {
                    let table = lua.create_table()?;
                    table.set("name", sproto_type.name.as_str())?;

                    let fields_table = lua.create_table()?;
                    for (i, field) in sproto_type.fields.iter().enumerate() {
                        let field_table = lua.create_table()?;
                        field_table.set("name", &*field.name)?;
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

        // host() -> Host
        methods.add_method("host", |_lua, this, ()| {
            let host = rpc::Host::new((*this.inner).clone());
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
                    body,
                    responder,
                    ud,
                } => {
                    let table = lua.create_table()?;
                    table.set("type", "request")?;
                    table.set("name", name.as_str())?;
                    // Return raw body as Lua string for caller to decode
                    table.set("body", lua.create_string(&body)?)?;

                    if let Some(resp) = responder {
                        table.set("session", resp.session())?;
                        table.set("__responder", ResponderUserData { inner: resp })?;
                    }
                    if let Some(u) = ud {
                        table.set("ud", u)?;
                    }

                    Ok(table)
                }
                rpc::DispatchResult::Response { session, body, ud } => {
                    let table = lua.create_table()?;
                    table.set("type", "response")?;
                    table.set("session", session as i64)?;
                    // Return raw body as Lua string for caller to decode
                    table.set("body", lua.create_string(&body)?)?;
                    if let Some(u) = ud {
                        table.set("ud", u)?;
                    }

                    Ok(table)
                }
            }
        });

        // attach(remote_sproto) -> Sender
        methods.add_method(
            "attach",
            |_lua, this, remote: LuaUserDataRef<SprotoUserData>| {
                let sender = this.inner.borrow().attach((*remote.inner).clone());
                Ok(SenderUserData {
                    inner: RefCell::new(sender),
                })
            },
        );

        // register_session(session)
        methods.add_method("register_session", |_lua, this, session: u64| {
            this.inner.borrow_mut().register_session(session);
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
        // request(protocol_name, body, session?, ud?) -> string
        // body is a pre-encoded binary string (use sproto:encode to produce it)
        methods.add_method(
            "request",
            |lua, this, (protocol_name, body, session, ud): (String, LuaString, Option<u64>, Option<i64>)| {
                let data = this
                    .inner
                    .borrow_mut()
                    .request(&protocol_name, &body.as_bytes(), session, ud)
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
        // respond(body, ud?) -> string
        // body is a pre-encoded binary string
        methods.add_method(
            "respond",
            |lua, this, (body, ud): (LuaString, Option<i64>)| {
                let data = this
                    .inner
                    .respond(&body.as_bytes(), ud)
                    .map_err(rpc_error_to_lua)?;

                lua.create_string(&data)
            },
        );
    }
}

/// Load binary schema and create Sproto userdata.
#[allow(clippy::arc_with_non_send_sync)]
pub fn lua_load_binary(_lua: &Lua, data: LuaString) -> LuaResult<SprotoUserData> {
    let sproto = binary_schema::load_binary(&data.as_bytes()).map_err(decode_error_to_lua)?;
    Ok(SprotoUserData {
        inner: Arc::new(sproto),
    })
}
