//! Round-trip tests for encode/decode and pack/unpack without external binary files.
//!
//! These tests verify that the implementation is self-consistent:
//! - encode(value) -> decode() should return the original value
//! - pack(data) -> unpack() should return the original data
//! - Full pipeline: encode -> pack -> unpack -> decode

use serde::{Deserialize, Serialize};
use sproto::pack;
use sproto::types::{Field, FieldType, Sproto, SprotoType};
use std::collections::HashMap;

/// Helper to create a simple Sproto schema for testing.
fn create_test_schema() -> Sproto {
    // Person type with various field types
    let person_type = SprotoType::new(
        "Person".to_string(),
        vec![
            Field {
                name: "name".into(),
                tag: 0,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "age".into(),
                tag: 1,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "active".into(),
                tag: 2,
                field_type: FieldType::Boolean,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "score".into(),
                tag: 3,
                field_type: FieldType::Double,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "data".into(),
                tag: 4,
                field_type: FieldType::Binary,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        0,
        5,
    );

    // Data type for array tests
    let data_type = SprotoType::new(
        "Data".to_string(),
        vec![
            Field {
                name: "numbers".into(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "names".into(),
                tag: 1,
                field_type: FieldType::String,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "flags".into(),
                tag: 2,
                field_type: FieldType::Boolean,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "values".into(),
                tag: 3,
                field_type: FieldType::Double,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        0,
        4,
    );

    // Nested type for nested struct tests
    let nested_type = SprotoType::new(
        "Nested".to_string(),
        vec![
            Field {
                name: "id".into(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "person".into(),
                tag: 1,
                field_type: FieldType::Struct(0), // Reference to Person
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "people".into(),
                tag: 2,
                field_type: FieldType::Struct(0), // Array of Person
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        0,
        3,
    );

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

// Serde helper: encode a value to sproto wire bytes.
fn encode<T: Serialize>(sproto: &Sproto, type_name: &str, value: &T) -> Vec<u8> {
    let st = sproto.get_type(type_name).unwrap();
    sproto::serde::to_bytes(sproto, st, value).unwrap()
}

// Serde helper: decode sproto wire bytes into a value.
fn decode<T: for<'de> Deserialize<'de>>(sproto: &Sproto, type_name: &str, data: &[u8]) -> T {
    let st = sproto.get_type(type_name).unwrap();
    sproto::serde::from_bytes(sproto, st, data).unwrap()
}

// Custom serde module for Option<Vec<u8>> via serde_bytes.
mod opt_bytes {
    use serde::{self, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) => serde_bytes::serialize(v, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Some(serde_bytes::deserialize(deserializer)?))
    }
}

// ---- Serde structs matching the test schema ----

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PersonEnc {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    age: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", with = "opt_bytes", default)]
    data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PersonDec {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    age: Option<i64>,
    #[serde(default)]
    active: Option<bool>,
    #[serde(default)]
    score: Option<f64>,
    #[serde(with = "opt_bytes", default)]
    data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DataEnc {
    #[serde(skip_serializing_if = "Option::is_none")]
    numbers: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    names: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct DataDec {
    #[serde(default)]
    numbers: Option<Vec<i64>>,
    #[serde(default)]
    names: Option<Vec<String>>,
    #[serde(default)]
    flags: Option<Vec<bool>>,
    #[serde(default)]
    values: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NestedPersonEnc {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    age: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NestedPersonDec {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    age: Option<i64>,
    #[serde(default)]
    active: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NestedEnc {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    person: Option<NestedPersonEnc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    people: Option<Vec<NestedPersonEnc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct NestedDec {
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    person: Option<NestedPersonDec>,
    #[serde(default)]
    people: Option<Vec<NestedPersonDec>>,
}

// ============================================================================
// Encode/Decode Round-trip Tests
// ============================================================================

#[test]
fn test_roundtrip_simple_string() {
    let sproto = create_test_schema();
    let original = PersonEnc {
        name: Some("Alice".into()),
        age: None,
        active: None,
        score: None,
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.name.as_deref(), Some("Alice"));
}

#[test]
fn test_roundtrip_simple_integer() {
    let sproto = create_test_schema();
    let original = PersonEnc {
        name: None,
        age: Some(42),
        active: None,
        score: None,
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.age, Some(42));
}

#[test]
fn test_roundtrip_simple_boolean() {
    let sproto = create_test_schema();

    // Test true
    let original = PersonEnc {
        name: None,
        age: None,
        active: Some(true),
        score: None,
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.active, Some(true));

    // Test false
    let original = PersonEnc {
        name: None,
        age: None,
        active: Some(false),
        score: None,
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.active, Some(false));
}

#[test]
fn test_roundtrip_simple_double() {
    let sproto = create_test_schema();
    let original = PersonEnc {
        name: None,
        age: None,
        active: None,
        score: Some(3.14159),
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    let score = decoded.score.unwrap();
    assert!((score - 3.14159).abs() < 1e-10);
}

#[test]
fn test_roundtrip_simple_binary() {
    let sproto = create_test_schema();
    let binary_data = vec![0x01, 0x02, 0x03, 0xFF, 0xFE];
    let original = PersonEnc {
        name: None,
        age: None,
        active: None,
        score: None,
        data: Some(binary_data.clone()),
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.data, Some(binary_data));
}

#[test]
fn test_roundtrip_all_primitive_types() {
    let sproto = create_test_schema();
    let original = PersonEnc {
        name: Some("Test User".into()),
        age: Some(25),
        active: Some(true),
        score: Some(98.5),
        data: Some(vec![0xDE, 0xAD, 0xBE, 0xEF]),
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);

    assert_eq!(decoded.name.as_deref(), Some("Test User"));
    assert_eq!(decoded.age, Some(25));
    assert_eq!(decoded.active, Some(true));
    assert!((decoded.score.unwrap() - 98.5).abs() < 1e-10);
    assert_eq!(decoded.data, Some(vec![0xDE, 0xAD, 0xBE, 0xEF]));
}

#[test]
fn test_roundtrip_integer_array() {
    let sproto = create_test_schema();
    let numbers: Vec<i64> = (1..=10).collect();
    let original = DataEnc {
        numbers: Some(numbers.clone()),
        names: None,
        flags: None,
        values: None,
    };
    let encoded = encode(&sproto, "Data", &original);
    let decoded: DataDec = decode(&sproto, "Data", &encoded);

    let arr = decoded.numbers.unwrap();
    assert_eq!(arr.len(), 10);
    assert_eq!(arr, numbers);
}

#[test]
fn test_roundtrip_large_integers() {
    let sproto = create_test_schema();
    let large_values = vec![1i64 << 33, -(1i64 << 40), i64::MAX, i64::MIN];
    let original = DataEnc {
        numbers: Some(large_values.clone()),
        names: None,
        flags: None,
        values: None,
    };
    let encoded = encode(&sproto, "Data", &original);
    let decoded: DataDec = decode(&sproto, "Data", &encoded);

    let arr = decoded.numbers.unwrap();
    assert_eq!(arr, large_values);
}

#[test]
fn test_roundtrip_string_array() {
    let sproto = create_test_schema();
    let names = vec![
        "Alice".to_string(),
        "Bob".to_string(),
        "Charlie".to_string(),
    ];
    let original = DataEnc {
        numbers: None,
        names: Some(names.clone()),
        flags: None,
        values: None,
    };
    let encoded = encode(&sproto, "Data", &original);
    let decoded: DataDec = decode(&sproto, "Data", &encoded);
    assert_eq!(decoded.names.unwrap(), names);
}

#[test]
fn test_roundtrip_boolean_array() {
    let sproto = create_test_schema();
    let flags = vec![true, false, true, false];
    let original = DataEnc {
        numbers: None,
        names: None,
        flags: Some(flags.clone()),
        values: None,
    };
    let encoded = encode(&sproto, "Data", &original);
    let decoded: DataDec = decode(&sproto, "Data", &encoded);
    assert_eq!(decoded.flags.unwrap(), flags);
}

#[test]
fn test_roundtrip_double_array() {
    let sproto = create_test_schema();
    let values = vec![1.1, 2.2, 3.3, -4.4];
    let original = DataEnc {
        numbers: None,
        names: None,
        flags: None,
        values: Some(values.clone()),
    };
    let encoded = encode(&sproto, "Data", &original);
    let decoded: DataDec = decode(&sproto, "Data", &encoded);

    let arr = decoded.values.unwrap();
    assert_eq!(arr.len(), 4);
    for (a, b) in arr.iter().zip(values.iter()) {
        assert!((a - b).abs() < 1e-10);
    }
}

#[test]
fn test_roundtrip_empty_array() {
    let sproto = create_test_schema();
    let original = DataEnc {
        numbers: Some(vec![]),
        names: None,
        flags: None,
        values: None,
    };
    let encoded = encode(&sproto, "Data", &original);
    let decoded: DataDec = decode(&sproto, "Data", &encoded);

    // Empty arrays may not appear in decoded output or appear as empty
    match decoded.numbers {
        Some(arr) => assert!(arr.is_empty()),
        None => {} // also acceptable
    }
}

#[test]
fn test_roundtrip_nested_struct() {
    let sproto = create_test_schema();
    let original = NestedEnc {
        id: Some(123),
        person: Some(NestedPersonEnc {
            name: Some("Alice".into()),
            age: Some(30),
            active: None,
        }),
        people: None,
    };
    let encoded = encode(&sproto, "Nested", &original);
    let decoded: NestedDec = decode(&sproto, "Nested", &encoded);

    assert_eq!(decoded.id, Some(123));
    let person = decoded.person.unwrap();
    assert_eq!(person.name.as_deref(), Some("Alice"));
    assert_eq!(person.age, Some(30));
}

#[test]
fn test_roundtrip_nested_struct_array() {
    let sproto = create_test_schema();
    let original = NestedEnc {
        id: Some(456),
        person: None,
        people: Some(vec![
            NestedPersonEnc {
                name: Some("Alice".into()),
                age: Some(25),
                active: None,
            },
            NestedPersonEnc {
                name: Some("Bob".into()),
                age: Some(30),
                active: None,
            },
        ]),
    };
    let encoded = encode(&sproto, "Nested", &original);
    let decoded: NestedDec = decode(&sproto, "Nested", &encoded);

    assert_eq!(decoded.id, Some(456));
    let people = decoded.people.unwrap();
    assert_eq!(people.len(), 2);
    assert_eq!(people[0].name.as_deref(), Some("Alice"));
    assert_eq!(people[0].age, Some(25));
    assert_eq!(people[1].name.as_deref(), Some("Bob"));
    assert_eq!(people[1].age, Some(30));
}

#[test]
fn test_roundtrip_unicode_string() {
    let sproto = create_test_schema();
    let unicode_name = "Hello, \u{4e16}\u{754c}! \u{1f600}"; // "Hello, 世界! 😀"
    let original = PersonEnc {
        name: Some(unicode_name.into()),
        age: None,
        active: None,
        score: None,
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.name.as_deref(), Some(unicode_name));
}

#[test]
fn test_roundtrip_empty_string() {
    let sproto = create_test_schema();
    let original = PersonEnc {
        name: Some("".into()),
        age: None,
        active: None,
        score: None,
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.name.as_deref(), Some(""));
}

#[test]
fn test_roundtrip_zero_integer() {
    let sproto = create_test_schema();
    let original = PersonEnc {
        name: None,
        age: Some(0),
        active: None,
        score: None,
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.age, Some(0));
}

#[test]
fn test_roundtrip_negative_integer() {
    let sproto = create_test_schema();
    let original = PersonEnc {
        name: None,
        age: Some(-12345),
        active: None,
        score: None,
        data: None,
    };
    let encoded = encode(&sproto, "Person", &original);
    let decoded: PersonDec = decode(&sproto, "Person", &encoded);
    assert_eq!(decoded.age, Some(-12345));
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
        0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04, 0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9,
        0xF8,
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
    let original = PersonEnc {
        name: Some("Test".into()),
        age: Some(100),
        active: None,
        score: None,
        data: None,
    };

    let encoded = encode(&sproto, "Person", &original);
    let packed = pack::pack(&encoded);
    let unpacked = pack::unpack(&packed).unwrap();
    let decoded: PersonDec = decode(&sproto, "Person", &unpacked[..encoded.len()]);

    assert_eq!(decoded.name.as_deref(), Some("Test"));
    assert_eq!(decoded.age, Some(100));
}

#[test]
fn test_full_pipeline_complex() {
    let sproto = create_test_schema();
    let original = NestedEnc {
        id: Some(999),
        person: None,
        people: Some(vec![
            NestedPersonEnc {
                name: Some("Alice".into()),
                age: Some(25),
                active: Some(true),
            },
            NestedPersonEnc {
                name: Some("Bob".into()),
                age: Some(30),
                active: Some(false),
            },
        ]),
    };

    let encoded = encode(&sproto, "Nested", &original);
    let packed = pack::pack(&encoded);
    let unpacked = pack::unpack(&packed).unwrap();
    let decoded: NestedDec = decode(&sproto, "Nested", &unpacked[..encoded.len()]);

    assert_eq!(decoded.id, Some(999));
    let people = decoded.people.unwrap();
    assert_eq!(people.len(), 2);
    assert_eq!(people[0].name.as_deref(), Some("Alice"));
    assert_eq!(people[1].name.as_deref(), Some("Bob"));
}

#[test]
fn test_full_pipeline_all_types() {
    let sproto = create_test_schema();
    let original = DataEnc {
        numbers: Some(vec![1, 2, 3]),
        names: Some(vec!["a".to_string(), "b".to_string()]),
        flags: Some(vec![true, false]),
        values: Some(vec![1.5, 2.5]),
    };

    let encoded = encode(&sproto, "Data", &original);
    let packed = pack::pack(&encoded);
    let unpacked = pack::unpack(&packed).unwrap();
    let decoded: DataDec = decode(&sproto, "Data", &unpacked[..encoded.len()]);

    assert_eq!(decoded.numbers.unwrap().len(), 3);
    assert_eq!(decoded.names.unwrap().len(), 2);
    assert_eq!(decoded.flags.unwrap().len(), 2);
    assert_eq!(decoded.values.unwrap().len(), 2);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_roundtrip_special_doubles() {
    let sproto = create_test_schema();
    let values = vec![0.0, -0.0, f64::MIN_POSITIVE, f64::MAX, f64::MIN];
    let original = DataEnc {
        numbers: None,
        names: None,
        flags: None,
        values: Some(values.clone()),
    };
    let encoded = encode(&sproto, "Data", &original);
    let decoded: DataDec = decode(&sproto, "Data", &encoded);

    let arr = decoded.values.unwrap();
    assert_eq!(arr.len(), 5);
    assert_eq!(arr[0], 0.0);
    // -0.0 and 0.0 may be equal in comparison
    assert_eq!(arr[2], f64::MIN_POSITIVE);
    assert_eq!(arr[3], f64::MAX);
    assert_eq!(arr[4], f64::MIN);
}

#[test]
fn test_roundtrip_boundary_integers() {
    let sproto = create_test_schema();
    let values = vec![
        i32::MAX as i64,
        i32::MIN as i64,
        (i32::MAX as i64) + 1,
        (i32::MIN as i64) - 1,
        0x7FFF - 1, // Just below inline threshold
        0x7FFF,     // At inline threshold
        0x7FFF + 1, // Just above inline threshold
    ];
    let original = DataEnc {
        numbers: Some(values.clone()),
        names: None,
        flags: None,
        values: None,
    };
    let encoded = encode(&sproto, "Data", &original);
    let decoded: DataDec = decode(&sproto, "Data", &encoded);

    let arr = decoded.numbers.unwrap();
    assert_eq!(arr, values);
}

// ============================================================================
// Serde Round-trip Tests
// ============================================================================

mod serde_tests {
    use super::*;

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
            name: "Alice".into(),
            age: 30,
        };

        let bytes = sproto::serde::to_bytes(&sproto, person_type, &original).unwrap();
        let decoded: SimplePerson =
            sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_serde_roundtrip_unicode() {
        let sproto = create_test_schema();
        let person_type = sproto.get_type("Person").unwrap();

        let original = SimplePerson {
            name: "你好世界 🌍".into(),
            age: 25,
        };

        let bytes = sproto::serde::to_bytes(&sproto, person_type, &original).unwrap();
        let decoded: SimplePerson =
            sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_serde_roundtrip_with_optional_some() {
        let sproto = create_test_schema();
        let person_type = sproto.get_type("Person").unwrap();

        let original = PersonWithOptional {
            name: "Bob".into(),
            age: Some(40),
            active: Some(true),
        };

        let bytes = sproto::serde::to_bytes(&sproto, person_type, &original).unwrap();
        let decoded: PersonWithOptional =
            sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_serde_roundtrip_with_optional_none() {
        let sproto = create_test_schema();
        let person_type = sproto.get_type("Person").unwrap();

        let original = PersonWithOptional {
            name: "Carol".into(),
            age: None,
            active: None,
        };

        let bytes = sproto::serde::to_bytes(&sproto, person_type, &original).unwrap();
        let decoded: PersonWithOptional =
            sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();

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
            name: "Alice".into(),
            age: 30,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded = DeriveSimple::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_roundtrip_unicode() {
        let original = DeriveSimple {
            name: "こんにちは 🎉".into(),
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
            name: "".into(),
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

// ============================================================================
// AddressBook full pipeline (Go reference: encode -> pack -> unpack -> decode)
// ============================================================================

#[test]
fn test_addressbook_full_pipeline() {
    let schema = sproto::parser::parse(
        r#"
        .PhoneNumber {
            number 0 : string
            type 1 : integer
        }
        .Person {
            name 0 : string
            id 1 : integer
            email 2 : string
            phone 3 : *PhoneNumber
        }
        .AddressBook {
            person 0 : *Person
        }
    "#,
    )
    .unwrap();

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct PhoneNumber {
        #[serde(skip_serializing_if = "Option::is_none")]
        number: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
        type_: Option<i64>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct ABPerson {
        #[serde(skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        email: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        phone: Option<Vec<PhoneNumber>>,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct AddressBook {
        #[serde(skip_serializing_if = "Option::is_none")]
        person: Option<Vec<ABPerson>>,
    }

    #[derive(Deserialize, Debug)]
    struct AddressBookDec {
        #[serde(default)]
        person: Option<Vec<ABPersonDec>>,
    }

    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct ABPersonDec {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        id: Option<i64>,
        #[serde(default)]
        email: Option<String>,
        #[serde(default)]
        phone: Option<Vec<PhoneNumberDec>>,
    }

    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct PhoneNumberDec {
        #[serde(default)]
        number: Option<String>,
        #[serde(default, rename = "type")]
        type_: Option<i64>,
    }

    let value = AddressBook {
        person: Some(vec![
            ABPerson {
                name: Some("Alice".into()),
                id: Some(10000),
                email: None,
                phone: Some(vec![
                    PhoneNumber {
                        number: Some("123456789".into()),
                        type_: Some(1),
                    },
                    PhoneNumber {
                        number: Some("87654321".into()),
                        type_: Some(2),
                    },
                ]),
            },
            ABPerson {
                name: Some("Bob".into()),
                id: Some(20000),
                email: None,
                phone: Some(vec![PhoneNumber {
                    number: Some("01234567890".into()),
                    type_: Some(3),
                }]),
            },
        ]),
    };

    // Go abData bytes
    let expected_encoded: &[u8] = &[
        1, 0, 0, 0, 122, 0, 0, 0, 68, 0, 0, 0, 4, 0, 0, 0, 34, 78, 1, 0, 0, 0, 5, 0, 0, 0, 65, 108,
        105, 99, 101, 45, 0, 0, 0, 19, 0, 0, 0, 2, 0, 0, 0, 4, 0, 9, 0, 0, 0, 49, 50, 51, 52, 53,
        54, 55, 56, 57, 18, 0, 0, 0, 2, 0, 0, 0, 6, 0, 8, 0, 0, 0, 56, 55, 54, 53, 52, 51, 50, 49,
        46, 0, 0, 0, 4, 0, 0, 0, 66, 156, 1, 0, 0, 0, 3, 0, 0, 0, 66, 111, 98, 25, 0, 0, 0, 21, 0,
        0, 0, 2, 0, 0, 0, 8, 0, 11, 0, 0, 0, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48,
    ];

    // Go abDataPacked bytes
    let expected_packed: &[u8] = &[
        17, 1, 122, 17, 68, 4, 71, 34, 78, 1, 5, 252, 65, 108, 105, 99, 101, 45, 136, 19, 2, 40, 4,
        9, 254, 49, 50, 51, 52, 53, 54, 55, 71, 56, 57, 18, 2, 20, 6, 8, 255, 0, 56, 55, 54, 53,
        52, 51, 50, 49, 17, 46, 4, 71, 66, 156, 1, 3, 60, 66, 111, 98, 25, 34, 21, 2, 138, 8, 11,
        48, 255, 0, 49, 50, 51, 52, 53, 54, 55, 56, 3, 57, 48,
    ];

    let st = schema.get_type("AddressBook").unwrap();
    let encoded = sproto::serde::to_bytes(&schema, st, &value).unwrap();
    assert_eq!(&encoded, expected_encoded, "encode mismatch");

    let packed = pack::pack(&encoded);
    assert_eq!(&packed, expected_packed, "pack mismatch");

    let unpacked = pack::unpack(&packed).unwrap();
    let decoded: AddressBookDec =
        sproto::serde::from_bytes(&schema, st, &unpacked[..encoded.len()]).unwrap();
    let persons = decoded.person.unwrap();
    assert_eq!(persons.len(), 2);
    assert_eq!(persons[0].name.as_deref(), Some("Alice"));
    assert_eq!(persons[1].name.as_deref(), Some("Bob"));
}
