use std::collections::HashMap;

use crate::codec;
use crate::error::{DecodeError, RpcError};
use crate::pack;
use crate::types::Sproto;
use crate::value::SprotoValue;

/// RPC dispatch result.
#[derive(Debug)]
pub enum DispatchResult {
    /// An incoming request message.
    Request {
        /// Protocol name.
        name: String,
        /// Decoded request message (may be empty struct if no request type).
        message: SprotoValue,
        /// Responder to create the response packet, if a session is present.
        responder: Option<Box<Responder>>,
        /// Optional user data from the package header.
        ud: Option<i64>,
    },
    /// An incoming response message.
    Response {
        /// Session ID matching the original request.
        session: u64,
        /// Decoded response message, if the protocol has a response type.
        message: Option<SprotoValue>,
        /// Optional user data from the package header.
        ud: Option<i64>,
    },
}

/// A responder that can encode and pack a response message.
#[derive(Debug)]
pub struct Responder {
    sproto: Sproto,
    package_type_idx: usize,
    response_type_idx: Option<usize>,
    session: u64,
}

impl Responder {
    /// Get the session ID for this responder.
    pub fn session(&self) -> u64 {
        self.session
    }

    /// Encode a response and return the packed binary.
    pub fn respond(
        &self,
        message: &SprotoValue,
        ud: Option<i64>,
    ) -> Result<Vec<u8>, RpcError> {
        let package_type = &self.sproto.types_list[self.package_type_idx];

        // Build package header: no type field (indicates response), session present
        let mut header_fields: Vec<(&str, SprotoValue)> = Vec::new();
        header_fields.push(("session", SprotoValue::Integer(self.session as i64)));
        if let Some(ud_val) = ud {
            header_fields.push(("ud", SprotoValue::Integer(ud_val)));
        }
        let header_val = SprotoValue::from_fields(header_fields);
        let header_bin = codec::encode(&self.sproto, package_type, &header_val)?;

        // Encode response content
        let content_bin = if let Some(resp_idx) = self.response_type_idx {
            let resp_type = &self.sproto.types_list[resp_idx];
            codec::encode(&self.sproto, resp_type, message)?
        } else {
            Vec::new()
        };

        // Combine header + content, then pack
        let mut combined = header_bin;
        combined.extend_from_slice(&content_bin);
        Ok(pack::pack(&combined))
    }
}

/// A request sender created by `Host::attach()`.
#[derive(Debug)]
pub struct RequestSender {
    local_sproto: Sproto,
    remote_sproto: Sproto,
    package_type_idx: usize,
    sessions: HashMap<u64, Option<usize>>, // session -> response type index
}

impl RequestSender {
    /// Create and send a request packet.
    ///
    /// - `protocol_name`: name of the protocol to call
    /// - `message`: request data (use empty struct for no-data requests)
    /// - `session`: optional session ID for request-response tracking
    /// - `ud`: optional user data
    pub fn request(
        &mut self,
        protocol_name: &str,
        message: &SprotoValue,
        session: Option<u64>,
        ud: Option<i64>,
    ) -> Result<Vec<u8>, RpcError> {
        let proto = self
            .remote_sproto
            .get_protocol(protocol_name)
            .ok_or_else(|| RpcError::UnknownProtocol(protocol_name.to_string()))?;

        let proto_tag = proto.tag;
        let request_type_idx = proto.request;
        let response_type_idx = proto.response;

        // Build package header
        let package_type = &self.local_sproto.types_list[self.package_type_idx];
        let mut header_fields: Vec<(&str, SprotoValue)> = Vec::new();
        header_fields.push(("type", SprotoValue::Integer(proto_tag as i64)));
        if let Some(s) = session {
            header_fields.push(("session", SprotoValue::Integer(s as i64)));
        }
        if let Some(ud_val) = ud {
            header_fields.push(("ud", SprotoValue::Integer(ud_val)));
        }
        let header_val = SprotoValue::from_fields(header_fields);
        let header_bin = codec::encode(&self.local_sproto, package_type, &header_val)?;

        // Encode request content
        let content_bin = if let Some(req_idx) = request_type_idx {
            let req_type = &self.remote_sproto.types_list[req_idx];
            codec::encode(&self.remote_sproto, req_type, message)?
        } else {
            Vec::new()
        };

        // Store session for response tracking
        if let Some(s) = session {
            self.sessions.insert(s, response_type_idx);
        }

        // Combine and pack
        let mut combined = header_bin;
        combined.extend_from_slice(&content_bin);
        Ok(pack::pack(&combined))
    }
}

/// RPC host endpoint that dispatches incoming messages.
pub struct Host {
    sproto: Sproto,
    package_type_idx: usize,
    sessions: HashMap<u64, Option<usize>>, // session -> response type index
}

impl Host {
    /// Create a new RPC host.
    ///
    /// `package_name` is the name of the package type in the schema (default "package").
    pub fn new(sproto: Sproto, package_name: &str) -> Result<Self, RpcError> {
        let pkg_idx = sproto
            .get_type_index(package_name)
            .ok_or_else(|| RpcError::PackageTypeNotFound(package_name.to_string()))?;
        Ok(Host {
            sproto,
            package_type_idx: pkg_idx,
            sessions: HashMap::new(),
        })
    }

    /// Dispatch an incoming packed binary message.
    pub fn dispatch(&mut self, packed_data: &[u8]) -> Result<DispatchResult, RpcError> {
        // Unpack
        let unpacked = pack::unpack(packed_data)?;

        // Decode package header
        let package_type = &self.sproto.types_list[self.package_type_idx];
        let header = codec::decode(&self.sproto, package_type, &unpacked)?;

        let header_map = header.as_struct().ok_or_else(|| {
            RpcError::Decode(DecodeError::InvalidData("package header is not a struct".into()))
        })?;

        let proto_type = header_map.get("type").and_then(|v| v.as_integer());
        let session = header_map.get("session").and_then(|v| v.as_integer());
        let ud = header_map.get("ud").and_then(|v| v.as_integer());

        // Calculate the header binary size to find content offset
        let header_bin = codec::encode(&self.sproto, package_type, &header)?;
        let content = &unpacked[header_bin.len()..];

        if let Some(proto_tag) = proto_type {
            // REQUEST
            let proto = self
                .sproto
                .get_protocol_by_tag(proto_tag as u16)
                .ok_or_else(|| RpcError::UnknownProtocol(format!("tag {}", proto_tag)))?;

            let proto_name = proto.name.clone();
            let request_type_idx = proto.request;
            let response_type_idx = proto.response;
            let message = if let Some(req_idx) = request_type_idx {
                if !content.is_empty() {
                    let req_type = &self.sproto.types_list[req_idx];
                    codec::decode(&self.sproto, req_type, content)?
                } else {
                    SprotoValue::new_struct()
                }
            } else {
                SprotoValue::new_struct()
            };

            let responder = session.map(|s| Box::new(Responder {
                sproto: self.sproto.clone(),
                package_type_idx: self.package_type_idx,
                response_type_idx,
                session: s as u64,
            }));

            Ok(DispatchResult::Request {
                name: proto_name,
                message,
                responder,
                ud,
            })
        } else {
            // RESPONSE
            let session_id = session.ok_or_else(|| {
                RpcError::Decode(DecodeError::InvalidData("response without session".into()))
            })? as u64;

            let response_type_idx = self
                .sessions
                .remove(&session_id)
                .ok_or(RpcError::UnknownSession(session_id))?;

            let message = if let Some(resp_idx) = response_type_idx {
                if !content.is_empty() {
                    let resp_type = &self.sproto.types_list[resp_idx];
                    Some(codec::decode(&self.sproto, resp_type, content)?)
                } else {
                    None
                }
            } else {
                None
            };

            Ok(DispatchResult::Response {
                session: session_id,
                message,
                ud,
            })
        }
    }

    /// Create a `RequestSender` attached to a remote sproto schema.
    pub fn attach(&self, remote_sproto: Sproto) -> RequestSender {
        RequestSender {
            local_sproto: self.sproto.clone(),
            remote_sproto,
            package_type_idx: self.package_type_idx,
            sessions: HashMap::new(),
        }
    }

    /// Register a session for tracking (used when a response is expected).
    pub fn register_session(&mut self, session: u64, response_type_idx: Option<usize>) {
        self.sessions.insert(session, response_type_idx);
    }
}
