//! RPC round-trip tests for sproto RPC functionality.
//!
//! These tests verify:
//! - RPC request encoding and dispatching
//! - RPC response encoding and decoding
//! - Request-response round-trip
//! - Various protocol configurations (with/without request/response types)

use sproto::codec::{StructDecoder, StructEncoder};
use sproto::rpc::{DispatchResult, Host};
use sproto::types::{Field, FieldType, Sproto};

/// Create a test RPC schema programmatically.
fn create_rpc_schema() -> Sproto {
    let mut s = Sproto::new();

    let login_req_idx = s.add_type(
        "login_request",
        vec![
            Field::new("username", 0, FieldType::String),
            Field::new("password", 1, FieldType::String),
        ],
    );
    let login_resp_idx = s.add_type(
        "login_response",
        vec![
            Field::new("ok", 0, FieldType::Boolean),
            Field::new("user_id", 1, FieldType::Integer),
            Field::new("message", 2, FieldType::String),
        ],
    );
    let ping_resp_idx = s.add_type(
        "ping_response",
        vec![Field::new("time", 0, FieldType::Integer)],
    );
    let echo_req_idx = s.add_type(
        "echo_request",
        vec![Field::new("data", 0, FieldType::String)],
    );
    let echo_resp_idx = s.add_type(
        "echo_response",
        vec![Field::new("data", 0, FieldType::String)],
    );

    s.add_protocol("login", 1, Some(login_req_idx), Some(login_resp_idx), false);
    s.add_protocol("ping", 2, None, Some(ping_resp_idx), false);
    s.add_protocol("logout", 3, None, None, true);
    s.add_protocol("notify", 4, None, None, false);
    s.add_protocol("echo", 5, Some(echo_req_idx), Some(echo_resp_idx), false);

    s
}

/// Encode a struct using StructEncoder with a closure.
fn encode_struct(
    sproto: &Sproto,
    type_name: &str,
    f: impl FnOnce(&mut StructEncoder) -> Result<(), sproto::error::EncodeError>,
) -> Vec<u8> {
    let st = sproto.get_type(type_name).unwrap();
    let mut buf = Vec::new();
    let mut enc = StructEncoder::new(sproto, st, &mut buf);
    f(&mut enc).unwrap();
    enc.finish();
    buf
}

/// Decode a string field by tag from raw bytes.
fn decode_string(sproto: &Sproto, type_name: &str, data: &[u8], tag: u16) -> Option<String> {
    let st = sproto.get_type(type_name).unwrap();
    let mut dec = StructDecoder::new(sproto, st, data).unwrap();
    while let Some(f) = dec.next_field().unwrap() {
        if f.tag() == tag {
            return Some(f.as_string().unwrap().to_owned());
        }
    }
    None
}

/// Decode an integer field by tag from raw bytes.
fn decode_integer(sproto: &Sproto, type_name: &str, data: &[u8], tag: u16) -> Option<i64> {
    let st = sproto.get_type(type_name).unwrap();
    let mut dec = StructDecoder::new(sproto, st, data).unwrap();
    while let Some(f) = dec.next_field().unwrap() {
        if f.tag() == tag {
            return Some(f.as_integer().unwrap());
        }
    }
    None
}

/// Decode a bool field by tag from raw bytes.
fn decode_bool(sproto: &Sproto, type_name: &str, data: &[u8], tag: u16) -> Option<bool> {
    let st = sproto.get_type(type_name).unwrap();
    let mut dec = StructDecoder::new(sproto, st, data).unwrap();
    while let Some(f) = dec.next_field().unwrap() {
        if f.tag() == tag {
            return Some(f.as_bool().unwrap());
        }
    }
    None
}

// ============================================================================
// RPC Host Creation Tests
// ============================================================================

#[test]
fn test_rpc_host_creation() {
    let sproto = create_rpc_schema();
    let _host = Host::new(sproto);
}

// ============================================================================
// RPC Round-trip Tests (using programmatic schema)
// ============================================================================

#[test]
fn test_rpc_roundtrip_with_request_response() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    // Client encodes and sends login request
    let request_body = encode_struct(&sproto, "login_request", |enc| {
        enc.set_string(0, "alice")?;
        enc.set_string(1, "secret123")?;
        Ok(())
    });
    let request_packet = client_sender
        .request("login", &request_body, Some(1001), None)
        .unwrap();

    // Server receives and dispatches
    server_host.register_session(1001);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request {
            name,
            body,
            responder,
            ud,
        } => {
            assert_eq!(name, "login");
            assert_eq!(
                decode_string(&sproto, "login_request", &body, 0).as_deref(),
                Some("alice")
            );
            assert_eq!(
                decode_string(&sproto, "login_request", &body, 1).as_deref(),
                Some("secret123")
            );
            assert!(responder.is_some());
            assert!(ud.is_none());

            // Server encodes and sends response
            let response_body = encode_struct(&sproto, "login_response", |enc| {
                enc.set_bool(0, true)?;
                enc.set_integer(1, 12345)?;
                enc.set_string(2, "Welcome!")?;
                Ok(())
            });
            let response_packet = responder.unwrap().respond(&response_body, None).unwrap();

            // Client receives response
            let mut client_host = Host::new(sproto.clone());
            client_host.register_session(1001);
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response { session, body, ud } => {
                    assert_eq!(session, 1001);
                    assert!(ud.is_none());
                    assert_eq!(decode_bool(&sproto, "login_response", &body, 0), Some(true));
                    assert_eq!(
                        decode_integer(&sproto, "login_response", &body, 1),
                        Some(12345)
                    );
                    assert_eq!(
                        decode_string(&sproto, "login_response", &body, 2).as_deref(),
                        Some("Welcome!")
                    );
                }
                _ => panic!("expected Response"),
            }
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_rpc_roundtrip_no_request_body() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    // Client sends ping request (no request body)
    let request_packet = client_sender
        .request("ping", &[], Some(2001), None)
        .unwrap();

    // Server receives and dispatches
    server_host.register_session(2001);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request {
            name, responder, ..
        } => {
            assert_eq!(name, "ping");
            assert!(responder.is_some());

            // Server sends response
            let response_body = encode_struct(&sproto, "ping_response", |enc| {
                enc.set_integer(0, 1234567890)?;
                Ok(())
            });
            let response_packet = responder.unwrap().respond(&response_body, None).unwrap();

            // Client receives response
            let mut client_host = Host::new(sproto.clone());
            client_host.register_session(2001);
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response { session, body, .. } => {
                    assert_eq!(session, 2001);
                    assert_eq!(
                        decode_integer(&sproto, "ping_response", &body, 0),
                        Some(1234567890)
                    );
                }
                _ => panic!("expected Response"),
            }
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_rpc_roundtrip_with_user_data() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    // Client sends request with user data
    let request_body = encode_struct(&sproto, "echo_request", |enc| {
        enc.set_string(0, "test echo")?;
        Ok(())
    });
    let request_packet = client_sender
        .request("echo", &request_body, Some(3001), Some(42))
        .unwrap();

    // Server receives
    server_host.register_session(3001);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request {
            name,
            body,
            responder,
            ud,
        } => {
            assert_eq!(name, "echo");
            assert_eq!(
                decode_string(&sproto, "echo_request", &body, 0).as_deref(),
                Some("test echo")
            );
            assert_eq!(ud, Some(42));

            // Server sends response with user data
            let response_body = encode_struct(&sproto, "echo_response", |enc| {
                enc.set_string(0, "echo: test echo")?;
                Ok(())
            });
            let response_packet = responder
                .unwrap()
                .respond(&response_body, Some(99))
                .unwrap();

            // Client receives
            let mut client_host = Host::new(sproto.clone());
            client_host.register_session(3001);
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response { session, body, ud } => {
                    assert_eq!(session, 3001);
                    assert_eq!(ud, Some(99));
                    assert_eq!(
                        decode_string(&sproto, "echo_response", &body, 0).as_deref(),
                        Some("echo: test echo")
                    );
                }
                _ => panic!("expected Response"),
            }
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_rpc_multiple_concurrent_sessions() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    // Send multiple requests
    let body1 = encode_struct(&sproto, "echo_request", |enc| {
        enc.set_string(0, "first")?;
        Ok(())
    });
    let body2 = encode_struct(&sproto, "echo_request", |enc| {
        enc.set_string(0, "second")?;
        Ok(())
    });
    let body3 = encode_struct(&sproto, "echo_request", |enc| {
        enc.set_string(0, "third")?;
        Ok(())
    });

    let req1 = client_sender
        .request("echo", &body1, Some(1), None)
        .unwrap();
    let req2 = client_sender
        .request("echo", &body2, Some(2), None)
        .unwrap();
    let req3 = client_sender
        .request("echo", &body3, Some(3), None)
        .unwrap();

    // Register all sessions
    server_host.register_session(1);
    server_host.register_session(2);
    server_host.register_session(3);

    // Dispatch all requests
    let result1 = server_host.dispatch(&req1).unwrap();
    let result2 = server_host.dispatch(&req2).unwrap();
    let result3 = server_host.dispatch(&req3).unwrap();

    // Verify all are echo requests
    for (result, expected_data) in [(result1, "first"), (result2, "second"), (result3, "third")] {
        match result {
            DispatchResult::Request { name, body, .. } => {
                assert_eq!(name, "echo");
                assert_eq!(
                    decode_string(&sproto, "echo_request", &body, 0).as_deref(),
                    Some(expected_data)
                );
            }
            _ => panic!("expected Request"),
        }
    }
}

#[test]
fn test_rpc_request_without_session() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    // Send notify (one-way, no session)
    let request_packet = client_sender.request("notify", &[], None, None).unwrap();

    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request {
            name, responder, ..
        } => {
            assert_eq!(name, "notify");
            // No session means no responder
            assert!(responder.is_none());
        }
        _ => panic!("expected Request"),
    }
}

// ============================================================================
// Error Case Tests
// ============================================================================

#[test]
fn test_rpc_unknown_protocol() {
    let sproto = create_rpc_schema();
    let host = Host::new(sproto.clone());
    let mut client_sender = host.attach(sproto);

    let result = client_sender.request("nonexistent", &[], None, None);
    assert!(result.is_err());
}

#[test]
fn test_rpc_response_unknown_session() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    // Send a request
    let request_body = encode_struct(&sproto, "echo_request", |enc| {
        enc.set_string(0, "test")?;
        Ok(())
    });
    let request_packet = client_sender
        .request("echo", &request_body, Some(9999), None)
        .unwrap();

    // Register a different session
    server_host.register_session(8888);

    // Dispatch the request to get responder
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { responder, .. } => {
            let response_body = encode_struct(&sproto, "echo_response", |enc| {
                enc.set_string(0, "resp")?;
                Ok(())
            });
            let response_packet = responder.unwrap().respond(&response_body, None).unwrap();

            // Try to dispatch response with wrong session registered
            let mut client_host = Host::new(sproto.clone());
            // Don't register session 9999
            client_host.register_session(1111);

            let result = client_host.dispatch(&response_packet);
            assert!(result.is_err());
        }
        _ => panic!("expected Request"),
    }
}

// ============================================================================
// Protocol Configuration Tests
// ============================================================================

#[test]
fn test_rpc_protocol_with_request_and_response() {
    let sproto = create_rpc_schema();

    let login_proto = sproto.get_protocol("login").unwrap();
    assert_eq!(login_proto.tag, 1);
    assert!(login_proto.request.is_some());
    assert!(login_proto.response.is_some());
    assert!(!login_proto.confirm);
}

#[test]
fn test_rpc_protocol_response_only() {
    let sproto = create_rpc_schema();

    let ping_proto = sproto.get_protocol("ping").unwrap();
    assert_eq!(ping_proto.tag, 2);
    assert!(ping_proto.request.is_none());
    assert!(ping_proto.response.is_some());
}

#[test]
fn test_rpc_protocol_confirm_nil() {
    let sproto = create_rpc_schema();

    let logout_proto = sproto.get_protocol("logout").unwrap();
    assert_eq!(logout_proto.tag, 3);
    assert!(logout_proto.request.is_none());
    assert!(logout_proto.response.is_none());
    assert!(logout_proto.confirm);
}

#[test]
fn test_rpc_protocol_one_way() {
    let sproto = create_rpc_schema();

    let notify_proto = sproto.get_protocol("notify").unwrap();
    assert_eq!(notify_proto.tag, 4);
    assert!(notify_proto.request.is_none());
    assert!(notify_proto.response.is_none());
    assert!(!notify_proto.confirm);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_rpc_large_session_id() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    let large_session = u64::MAX / 2;
    let request_body = encode_struct(&sproto, "echo_request", |enc| {
        enc.set_string(0, "large session")?;
        Ok(())
    });
    let request_packet = client_sender
        .request("echo", &request_body, Some(large_session), None)
        .unwrap();

    server_host.register_session(large_session);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { responder, .. } => {
            let response_body = encode_struct(&sproto, "echo_response", |enc| {
                enc.set_string(0, "resp")?;
                Ok(())
            });
            let response_packet = responder.unwrap().respond(&response_body, None).unwrap();

            let mut client_host = Host::new(sproto.clone());
            client_host.register_session(large_session);
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response { session, .. } => {
                    assert_eq!(session, large_session);
                }
                _ => panic!("expected Response"),
            }
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_rpc_unicode_in_request() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    let unicode_data = "Hello \u{4e16}\u{754c}! \u{1f389} \u{041f}\u{0440}\u{0438}\u{0432}\u{0435}\u{0442} \u{043c}\u{0438}\u{0440}!";
    let request_body = encode_struct(&sproto, "echo_request", |enc| {
        enc.set_string(0, unicode_data)?;
        Ok(())
    });
    let request_packet = client_sender
        .request("echo", &request_body, Some(1), None)
        .unwrap();

    server_host.register_session(1);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { body, .. } => {
            assert_eq!(
                decode_string(&sproto, "echo_request", &body, 0).as_deref(),
                Some(unicode_data)
            );
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_rpc_empty_string_in_request() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    let request_body = encode_struct(&sproto, "echo_request", |enc| {
        enc.set_string(0, "")?;
        Ok(())
    });
    let request_packet = client_sender
        .request("echo", &request_body, Some(1), None)
        .unwrap();

    server_host.register_session(1);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { body, .. } => {
            assert_eq!(
                decode_string(&sproto, "echo_request", &body, 0).as_deref(),
                Some("")
            );
        }
        _ => panic!("expected Request"),
    }
}
