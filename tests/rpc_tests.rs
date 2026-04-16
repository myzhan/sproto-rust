//! RPC round-trip tests for sproto RPC functionality.
//!
//! These tests verify:
//! - RPC request encoding and dispatching
//! - RPC response encoding and decoding
//! - Request-response round-trip
//! - Various protocol configurations (with/without request/response types)

use serde::{Deserialize, Serialize};
use sproto::rpc::{DispatchResult, Host};
use sproto::types::{Field, FieldType, Protocol, Sproto, SprotoType};
use std::collections::HashMap;

/// Create a test RPC schema programmatically (without external files).
fn create_rpc_schema() -> Sproto {
    // login_request type
    let login_request = SprotoType::new(
        "login_request".to_string(),
        vec![
            Field {
                name: "username".into(),
                tag: 0,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "password".into(),
                tag: 1,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        0,
        2,
    );

    // login_response type
    let login_response = SprotoType::new(
        "login_response".to_string(),
        vec![
            Field {
                name: "ok".into(),
                tag: 0,
                field_type: FieldType::Boolean,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "user_id".into(),
                tag: 1,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "message".into(),
                tag: 2,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        0,
        3,
    );

    // ping_response type
    let ping_response = SprotoType::new(
        "ping_response".to_string(),
        vec![Field {
            name: "time".into(),
            tag: 0,
            field_type: FieldType::Integer,
            is_array: false,
            key_tag: -1,
            is_map: false,
            decimal_precision: 0,
        }],
        0,
        1,
    );

    // echo_request type
    let echo_request = SprotoType::new(
        "echo_request".to_string(),
        vec![Field {
            name: "data".into(),
            tag: 0,
            field_type: FieldType::String,
            is_array: false,
            key_tag: -1,
            is_map: false,
            decimal_precision: 0,
        }],
        0,
        1,
    );

    // echo_response type
    let echo_response = SprotoType::new(
        "echo_response".to_string(),
        vec![Field {
            name: "data".into(),
            tag: 0,
            field_type: FieldType::String,
            is_array: false,
            key_tag: -1,
            is_map: false,
            decimal_precision: 0,
        }],
        0,
        1,
    );

    let mut types_by_name = HashMap::new();
    types_by_name.insert("login_request".to_string(), 0);
    types_by_name.insert("login_response".to_string(), 1);
    types_by_name.insert("ping_response".to_string(), 2);
    types_by_name.insert("echo_request".to_string(), 3);
    types_by_name.insert("echo_response".to_string(), 4);

    // Protocols
    let login_proto = Protocol {
        name: "login".into(),
        tag: 1,
        request: Some(0),  // login_request
        response: Some(1), // login_response
        confirm: false,
    };

    let ping_proto = Protocol {
        name: "ping".into(),
        tag: 2,
        request: None,     // no request body
        response: Some(2), // ping_response
        confirm: false,
    };

    let logout_proto = Protocol {
        name: "logout".into(),
        tag: 3,
        request: None,
        response: None, // no response (confirm)
        confirm: true,
    };

    let notify_proto = Protocol {
        name: "notify".into(),
        tag: 4,
        request: None,
        response: None, // one-way notification
        confirm: false,
    };

    let echo_proto = Protocol {
        name: "echo".into(),
        tag: 5,
        request: Some(3),  // echo_request
        response: Some(4), // echo_response
        confirm: false,
    };

    let mut protocols_by_name = HashMap::new();
    protocols_by_name.insert("login".to_string(), 0);
    protocols_by_name.insert("ping".to_string(), 1);
    protocols_by_name.insert("logout".to_string(), 2);
    protocols_by_name.insert("notify".to_string(), 3);
    protocols_by_name.insert("echo".to_string(), 4);

    let mut protocols_by_tag = HashMap::new();
    protocols_by_tag.insert(1, 0);
    protocols_by_tag.insert(2, 1);
    protocols_by_tag.insert(3, 2);
    protocols_by_tag.insert(4, 3);
    protocols_by_tag.insert(5, 4);

    Sproto {
        types_list: vec![
            login_request,
            login_response,
            ping_response,
            echo_request,
            echo_response,
        ],
        types_by_name,
        protocols: vec![
            login_proto,
            ping_proto,
            logout_proto,
            notify_proto,
            echo_proto,
        ],
        protocols_by_name,
        protocols_by_tag,
    }
}

// Serde helpers for encoding/decoding request/response bodies
fn encode_body<T: Serialize>(sproto: &Sproto, type_name: &str, value: &T) -> Vec<u8> {
    let st = sproto.get_type(type_name).unwrap();
    sproto::serde::to_bytes(sproto, st, value).unwrap()
}

fn decode_body<T: for<'de> Deserialize<'de>>(sproto: &Sproto, type_name: &str, data: &[u8]) -> T {
    let st = sproto.get_type(type_name).unwrap();
    sproto::serde::from_bytes(sproto, st, data).unwrap()
}

// Serde structs for the RPC test schema
#[derive(Debug, Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Deserialize)]
struct LoginRequestDec {
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    password: Option<String>,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    ok: bool,
    user_id: i64,
    message: String,
}

#[derive(Debug, Deserialize)]
struct LoginResponseDec {
    #[serde(default)]
    ok: Option<bool>,
    #[serde(default)]
    user_id: Option<i64>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct EchoRequest {
    data: String,
}

#[derive(Debug, Deserialize)]
struct EchoRequestDec {
    #[serde(default)]
    data: Option<String>,
}

#[derive(Debug, Serialize)]
struct EchoResponse {
    data: String,
}

#[derive(Debug, Deserialize)]
struct EchoResponseDec {
    #[serde(default)]
    data: Option<String>,
}

#[derive(Debug, Serialize)]
struct PingResponse {
    time: i64,
}

#[derive(Debug, Deserialize)]
struct PingResponseDec {
    #[serde(default)]
    time: Option<i64>,
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
    let request_body = encode_body(
        &sproto,
        "login_request",
        &LoginRequest {
            username: "alice".into(),
            password: "secret123".into(),
        },
    );
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
            let req: LoginRequestDec = decode_body(&sproto, "login_request", &body);
            assert_eq!(req.username.as_deref(), Some("alice"));
            assert_eq!(req.password.as_deref(), Some("secret123"));
            assert!(responder.is_some());
            assert!(ud.is_none());

            // Server encodes and sends response
            let response_body = encode_body(
                &sproto,
                "login_response",
                &LoginResponse {
                    ok: true,
                    user_id: 12345,
                    message: "Welcome!".into(),
                },
            );
            let response_packet = responder.unwrap().respond(&response_body, None).unwrap();

            // Client receives response
            let mut client_host = Host::new(sproto.clone());
            client_host.register_session(1001);
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response { session, body, ud } => {
                    assert_eq!(session, 1001);
                    assert!(ud.is_none());
                    let resp: LoginResponseDec = decode_body(&sproto, "login_response", &body);
                    assert_eq!(resp.ok, Some(true));
                    assert_eq!(resp.user_id, Some(12345));
                    assert_eq!(resp.message.as_deref(), Some("Welcome!"));
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
            let response_body =
                encode_body(&sproto, "ping_response", &PingResponse { time: 1234567890 });
            let response_packet = responder.unwrap().respond(&response_body, None).unwrap();

            // Client receives response
            let mut client_host = Host::new(sproto.clone());
            client_host.register_session(2001);
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response { session, body, .. } => {
                    assert_eq!(session, 2001);
                    let resp: PingResponseDec = decode_body(&sproto, "ping_response", &body);
                    assert_eq!(resp.time, Some(1234567890));
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
    let request_body = encode_body(
        &sproto,
        "echo_request",
        &EchoRequest {
            data: "test echo".into(),
        },
    );
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
            let req: EchoRequestDec = decode_body(&sproto, "echo_request", &body);
            assert_eq!(req.data.as_deref(), Some("test echo"));
            assert_eq!(ud, Some(42));

            // Server sends response with user data
            let response_body = encode_body(
                &sproto,
                "echo_response",
                &EchoResponse {
                    data: "echo: test echo".into(),
                },
            );
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
                    let resp: EchoResponseDec = decode_body(&sproto, "echo_response", &body);
                    assert_eq!(resp.data.as_deref(), Some("echo: test echo"));
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
    let body1 = encode_body(
        &sproto,
        "echo_request",
        &EchoRequest {
            data: "first".into(),
        },
    );
    let body2 = encode_body(
        &sproto,
        "echo_request",
        &EchoRequest {
            data: "second".into(),
        },
    );
    let body3 = encode_body(
        &sproto,
        "echo_request",
        &EchoRequest {
            data: "third".into(),
        },
    );

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
                let req: EchoRequestDec = decode_body(&sproto, "echo_request", &body);
                assert_eq!(req.data.as_deref(), Some(expected_data));
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
    let request_body = encode_body(
        &sproto,
        "echo_request",
        &EchoRequest {
            data: "test".into(),
        },
    );
    let request_packet = client_sender
        .request("echo", &request_body, Some(9999), None)
        .unwrap();

    // Register a different session
    server_host.register_session(8888);

    // Dispatch the request to get responder
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { responder, .. } => {
            let response_body = encode_body(
                &sproto,
                "echo_response",
                &EchoResponse {
                    data: "resp".into(),
                },
            );
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
    let request_body = encode_body(
        &sproto,
        "echo_request",
        &EchoRequest {
            data: "large session".into(),
        },
    );
    let request_packet = client_sender
        .request("echo", &request_body, Some(large_session), None)
        .unwrap();

    server_host.register_session(large_session);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { responder, .. } => {
            let response_body = encode_body(
                &sproto,
                "echo_response",
                &EchoResponse {
                    data: "resp".into(),
                },
            );
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

    let unicode_data = "Hello 世界! 🎉 Привет мир!";
    let request_body = encode_body(
        &sproto,
        "echo_request",
        &EchoRequest {
            data: unicode_data.into(),
        },
    );
    let request_packet = client_sender
        .request("echo", &request_body, Some(1), None)
        .unwrap();

    server_host.register_session(1);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { body, .. } => {
            let req: EchoRequestDec = decode_body(&sproto, "echo_request", &body);
            assert_eq!(req.data.as_deref(), Some(unicode_data));
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_rpc_empty_string_in_request() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone());
    let mut client_sender = server_host.attach(sproto.clone());

    let request_body = encode_body(&sproto, "echo_request", &EchoRequest { data: "".into() });
    let request_packet = client_sender
        .request("echo", &request_body, Some(1), None)
        .unwrap();

    server_host.register_session(1);
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { body, .. } => {
            let req: EchoRequestDec = decode_body(&sproto, "echo_request", &body);
            assert_eq!(req.data.as_deref(), Some(""));
        }
        _ => panic!("expected Request"),
    }
}
