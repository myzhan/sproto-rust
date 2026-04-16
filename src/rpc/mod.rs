use std::collections::HashSet;
use std::collections::HashMap;

use crate::codec::wire::*;
use crate::error::{DecodeError, RpcError};
use crate::pack;
use crate::types::Sproto;

/// Decoded package header fields (private).
struct PackageHeader {
    type_tag: Option<i64>,
    session: Option<i64>,
    ud: Option<i64>,
    bytes_consumed: usize,
}

/// RPC dispatch result.
#[derive(Debug)]
pub enum DispatchResult {
    /// An incoming request message.
    Request {
        /// Protocol name.
        name: String,
        /// Raw wire-encoded request body (empty if no request data).
        body: Vec<u8>,
        /// Responder to create the response packet, if a session is present.
        responder: Option<Box<Responder>>,
        /// Optional user data from the package header.
        ud: Option<i64>,
    },
    /// An incoming response message.
    Response {
        /// Session ID matching the original request.
        session: u64,
        /// Raw wire-encoded response body (empty if no response data).
        body: Vec<u8>,
        /// Optional user data from the package header.
        ud: Option<i64>,
    },
}

/// A responder that encodes a response packet with the correct session.
#[derive(Debug)]
pub struct Responder {
    session: u64,
}

impl Responder {
    /// Get the session ID for this responder.
    pub fn session(&self) -> u64 {
        self.session
    }

    /// Build a response packet from an already-encoded body.
    ///
    /// The `body` should be the wire-encoded response message (e.g., produced by
    /// serde or derive encoding). Pass an empty slice if there is no response data.
    pub fn respond(
        &self,
        body: &[u8],
        ud: Option<i64>,
    ) -> Result<Vec<u8>, RpcError> {
        // Response header: no type field (indicates response), session present
        let header = encode_package_header(None, Some(self.session), ud);
        let mut combined = header;
        combined.extend_from_slice(body);
        Ok(pack::pack(&combined))
    }
}

/// A request sender created by `Host::attach()`.
#[derive(Debug)]
pub struct RequestSender {
    remote_sproto: Sproto,
    sessions: HashMap<u64, ()>,
}

impl RequestSender {
    /// Create and send a request packet.
    ///
    /// - `protocol_name`: name of the protocol to call on the remote side
    /// - `body`: wire-encoded request data (pass empty slice for no-data requests)
    /// - `session`: optional session ID for request-response tracking
    /// - `ud`: optional user data
    pub fn request(
        &mut self,
        protocol_name: &str,
        body: &[u8],
        session: Option<u64>,
        ud: Option<i64>,
    ) -> Result<Vec<u8>, RpcError> {
        let proto = self
            .remote_sproto
            .get_protocol(protocol_name)
            .ok_or_else(|| RpcError::UnknownProtocol(protocol_name.to_string()))?;

        let proto_tag = proto.tag;

        // Build request header with protocol type tag
        let header = encode_package_header(Some(proto_tag), session, ud);

        if let Some(s) = session {
            self.sessions.insert(s, ());
        }

        let mut combined = header;
        combined.extend_from_slice(body);
        Ok(pack::pack(&combined))
    }
}

/// RPC host endpoint that dispatches incoming messages.
pub struct Host {
    sproto: Sproto,
    sessions: HashSet<u64>,
}

impl Host {
    /// Create a new RPC host.
    pub fn new(sproto: Sproto) -> Self {
        Host {
            sproto,
            sessions: HashSet::new(),
        }
    }

    /// Dispatch an incoming packed binary message.
    ///
    /// Returns a `DispatchResult` with the raw body bytes. The caller is
    /// responsible for decoding the body using serde or derive traits.
    pub fn dispatch(&mut self, packed_data: &[u8]) -> Result<DispatchResult, RpcError> {
        let unpacked = pack::unpack(packed_data)?;

        // Decode the package header inline
        let header = decode_package_header(&unpacked)?;
        let body = unpacked[header.bytes_consumed..].to_vec();

        if let Some(proto_type) = header.type_tag {
            // REQUEST
            let proto = self
                .sproto
                .get_protocol_by_tag(proto_type as u16)
                .ok_or_else(|| RpcError::UnknownProtocol(format!("tag {}", proto_type)))?;

            let proto_name = proto.name.clone();

            let responder = header.session.map(|s| {
                Box::new(Responder {
                    session: s as u64,
                })
            });

            Ok(DispatchResult::Request {
                name: proto_name,
                body,
                responder,
                ud: header.ud,
            })
        } else {
            // RESPONSE
            let session_id = header.session.ok_or_else(|| {
                RpcError::Decode(DecodeError::InvalidData(
                    "response without session".into(),
                ))
            })? as u64;

            if !self.sessions.remove(&session_id) {
                return Err(RpcError::UnknownSession(session_id));
            }

            Ok(DispatchResult::Response {
                session: session_id,
                body,
                ud: header.ud,
            })
        }
    }

    /// Create a `RequestSender` attached to a remote sproto schema.
    pub fn attach(&self, remote_sproto: Sproto) -> RequestSender {
        RequestSender {
            remote_sproto,
            sessions: HashMap::new(),
        }
    }

    /// Register a session for response tracking.
    pub fn register_session(&mut self, session: u64) {
        self.sessions.insert(session);
    }
}

// ---------------------------------------------------------------------------
// Private: package header wire encoding/decoding
//
// Package header is a fixed-schema struct with three integer fields:
//   type    (tag 0) — protocol tag number
//   session (tag 1) — session ID
//   ud      (tag 2) — user data
// ---------------------------------------------------------------------------

/// Encode a package header directly into sproto wire format.
fn encode_package_header(
    type_tag: Option<u16>,
    session: Option<u64>,
    ud: Option<i64>,
) -> Vec<u8> {
    let mut descriptors: Vec<u16> = Vec::with_capacity(4);
    let mut data_part: Vec<u8> = Vec::new();
    let mut last_tag: i32 = -1;

    if let Some(t) = type_tag {
        push_tag_gap(&mut descriptors, 0, last_tag);
        encode_header_int(t as i64, &mut descriptors, &mut data_part);
        last_tag = 0;
    }

    if let Some(s) = session {
        push_tag_gap(&mut descriptors, 1, last_tag);
        encode_header_int(s as i64, &mut descriptors, &mut data_part);
        last_tag = 1;
    }

    if let Some(u) = ud {
        push_tag_gap(&mut descriptors, 2, last_tag);
        encode_header_int(u, &mut descriptors, &mut data_part);
    }

    // Assemble: [field_count][descriptors...][data_part...]
    let header_size = SIZEOF_HEADER + descriptors.len() * SIZEOF_FIELD;
    let mut buf = vec![0u8; header_size];
    write_u16_le(&mut buf[0..], descriptors.len() as u16);
    for (i, &d) in descriptors.iter().enumerate() {
        write_u16_le(&mut buf[SIZEOF_HEADER + i * SIZEOF_FIELD..], d);
    }
    buf.extend_from_slice(&data_part);
    buf
}

/// Push a skip descriptor if there is a tag gap.
#[inline]
fn push_tag_gap(descriptors: &mut Vec<u16>, target_tag: i32, last_tag: i32) {
    let gap = target_tag - last_tag - 1;
    if gap > 0 {
        descriptors.push(((gap - 1) * 2 + 1) as u16);
    }
}

/// Encode a single integer value into field descriptor + optional data part.
#[inline]
fn encode_header_int(int_val: i64, descriptors: &mut Vec<u16>, data_part: &mut Vec<u8>) {
    let uint_val = int_val as u64;
    let u32_val = uint_val as u32;
    if uint_val == u32_val as u64 && u32_val < 0x7fff {
        // Small non-negative: inline as (value + 1) * 2
        descriptors.push(((u32_val + 1) * 2) as u16);
    } else {
        // Large or negative: data part
        descriptors.push(0);
        let i32_check = int_val as i32;
        if i32_check as i64 == int_val {
            // Fits in 4 bytes
            let start = data_part.len();
            data_part.resize(start + SIZEOF_LENGTH + SIZEOF_INT32, 0);
            write_u32_le(&mut data_part[start..], SIZEOF_INT32 as u32);
            write_u32_le(
                &mut data_part[start + SIZEOF_LENGTH..],
                int_val as u32,
            );
        } else {
            // Needs 8 bytes
            let start = data_part.len();
            data_part.resize(start + SIZEOF_LENGTH + SIZEOF_INT64, 0);
            write_u32_le(&mut data_part[start..], SIZEOF_INT64 as u32);
            write_u64_le(&mut data_part[start + SIZEOF_LENGTH..], uint_val);
        }
    }
}

/// Decode a package header from wire bytes.
///
/// Returns the decoded fields and `bytes_consumed` so the caller can locate
/// the body at `&data[bytes_consumed..]`.
fn decode_package_header(data: &[u8]) -> Result<PackageHeader, DecodeError> {
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

    let mut tag: i32 = -1;
    let mut type_tag: Option<i64> = None;
    let mut session: Option<i64> = None;
    let mut ud: Option<i64> = None;

    for i in 0..fn_count {
        let value = read_u16_le(&field_part[i * SIZEOF_FIELD..]) as i32;
        tag += 1;

        if value & 1 != 0 {
            // Odd value: skip tags
            tag += value / 2;
            continue;
        }

        let decoded_value = value / 2 - 1;

        let int_val = if decoded_value >= 0 {
            decoded_value as i64
        } else {
            // Data in data part
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
            let content =
                &data[data_offset + SIZEOF_LENGTH..data_offset + SIZEOF_LENGTH + dsz];
            data_offset += SIZEOF_LENGTH + dsz;

            if dsz == SIZEOF_INT32 {
                expand64(read_u32_le(content)) as i64
            } else if dsz == SIZEOF_INT64 {
                read_u64_le(content) as i64
            } else {
                return Err(DecodeError::InvalidData(format!(
                    "package header integer has invalid size {}",
                    dsz
                )));
            }
        };

        match tag {
            0 => type_tag = Some(int_val),
            1 => session = Some(int_val),
            2 => ud = Some(int_val),
            _ => {} // ignore unknown fields
        }
    }

    Ok(PackageHeader {
        type_tag,
        session,
        ud,
        bytes_consumed: data_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_request_basic() {
        let encoded = encode_package_header(Some(5), Some(1), None);
        let h = decode_package_header(&encoded).unwrap();
        assert_eq!(h.type_tag, Some(5));
        assert_eq!(h.session, Some(1));
        assert_eq!(h.ud, None);
        assert_eq!(h.bytes_consumed, encoded.len());
    }

    #[test]
    fn test_header_response_with_ud() {
        let encoded = encode_package_header(None, Some(42), Some(-10));
        let h = decode_package_header(&encoded).unwrap();
        assert_eq!(h.type_tag, None);
        assert_eq!(h.session, Some(42));
        assert_eq!(h.ud, Some(-10));
        assert_eq!(h.bytes_consumed, encoded.len());
    }

    #[test]
    fn test_header_large_session() {
        let large: u64 = 0x1_0000_0000;
        let encoded = encode_package_header(Some(1), Some(large), None);
        let h = decode_package_header(&encoded).unwrap();
        assert_eq!(h.type_tag, Some(1));
        assert_eq!(h.session, Some(large as i64));
        assert_eq!(h.bytes_consumed, encoded.len());
    }

    #[test]
    fn test_header_only_type() {
        let encoded = encode_package_header(Some(100), None, None);
        let h = decode_package_header(&encoded).unwrap();
        assert_eq!(h.type_tag, Some(100));
        assert_eq!(h.session, None);
        assert_eq!(h.ud, None);
    }

    #[test]
    fn test_header_all_fields() {
        let encoded = encode_package_header(Some(3), Some(999), Some(42));
        let h = decode_package_header(&encoded).unwrap();
        assert_eq!(h.type_tag, Some(3));
        assert_eq!(h.session, Some(999));
        assert_eq!(h.ud, Some(42));
        assert_eq!(h.bytes_consumed, encoded.len());
    }

    #[test]
    fn test_header_with_body_content() {
        // Verify bytes_consumed correctly separates header from body
        let header = encode_package_header(Some(1), Some(2), None);
        let mut data = header;
        let body = b"hello body";
        data.extend_from_slice(body);

        let h = decode_package_header(&data).unwrap();
        assert_eq!(&data[h.bytes_consumed..], body);
    }

    #[test]
    fn test_header_empty() {
        // Empty header (no fields present)
        let encoded = encode_package_header(None, None, None);
        let h = decode_package_header(&encoded).unwrap();
        assert_eq!(h.type_tag, None);
        assert_eq!(h.session, None);
        assert_eq!(h.ud, None);
        assert_eq!(h.bytes_consumed, encoded.len());
    }

    #[test]
    fn test_header_negative_ud() {
        let encoded = encode_package_header(Some(0), Some(1), Some(-100));
        let h = decode_package_header(&encoded).unwrap();
        assert_eq!(h.type_tag, Some(0));
        assert_eq!(h.session, Some(1));
        assert_eq!(h.ud, Some(-100));
    }

    #[test]
    fn test_header_type_zero() {
        // type=0 is a valid protocol tag
        let encoded = encode_package_header(Some(0), None, None);
        let h = decode_package_header(&encoded).unwrap();
        assert_eq!(h.type_tag, Some(0));
    }
}
