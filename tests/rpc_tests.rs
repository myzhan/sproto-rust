//! RPC round-trip tests for sproto RPC functionality.
//!
//! These tests verify:
//! - RPC request encoding and dispatching
//! - RPC response encoding and decoding
//! - Request-response round-trip
//! - Various protocol configurations (with/without request/response types)

use sproto::binary_schema;
use sproto::rpc::{DispatchResult, Host};
use sproto::types::{Field, FieldType, Protocol, Sproto, SprotoType};
use sproto::value::SprotoValue;
use std::collections::HashMap;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

fn load_rpc_schema() -> Sproto {
    binary_schema::load_binary(&testdata("rpc_schema.bin")).unwrap()
}

/// Create a test RPC schema programmatically (without external files).
fn create_rpc_schema() -> Sproto {
    // package type for RPC headers
    let package_type = SprotoType {
        name: "package".to_string(),
        fields: vec![
            Field {
                name: "type".to_string(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "session".to_string(),
                tag: 1,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "ud".to_string(),
                tag: 2,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 3,
    };

    // login_request type
    let login_request = SprotoType {
        name: "login_request".to_string(),
        fields: vec![
            Field {
                name: "username".to_string(),
                tag: 0,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "password".to_string(),
                tag: 1,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 2,
    };

    // login_response type
    let login_response = SprotoType {
        name: "login_response".to_string(),
        fields: vec![
            Field {
                name: "ok".to_string(),
                tag: 0,
                field_type: FieldType::Boolean,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "user_id".to_string(),
                tag: 1,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "message".to_string(),
                tag: 2,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 3,
    };

    // ping_response type
    let ping_response = SprotoType {
        name: "ping_response".to_string(),
        fields: vec![Field {
            name: "time".to_string(),
            tag: 0,
            field_type: FieldType::Integer,
            is_array: false,
            key_tag: -1,
            is_map: false,
            decimal_precision: 0,
        }],
        base_tag: 0,
        maxn: 1,
    };

    // echo_request type
    let echo_request = SprotoType {
        name: "echo_request".to_string(),
        fields: vec![Field {
            name: "data".to_string(),
            tag: 0,
            field_type: FieldType::String,
            is_array: false,
            key_tag: -1,
            is_map: false,
            decimal_precision: 0,
        }],
        base_tag: 0,
        maxn: 1,
    };

    // echo_response type
    let echo_response = SprotoType {
        name: "echo_response".to_string(),
        fields: vec![Field {
            name: "data".to_string(),
            tag: 0,
            field_type: FieldType::String,
            is_array: false,
            key_tag: -1,
            is_map: false,
            decimal_precision: 0,
        }],
        base_tag: 0,
        maxn: 1,
    };

    let mut types_by_name = HashMap::new();
    types_by_name.insert("package".to_string(), 0);
    types_by_name.insert("login_request".to_string(), 1);
    types_by_name.insert("login_response".to_string(), 2);
    types_by_name.insert("ping_response".to_string(), 3);
    types_by_name.insert("echo_request".to_string(), 4);
    types_by_name.insert("echo_response".to_string(), 5);

    // Protocols
    let login_proto = Protocol {
        name: "login".to_string(),
        tag: 1,
        request: Some(1),  // login_request
        response: Some(2), // login_response
        confirm: false,
    };

    let ping_proto = Protocol {
        name: "ping".to_string(),
        tag: 2,
        request: None,     // no request body
        response: Some(3), // ping_response
        confirm: false,
    };

    let logout_proto = Protocol {
        name: "logout".to_string(),
        tag: 3,
        request: None,
        response: None, // no response (confirm)
        confirm: true,
    };

    let notify_proto = Protocol {
        name: "notify".to_string(),
        tag: 4,
        request: None,
        response: None, // one-way notification
        confirm: false,
    };

    let echo_proto = Protocol {
        name: "echo".to_string(),
        tag: 5,
        request: Some(4),  // echo_request
        response: Some(5), // echo_response
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
            package_type,
            login_request,
            login_response,
            ping_response,
            echo_request,
            echo_response,
        ],
        types_by_name,
        protocols: vec![login_proto, ping_proto, logout_proto, notify_proto, echo_proto],
        protocols_by_name,
        protocols_by_tag,
    }
}

// ============================================================================
// RPC Host Creation Tests
// ============================================================================

#[test]
fn test_rpc_host_creation() {
    let sproto = create_rpc_schema();
    let host = Host::new(sproto, "package");
    assert!(host.is_ok());
}

#[test]
fn test_rpc_host_creation_invalid_package() {
    let sproto = create_rpc_schema();
    let host = Host::new(sproto, "nonexistent");
    assert!(host.is_err());
}

// ============================================================================
// RPC Request/Response Round-trip Tests (using external fixtures)
// ============================================================================

#[test]
fn test_rpc_dispatch_foobar_request() {
    let sproto = load_rpc_schema();
    let mut host = Host::new(sproto, "package").unwrap();

    let request_data = testdata("rpc_foobar_request.bin");
    let result = host.dispatch(&request_data).unwrap();

    match result {
        DispatchResult::Request {
            name,
            message,
            responder,
            ud: _,
        } => {
            assert_eq!(name, "foobar");
            assert_eq!(message.get("what").unwrap().as_str(), Some("hello"));
            assert!(responder.is_some());
        }
        _ => panic!("expected Request, got Response"),
    }
}

#[test]
fn test_rpc_dispatch_foo_request() {
    let sproto = load_rpc_schema();
    let mut host = Host::new(sproto, "package").unwrap();

    let request_data = testdata("rpc_foo_request.bin");
    let result = host.dispatch(&request_data).unwrap();

    match result {
        DispatchResult::Request {
            name,
            message: _,
            responder,
            ud: _,
        } => {
            assert_eq!(name, "foo");
            // foo has no request type, so message should be empty struct
            assert!(responder.is_some());
        }
        _ => panic!("expected Request, got Response"),
    }
}

#[test]
fn test_rpc_dispatch_bar_request() {
    let sproto = load_rpc_schema();
    let mut host = Host::new(sproto, "package").unwrap();

    let request_data = testdata("rpc_bar_request.bin");
    let result = host.dispatch(&request_data).unwrap();

    match result {
        DispatchResult::Request {
            name,
            message: _,
            responder,
            ud: _,
        } => {
            assert_eq!(name, "bar");
            // bar has response nil, but still has a responder
            assert!(responder.is_some());
        }
        _ => panic!("expected Request, got Response"),
    }
}

// ============================================================================
// RPC Round-trip Tests (using programmatic schema)
// ============================================================================

#[test]
fn test_rpc_roundtrip_with_request_response() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    // Client sends login request
    let request_msg = SprotoValue::from_fields(vec![
        ("username", "alice".into()),
        ("password", "secret123".into()),
    ]);
    let request_packet = client_sender
        .request("login", &request_msg, Some(1001), None)
        .unwrap();

    // Server receives and dispatches
    server_host.register_session(1001, Some(2)); // Register for response tracking
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request {
            name,
            message,
            responder,
            ud,
        } => {
            assert_eq!(name, "login");
            assert_eq!(message.get("username").unwrap().as_str(), Some("alice"));
            assert_eq!(message.get("password").unwrap().as_str(), Some("secret123"));
            assert!(responder.is_some());
            assert!(ud.is_none());

            // Server sends response
            let response_msg = SprotoValue::from_fields(vec![
                ("ok", true.into()),
                ("user_id", 12345i64.into()),
                ("message", "Welcome!".into()),
            ]);
            let response_packet = responder.unwrap().respond(&response_msg, None).unwrap();

            // Client receives response
            let mut client_host = Host::new(sproto.clone(), "package").unwrap();
            client_host.register_session(1001, Some(2));
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response {
                    session,
                    message,
                    ud,
                } => {
                    assert_eq!(session, 1001);
                    assert!(ud.is_none());
                    let msg = message.unwrap();
                    assert_eq!(msg.get("ok").unwrap().as_boolean(), Some(true));
                    assert_eq!(msg.get("user_id").unwrap().as_integer(), Some(12345));
                    assert_eq!(msg.get("message").unwrap().as_str(), Some("Welcome!"));
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
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    // Client sends ping request (no request body)
    let empty_msg = SprotoValue::new_struct();
    let request_packet = client_sender
        .request("ping", &empty_msg, Some(2001), None)
        .unwrap();

    // Server receives and dispatches
    server_host.register_session(2001, Some(3));
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request {
            name, responder, ..
        } => {
            assert_eq!(name, "ping");
            assert!(responder.is_some());

            // Server sends response
            let response_msg = SprotoValue::from_fields(vec![("time", 1234567890i64.into())]);
            let response_packet = responder.unwrap().respond(&response_msg, None).unwrap();

            // Client receives response
            let mut client_host = Host::new(sproto.clone(), "package").unwrap();
            client_host.register_session(2001, Some(3));
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response {
                    session, message, ..
                } => {
                    assert_eq!(session, 2001);
                    let msg = message.unwrap();
                    assert_eq!(msg.get("time").unwrap().as_integer(), Some(1234567890));
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
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    // Client sends request with user data
    let request_msg = SprotoValue::from_fields(vec![("data", "test echo".into())]);
    let request_packet = client_sender
        .request("echo", &request_msg, Some(3001), Some(42))
        .unwrap();

    // Server receives
    server_host.register_session(3001, Some(5));
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request {
            name,
            message,
            responder,
            ud,
        } => {
            assert_eq!(name, "echo");
            assert_eq!(message.get("data").unwrap().as_str(), Some("test echo"));
            assert_eq!(ud, Some(42));

            // Server sends response with user data
            let response_msg = SprotoValue::from_fields(vec![("data", "echo: test echo".into())]);
            let response_packet = responder.unwrap().respond(&response_msg, Some(99)).unwrap();

            // Client receives
            let mut client_host = Host::new(sproto.clone(), "package").unwrap();
            client_host.register_session(3001, Some(5));
            let response_result = client_host.dispatch(&response_packet).unwrap();

            match response_result {
                DispatchResult::Response {
                    session,
                    message,
                    ud,
                } => {
                    assert_eq!(session, 3001);
                    assert_eq!(ud, Some(99));
                    let msg = message.unwrap();
                    assert_eq!(msg.get("data").unwrap().as_str(), Some("echo: test echo"));
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
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    // Send multiple requests
    let req1 = client_sender
        .request(
            "echo",
            &SprotoValue::from_fields(vec![("data", "first".into())]),
            Some(1),
            None,
        )
        .unwrap();

    let req2 = client_sender
        .request(
            "echo",
            &SprotoValue::from_fields(vec![("data", "second".into())]),
            Some(2),
            None,
        )
        .unwrap();

    let req3 = client_sender
        .request(
            "echo",
            &SprotoValue::from_fields(vec![("data", "third".into())]),
            Some(3),
            None,
        )
        .unwrap();

    // Register all sessions
    server_host.register_session(1, Some(5));
    server_host.register_session(2, Some(5));
    server_host.register_session(3, Some(5));

    // Dispatch all requests
    let result1 = server_host.dispatch(&req1).unwrap();
    let result2 = server_host.dispatch(&req2).unwrap();
    let result3 = server_host.dispatch(&req3).unwrap();

    // Verify all are echo requests
    for (result, expected_data) in [
        (result1, "first"),
        (result2, "second"),
        (result3, "third"),
    ] {
        match result {
            DispatchResult::Request { name, message, .. } => {
                assert_eq!(name, "echo");
                assert_eq!(message.get("data").unwrap().as_str(), Some(expected_data));
            }
            _ => panic!("expected Request"),
        }
    }
}

#[test]
fn test_rpc_request_without_session() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    // Send notify (one-way, no session)
    let request_packet = client_sender
        .request("notify", &SprotoValue::new_struct(), None, None)
        .unwrap();

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
    let server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto);

    let result = client_sender.request("nonexistent", &SprotoValue::new_struct(), None, None);
    assert!(result.is_err());
}

#[test]
fn test_rpc_response_unknown_session() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    // Send a request
    let request_msg = SprotoValue::from_fields(vec![("data", "test".into())]);
    let request_packet = client_sender
        .request("echo", &request_msg, Some(9999), None)
        .unwrap();

    // Register a different session
    server_host.register_session(8888, Some(5));

    // Dispatch the request to get responder
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { responder, .. } => {
            let response_packet = responder
                .unwrap()
                .respond(&SprotoValue::from_fields(vec![("data", "resp".into())]), None)
                .unwrap();

            // Try to dispatch response with wrong session registered
            let mut client_host = Host::new(sproto.clone(), "package").unwrap();
            // Don't register session 9999
            client_host.register_session(1111, Some(5));

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
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    let large_session = u64::MAX / 2;
    let request_packet = client_sender
        .request(
            "echo",
            &SprotoValue::from_fields(vec![("data", "large session".into())]),
            Some(large_session),
            None,
        )
        .unwrap();

    server_host.register_session(large_session, Some(5));
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { responder, .. } => {
            let response_packet = responder
                .unwrap()
                .respond(&SprotoValue::from_fields(vec![("data", "resp".into())]), None)
                .unwrap();

            let mut client_host = Host::new(sproto.clone(), "package").unwrap();
            client_host.register_session(large_session, Some(5));
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
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    let unicode_data = "Hello 世界! 🎉 Привет мир!";
    let request_packet = client_sender
        .request(
            "echo",
            &SprotoValue::from_fields(vec![("data", unicode_data.into())]),
            Some(1),
            None,
        )
        .unwrap();

    server_host.register_session(1, Some(5));
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { message, .. } => {
            assert_eq!(message.get("data").unwrap().as_str(), Some(unicode_data));
        }
        _ => panic!("expected Request"),
    }
}

#[test]
fn test_rpc_empty_string_in_request() {
    let sproto = create_rpc_schema();
    let mut server_host = Host::new(sproto.clone(), "package").unwrap();
    let mut client_sender = server_host.attach(sproto.clone());

    let request_packet = client_sender
        .request(
            "echo",
            &SprotoValue::from_fields(vec![("data", "".into())]),
            Some(1),
            None,
        )
        .unwrap();

    server_host.register_session(1, Some(5));
    let dispatch_result = server_host.dispatch(&request_packet).unwrap();

    match dispatch_result {
        DispatchResult::Request { message, .. } => {
            assert_eq!(message.get("data").unwrap().as_str(), Some(""));
        }
        _ => panic!("expected Request"),
    }
}
