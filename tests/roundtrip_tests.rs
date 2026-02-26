//! Round-trip tests for encode/decode and pack/unpack without external binary files.
//!
//! These tests verify that the implementation is self-consistent:
//! - encode(value) -> decode() should return the original value
//! - pack(data) -> unpack() should return the original data
//! - Full pipeline: encode -> pack -> unpack -> decode

use sproto::codec;
use sproto::pack;
use sproto::types::{Field, FieldType, Sproto, SprotoType};
use sproto::value::SprotoValue;
use std::collections::HashMap;

/// Helper to create a simple Sproto schema for testing.
fn create_test_schema() -> Sproto {
    // Person type with various field types
    let person_type = SprotoType {
        name: "Person".to_string(),
        fields: vec![
            Field {
                name: "name".to_string(),
                tag: 0,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "age".to_string(),
                tag: 1,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "active".to_string(),
                tag: 2,
                field_type: FieldType::Boolean,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "score".to_string(),
                tag: 3,
                field_type: FieldType::Double,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "data".to_string(),
                tag: 4,
                field_type: FieldType::Binary,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 5,
    };

    // Data type for array tests
    let data_type = SprotoType {
        name: "Data".to_string(),
        fields: vec![
            Field {
                name: "numbers".to_string(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "names".to_string(),
                tag: 1,
                field_type: FieldType::String,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "flags".to_string(),
                tag: 2,
                field_type: FieldType::Boolean,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "values".to_string(),
                tag: 3,
                field_type: FieldType::Double,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 4,
    };

    // Nested type for nested struct tests
    let nested_type = SprotoType {
        name: "Nested".to_string(),
        fields: vec![
            Field {
                name: "id".to_string(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "person".to_string(),
                tag: 1,
                field_type: FieldType::Struct(0), // Reference to Person
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "people".to_string(),
                tag: 2,
                field_type: FieldType::Struct(0), // Array of Person
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 3,
    };

    let mut types_by_name = HashMap::new();
    types_by_name.insert("Person".to_string(), 0);
    types_by_name.insert("Data".to_string(), 1);
    types_by_name.insert("Nested".to_string(), 2);

    Sproto {
        types_list: vec![person_type, data_type, nested_type],
        types_by_name,
        protocols: vec![],
        protocols_by_name: HashMap::new(),
        protocols_by_tag: HashMap::new(),
    }
}

// ============================================================================
// Encode/Decode Round-trip Tests
// ============================================================================

#[test]
fn test_roundtrip_simple_string() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let original = SprotoValue::from_fields(vec![("name", "Alice".into())]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    assert_eq!(decoded.get("name").unwrap().as_str(), Some("Alice"));
}

#[test]
fn test_roundtrip_simple_integer() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let original = SprotoValue::from_fields(vec![("age", 42i64.into())]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    assert_eq!(decoded.get("age").unwrap().as_integer(), Some(42));
}

#[test]
fn test_roundtrip_simple_boolean() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    // Test true
    let original = SprotoValue::from_fields(vec![("active", true.into())]);
    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();
    assert_eq!(decoded.get("active").unwrap().as_boolean(), Some(true));

    // Test false
    let original = SprotoValue::from_fields(vec![("active", false.into())]);
    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();
    assert_eq!(decoded.get("active").unwrap().as_boolean(), Some(false));
}

#[test]
fn test_roundtrip_simple_double() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let original = SprotoValue::from_fields(vec![("score", 3.14159f64.into())]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    let score = decoded.get("score").unwrap().as_double().unwrap();
    assert!((score - 3.14159).abs() < 1e-10);
}

#[test]
fn test_roundtrip_simple_binary() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let binary_data = vec![0x01, 0x02, 0x03, 0xFF, 0xFE];
    let original = SprotoValue::from_fields(vec![("data", SprotoValue::Binary(binary_data.clone()))]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    assert_eq!(decoded.get("data").unwrap().as_binary(), Some(binary_data.as_slice()));
}

#[test]
fn test_roundtrip_all_primitive_types() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let original = SprotoValue::from_fields(vec![
        ("name", "Test User".into()),
        ("age", 25i64.into()),
        ("active", true.into()),
        ("score", 98.5f64.into()),
        ("data", SprotoValue::Binary(vec![0xDE, 0xAD, 0xBE, 0xEF])),
    ]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    assert_eq!(decoded.get("name").unwrap().as_str(), Some("Test User"));
    assert_eq!(decoded.get("age").unwrap().as_integer(), Some(25));
    assert_eq!(decoded.get("active").unwrap().as_boolean(), Some(true));
    assert!((decoded.get("score").unwrap().as_double().unwrap() - 98.5).abs() < 1e-10);
    assert_eq!(
        decoded.get("data").unwrap().as_binary(),
        Some([0xDE, 0xAD, 0xBE, 0xEF].as_slice())
    );
}

#[test]
fn test_roundtrip_integer_array() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    let numbers: Vec<SprotoValue> = (1..=10).map(|i| SprotoValue::Integer(i)).collect();
    let original = SprotoValue::from_fields(vec![("numbers", SprotoValue::Array(numbers))]);

    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let decoded = codec::decode(&sproto, data_type, &encoded).unwrap();

    let arr = decoded.get("numbers").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 10);
    for (i, val) in arr.iter().enumerate() {
        assert_eq!(val.as_integer(), Some((i + 1) as i64));
    }
}

#[test]
fn test_roundtrip_large_integers() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    // Test with 64-bit integers that exceed 32-bit range
    let large_values = vec![
        SprotoValue::Integer(1i64 << 33),
        SprotoValue::Integer(-(1i64 << 40)),
        SprotoValue::Integer(i64::MAX),
        SprotoValue::Integer(i64::MIN),
    ];
    let original = SprotoValue::from_fields(vec![("numbers", SprotoValue::Array(large_values.clone()))]);

    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let decoded = codec::decode(&sproto, data_type, &encoded).unwrap();

    let arr = decoded.get("numbers").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0].as_integer(), Some(1i64 << 33));
    assert_eq!(arr[1].as_integer(), Some(-(1i64 << 40)));
    assert_eq!(arr[2].as_integer(), Some(i64::MAX));
    assert_eq!(arr[3].as_integer(), Some(i64::MIN));
}

#[test]
fn test_roundtrip_string_array() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    let names = vec![
        SprotoValue::Str("Alice".to_string()),
        SprotoValue::Str("Bob".to_string()),
        SprotoValue::Str("Charlie".to_string()),
    ];
    let original = SprotoValue::from_fields(vec![("names", SprotoValue::Array(names))]);

    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let decoded = codec::decode(&sproto, data_type, &encoded).unwrap();

    let arr = decoded.get("names").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].as_str(), Some("Alice"));
    assert_eq!(arr[1].as_str(), Some("Bob"));
    assert_eq!(arr[2].as_str(), Some("Charlie"));
}

#[test]
fn test_roundtrip_boolean_array() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    let flags = vec![
        SprotoValue::Boolean(true),
        SprotoValue::Boolean(false),
        SprotoValue::Boolean(true),
        SprotoValue::Boolean(false),
    ];
    let original = SprotoValue::from_fields(vec![("flags", SprotoValue::Array(flags))]);

    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let decoded = codec::decode(&sproto, data_type, &encoded).unwrap();

    let arr = decoded.get("flags").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0].as_boolean(), Some(true));
    assert_eq!(arr[1].as_boolean(), Some(false));
    assert_eq!(arr[2].as_boolean(), Some(true));
    assert_eq!(arr[3].as_boolean(), Some(false));
}

#[test]
fn test_roundtrip_double_array() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    let values = vec![
        SprotoValue::Double(1.1),
        SprotoValue::Double(2.2),
        SprotoValue::Double(3.3),
        SprotoValue::Double(-4.4),
    ];
    let original = SprotoValue::from_fields(vec![("values", SprotoValue::Array(values))]);

    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let decoded = codec::decode(&sproto, data_type, &encoded).unwrap();

    let arr = decoded.get("values").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 4);
    assert!((arr[0].as_double().unwrap() - 1.1).abs() < 1e-10);
    assert!((arr[1].as_double().unwrap() - 2.2).abs() < 1e-10);
    assert!((arr[2].as_double().unwrap() - 3.3).abs() < 1e-10);
    assert!((arr[3].as_double().unwrap() - (-4.4)).abs() < 1e-10);
}

#[test]
fn test_roundtrip_empty_array() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    let original = SprotoValue::from_fields(vec![("numbers", SprotoValue::Array(vec![]))]);

    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let decoded = codec::decode(&sproto, data_type, &encoded).unwrap();

    // Empty arrays are typically not included in the decoded result
    // or decoded as an empty array
    if let Some(arr) = decoded.get("numbers") {
        assert!(arr.as_array().unwrap().is_empty());
    }
}

#[test]
fn test_roundtrip_nested_struct() {
    let sproto = create_test_schema();
    let nested_type = sproto.get_type("Nested").unwrap();

    let person = SprotoValue::from_fields(vec![
        ("name", "Alice".into()),
        ("age", 30i64.into()),
    ]);

    let original = SprotoValue::from_fields(vec![
        ("id", 123i64.into()),
        ("person", person),
    ]);

    let encoded = codec::encode(&sproto, nested_type, &original).unwrap();
    let decoded = codec::decode(&sproto, nested_type, &encoded).unwrap();

    assert_eq!(decoded.get("id").unwrap().as_integer(), Some(123));
    let person = decoded.get("person").unwrap();
    assert_eq!(person.get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(person.get("age").unwrap().as_integer(), Some(30));
}

#[test]
fn test_roundtrip_nested_struct_array() {
    let sproto = create_test_schema();
    let nested_type = sproto.get_type("Nested").unwrap();

    let people = vec![
        SprotoValue::from_fields(vec![
            ("name", "Alice".into()),
            ("age", 25i64.into()),
        ]),
        SprotoValue::from_fields(vec![
            ("name", "Bob".into()),
            ("age", 30i64.into()),
        ]),
    ];

    let original = SprotoValue::from_fields(vec![
        ("id", 456i64.into()),
        ("people", SprotoValue::Array(people)),
    ]);

    let encoded = codec::encode(&sproto, nested_type, &original).unwrap();
    let decoded = codec::decode(&sproto, nested_type, &encoded).unwrap();

    assert_eq!(decoded.get("id").unwrap().as_integer(), Some(456));
    let people = decoded.get("people").unwrap().as_array().unwrap();
    assert_eq!(people.len(), 2);
    assert_eq!(people[0].get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(people[0].get("age").unwrap().as_integer(), Some(25));
    assert_eq!(people[1].get("name").unwrap().as_str(), Some("Bob"));
    assert_eq!(people[1].get("age").unwrap().as_integer(), Some(30));
}

#[test]
fn test_roundtrip_unicode_string() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let unicode_name = "Hello, \u{4e16}\u{754c}! \u{1f600}"; // "Hello, 世界! 😀"
    let original = SprotoValue::from_fields(vec![("name", unicode_name.into())]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    assert_eq!(decoded.get("name").unwrap().as_str(), Some(unicode_name));
}

#[test]
fn test_roundtrip_empty_string() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let original = SprotoValue::from_fields(vec![("name", "".into())]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    assert_eq!(decoded.get("name").unwrap().as_str(), Some(""));
}

#[test]
fn test_roundtrip_zero_integer() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let original = SprotoValue::from_fields(vec![("age", 0i64.into())]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    assert_eq!(decoded.get("age").unwrap().as_integer(), Some(0));
}

#[test]
fn test_roundtrip_negative_integer() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let original = SprotoValue::from_fields(vec![("age", (-12345i64).into())]);

    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();

    assert_eq!(decoded.get("age").unwrap().as_integer(), Some(-12345));
}

// ============================================================================
// Pack/Unpack Round-trip Tests
// ============================================================================

#[test]
fn test_pack_unpack_simple() {
    let data = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];

    let packed = pack::pack(&data);
    let unpacked = pack::unpack(&packed).unwrap();

    // Unpacked may have trailing zeros due to 8-byte alignment
    assert_eq!(&unpacked[..data.len()], &data[..]);
}

#[test]
fn test_pack_unpack_all_zeros() {
    let data = vec![0x00; 32];

    let packed = pack::pack(&data);
    let unpacked = pack::unpack(&packed).unwrap();

    assert_eq!(&unpacked[..data.len()], &data[..]);
}

#[test]
fn test_pack_unpack_all_ff() {
    let data = vec![0xFF; 32];

    let packed = pack::pack(&data);
    let unpacked = pack::unpack(&packed).unwrap();

    assert_eq!(&unpacked[..data.len()], &data[..]);
}

#[test]
fn test_pack_unpack_mixed_data() {
    let data = vec![
        0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04,
        0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, 0xF8,
    ];

    let packed = pack::pack(&data);
    let unpacked = pack::unpack(&packed).unwrap();

    assert_eq!(&unpacked[..data.len()], &data[..]);
}

#[test]
fn test_pack_unpack_empty() {
    let data: Vec<u8> = vec![];

    let packed = pack::pack(&data);
    let unpacked = pack::unpack(&packed).unwrap();

    assert!(unpacked.is_empty());
}

#[test]
fn test_pack_unpack_single_byte() {
    let data = vec![0x42];

    let packed = pack::pack(&data);
    let unpacked = pack::unpack(&packed).unwrap();

    // Due to 8-byte alignment, the result may be longer
    assert_eq!(unpacked[0], 0x42);
}

#[test]
fn test_pack_unpack_large_data() {
    // Test with 1KB of random-ish data
    let data: Vec<u8> = (0..1024).map(|i| (i % 256) as u8).collect();

    let packed = pack::pack(&data);
    let unpacked = pack::unpack(&packed).unwrap();

    assert_eq!(&unpacked[..data.len()], &data[..]);
}

#[test]
fn test_pack_compression() {
    // Data with many zeros should compress well
    let data = vec![0x00; 64];
    let packed = pack::pack(&data);

    // Packed should be smaller than original (or at least not much larger)
    assert!(packed.len() < data.len() + 16);
}

// ============================================================================
// Full Pipeline Tests: encode -> pack -> unpack -> decode
// ============================================================================

#[test]
fn test_full_pipeline_simple() {
    let sproto = create_test_schema();
    let person_type = sproto.get_type("Person").unwrap();

    let original = SprotoValue::from_fields(vec![
        ("name", "Test".into()),
        ("age", 100i64.into()),
    ]);

    // Full pipeline: encode -> pack -> unpack -> decode
    let encoded = codec::encode(&sproto, person_type, &original).unwrap();
    let packed = pack::pack(&encoded);
    let unpacked = pack::unpack(&packed).unwrap();
    let decoded = codec::decode(&sproto, person_type, &unpacked[..encoded.len()]).unwrap();

    assert_eq!(decoded.get("name").unwrap().as_str(), Some("Test"));
    assert_eq!(decoded.get("age").unwrap().as_integer(), Some(100));
}

#[test]
fn test_full_pipeline_complex() {
    let sproto = create_test_schema();
    let nested_type = sproto.get_type("Nested").unwrap();

    let people = vec![
        SprotoValue::from_fields(vec![
            ("name", "Alice".into()),
            ("age", 25i64.into()),
            ("active", true.into()),
        ]),
        SprotoValue::from_fields(vec![
            ("name", "Bob".into()),
            ("age", 30i64.into()),
            ("active", false.into()),
        ]),
    ];

    let original = SprotoValue::from_fields(vec![
        ("id", 999i64.into()),
        ("people", SprotoValue::Array(people)),
    ]);

    // Full pipeline
    let encoded = codec::encode(&sproto, nested_type, &original).unwrap();
    let packed = pack::pack(&encoded);
    let unpacked = pack::unpack(&packed).unwrap();
    let decoded = codec::decode(&sproto, nested_type, &unpacked[..encoded.len()]).unwrap();

    assert_eq!(decoded.get("id").unwrap().as_integer(), Some(999));
    let people = decoded.get("people").unwrap().as_array().unwrap();
    assert_eq!(people.len(), 2);
    assert_eq!(people[0].get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(people[1].get("name").unwrap().as_str(), Some("Bob"));
}

#[test]
fn test_full_pipeline_all_types() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    let original = SprotoValue::from_fields(vec![
        ("numbers", SprotoValue::Array(vec![
            SprotoValue::Integer(1),
            SprotoValue::Integer(2),
            SprotoValue::Integer(3),
        ])),
        ("names", SprotoValue::Array(vec![
            SprotoValue::Str("a".to_string()),
            SprotoValue::Str("b".to_string()),
        ])),
        ("flags", SprotoValue::Array(vec![
            SprotoValue::Boolean(true),
            SprotoValue::Boolean(false),
        ])),
        ("values", SprotoValue::Array(vec![
            SprotoValue::Double(1.5),
            SprotoValue::Double(2.5),
        ])),
    ]);

    // Full pipeline
    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let packed = pack::pack(&encoded);
    let unpacked = pack::unpack(&packed).unwrap();
    let decoded = codec::decode(&sproto, data_type, &unpacked[..encoded.len()]).unwrap();

    let numbers = decoded.get("numbers").unwrap().as_array().unwrap();
    assert_eq!(numbers.len(), 3);

    let names = decoded.get("names").unwrap().as_array().unwrap();
    assert_eq!(names.len(), 2);

    let flags = decoded.get("flags").unwrap().as_array().unwrap();
    assert_eq!(flags.len(), 2);

    let values = decoded.get("values").unwrap().as_array().unwrap();
    assert_eq!(values.len(), 2);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_roundtrip_special_doubles() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    let values = vec![
        SprotoValue::Double(0.0),
        SprotoValue::Double(-0.0),
        SprotoValue::Double(f64::MIN_POSITIVE),
        SprotoValue::Double(f64::MAX),
        SprotoValue::Double(f64::MIN),
    ];
    let original = SprotoValue::from_fields(vec![("values", SprotoValue::Array(values))]);

    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let decoded = codec::decode(&sproto, data_type, &encoded).unwrap();

    let arr = decoded.get("values").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0].as_double().unwrap(), 0.0);
    // -0.0 and 0.0 may be equal in comparison
    assert_eq!(arr[2].as_double().unwrap(), f64::MIN_POSITIVE);
    assert_eq!(arr[3].as_double().unwrap(), f64::MAX);
    assert_eq!(arr[4].as_double().unwrap(), f64::MIN);
}

#[test]
fn test_roundtrip_boundary_integers() {
    let sproto = create_test_schema();
    let data_type = sproto.get_type("Data").unwrap();

    // Test boundary values around 32-bit limits
    let values = vec![
        SprotoValue::Integer(i32::MAX as i64),
        SprotoValue::Integer(i32::MIN as i64),
        SprotoValue::Integer((i32::MAX as i64) + 1),
        SprotoValue::Integer((i32::MIN as i64) - 1),
        SprotoValue::Integer(0x7FFF - 1), // Just below inline threshold
        SprotoValue::Integer(0x7FFF),     // At inline threshold
        SprotoValue::Integer(0x7FFF + 1), // Just above inline threshold
    ];
    let original = SprotoValue::from_fields(vec![("numbers", SprotoValue::Array(values))]);

    let encoded = codec::encode(&sproto, data_type, &original).unwrap();
    let decoded = codec::decode(&sproto, data_type, &encoded).unwrap();

    let arr = decoded.get("numbers").unwrap().as_array().unwrap();
    assert_eq!(arr.len(), 7);
    assert_eq!(arr[0].as_integer(), Some(i32::MAX as i64));
    assert_eq!(arr[1].as_integer(), Some(i32::MIN as i64));
    assert_eq!(arr[2].as_integer(), Some((i32::MAX as i64) + 1));
    assert_eq!(arr[3].as_integer(), Some((i32::MIN as i64) - 1));
}

// ============================================================================
// Serde Round-trip Tests
// ============================================================================

mod serde_tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct SimplePerson {
        name: String,
        age: i64,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct PersonWithOptional {
        name: String,
        age: Option<i64>,
        active: Option<bool>,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct DataArrays {
        numbers: Vec<i64>,
        names: Vec<String>,
    }

    #[test]
    fn test_serde_roundtrip_simple() {
        let sproto = create_test_schema();
        let person_type = sproto.get_type("Person").unwrap();

        let original = SimplePerson {
            name: "Alice".to_string(),
            age: 30,
        };

        let bytes = sproto::serde::to_bytes(&sproto, person_type, &original).unwrap();
        let decoded: SimplePerson = sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_serde_roundtrip_unicode() {
        let sproto = create_test_schema();
        let person_type = sproto.get_type("Person").unwrap();

        let original = SimplePerson {
            name: "你好世界 🌍".to_string(),
            age: 25,
        };

        let bytes = sproto::serde::to_bytes(&sproto, person_type, &original).unwrap();
        let decoded: SimplePerson = sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_serde_roundtrip_with_optional_some() {
        let sproto = create_test_schema();
        let person_type = sproto.get_type("Person").unwrap();

        let original = PersonWithOptional {
            name: "Bob".to_string(),
            age: Some(40),
            active: Some(true),
        };

        let bytes = sproto::serde::to_bytes(&sproto, person_type, &original).unwrap();
        let decoded: PersonWithOptional = sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_serde_roundtrip_with_optional_none() {
        let sproto = create_test_schema();
        let person_type = sproto.get_type("Person").unwrap();

        let original = PersonWithOptional {
            name: "Carol".to_string(),
            age: None,
            active: None,
        };

        let bytes = sproto::serde::to_bytes(&sproto, person_type, &original).unwrap();
        let decoded: PersonWithOptional = sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_serde_roundtrip_arrays() {
        let sproto = create_test_schema();
        let data_type = sproto.get_type("Data").unwrap();

        let original = DataArrays {
            numbers: vec![1, 2, 3, 4, 5],
            names: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        };

        let bytes = sproto::serde::to_bytes(&sproto, data_type, &original).unwrap();
        let decoded: DataArrays = sproto::serde::from_bytes(&sproto, data_type, &bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_serde_to_value_from_value() {
        let original = SimplePerson {
            name: "Test".to_string(),
            age: 99,
        };

        let value = sproto::serde::to_value(&original).unwrap();
        let decoded: SimplePerson = sproto::serde::from_value(&value).unwrap();

        assert_eq!(original, decoded);
    }
}

// ============================================================================
// Derive Macro Round-trip Tests
// ============================================================================

mod derive_tests {
    use sproto::{SprotoDecode, SprotoEncode};

    #[derive(Debug, Clone, PartialEq, SprotoEncode, SprotoDecode)]
    struct DeriveSimple {
        #[sproto(tag = 0)]
        name: String,
        #[sproto(tag = 1)]
        age: i64,
    }

    #[derive(Debug, Clone, PartialEq, SprotoEncode, SprotoDecode)]
    struct DeriveAllTypes {
        #[sproto(tag = 0)]
        str_field: String,
        #[sproto(tag = 1)]
        int_field: i64,
        #[sproto(tag = 2)]
        bool_field: bool,
        #[sproto(tag = 3)]
        double_field: f64,
    }

    #[derive(Debug, Clone, PartialEq, SprotoEncode, SprotoDecode)]
    struct DeriveWithArrays {
        #[sproto(tag = 0)]
        numbers: Vec<i64>,
        #[sproto(tag = 1)]
        values: Vec<f64>,
    }

    #[derive(Debug, Clone, PartialEq, SprotoEncode, SprotoDecode)]
    struct DeriveWithOptional {
        #[sproto(tag = 0)]
        required: String,
        #[sproto(tag = 1)]
        optional_int: Option<i64>,
        #[sproto(tag = 2)]
        optional_str: Option<String>,
    }

    #[derive(Debug, Clone, PartialEq, SprotoEncode, SprotoDecode)]
    struct DeriveNonContiguous {
        #[sproto(tag = 0)]
        first: i64,
        #[sproto(tag = 5)]
        second: String,
        #[sproto(tag = 10)]
        third: bool,
    }

    #[test]
    fn test_derive_roundtrip_simple() {
        let original = DeriveSimple {
            name: "Alice".to_string(),
            age: 30,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveSimple::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_unicode() {
        let original = DeriveSimple {
            name: "こんにちは 🎉".to_string(),
            age: 25,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveSimple::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_all_types() {
        let original = DeriveAllTypes {
            str_field: "test".to_string(),
            int_field: 12345,
            bool_field: true,
            double_field: 3.14159,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveAllTypes::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_arrays() {
        let original = DeriveWithArrays {
            numbers: vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            values: vec![1.1, 2.2, 3.3],
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveWithArrays::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_large_integers() {
        let original = DeriveWithArrays {
            numbers: vec![i64::MAX, i64::MIN, 1i64 << 33, -(1i64 << 40)],
            values: vec![],
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveWithArrays::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_optional_some() {
        let original = DeriveWithOptional {
            required: "test".to_string(),
            optional_int: Some(42),
            optional_str: Some("optional".to_string()),
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveWithOptional::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_optional_none() {
        let original = DeriveWithOptional {
            required: "test".to_string(),
            optional_int: None,
            optional_str: None,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveWithOptional::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_optional_mixed() {
        let original = DeriveWithOptional {
            required: "test".to_string(),
            optional_int: Some(100),
            optional_str: None,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveWithOptional::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_non_contiguous_tags() {
        let original = DeriveNonContiguous {
            first: 1,
            second: "middle".to_string(),
            third: true,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveNonContiguous::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_empty_string() {
        let original = DeriveSimple {
            name: "".to_string(),
            age: 0,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveSimple::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_negative_values() {
        let original = DeriveAllTypes {
            str_field: "negative".to_string(),
            int_field: -99999,
            bool_field: false,
            double_field: -123.456,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveAllTypes::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }
}
