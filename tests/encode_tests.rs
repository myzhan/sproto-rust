//! Cross-validation: encode in Rust and compare bytes with C-generated .bin files.

use serde::{Deserialize, Serialize};
use sproto::binary_schema;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

fn load_sproto() -> sproto::Sproto {
    binary_schema::load_binary(&testdata("schema.bin")).unwrap()
}

fn hexdump(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn encode<T: Serialize>(sproto: &sproto::Sproto, type_name: &str, value: &T) -> Vec<u8> {
    let st = sproto.get_type(type_name).unwrap();
    sproto::serde::to_bytes(sproto, st, value).unwrap()
}

fn decode<T: for<'de> Deserialize<'de>>(
    sproto: &sproto::Sproto,
    type_name: &str,
    data: &[u8],
) -> T {
    let st = sproto.get_type(type_name).unwrap();
    sproto::serde::from_bytes(sproto, st, data).unwrap()
}

// ---------------------------------------------------------------------------
// Serde helper for binary (Vec<u8>) fields
// ---------------------------------------------------------------------------

mod opt_bytes {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(val: &Option<Vec<u8>>, s: S) -> Result<S::Ok, S::Error> {
        match val {
            Some(b) => serde_bytes::serialize(b, s),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Vec<u8>>, D::Error> {
        Ok(Some(serde_bytes::deserialize(d)?))
    }
}

// ---------------------------------------------------------------------------
// Serde structs matching the unified binary schema
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct PhoneNumber {
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#type: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Default)]
struct Person {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    age: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none", with = "opt_bytes")]
    photo: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fpn: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<PhoneNumber>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phones: Option<Vec<PhoneNumber>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<Person>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    numbers: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    flags: Option<Vec<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    values: Option<Vec<f64>>,
}

// ---------------------------------------------------------------------------
// Binary schema cross-validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_encode_simple_struct() {
    let sproto = load_sproto();
    let value = Person {
        name: Some("Alice".into()),
        age: Some(13),
        active: Some(false),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("simple_struct_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "simple_struct encode mismatch"
    );
}

#[test]
fn test_encode_all_scalars() {
    let sproto = load_sproto();
    let value = Person {
        name: Some("Alice".into()),
        age: Some(30),
        active: Some(true),
        score: Some(0.01171875),
        photo: Some(vec![0x28, 0x29, 0x30, 0x31]),
        fpn: Some(182), // integer(2): pre-scaled
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("all_scalars_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "all_scalars encode mismatch"
    );
}

#[test]
fn test_encode_nested_struct() {
    let sproto = load_sproto();
    let value = Person {
        name: Some("Alice".into()),
        phone: Some(PhoneNumber {
            number: Some("123456789".into()),
            r#type: Some(1),
        }),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("nested_struct_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "nested_struct encode mismatch"
    );
}

#[test]
fn test_encode_struct_array() {
    let sproto = load_sproto();
    let value = Person {
        name: Some("Bob".into()),
        age: Some(40),
        children: Some(vec![
            Person {
                name: Some("Alice".into()),
                age: Some(13),
                ..Default::default()
            },
            Person {
                name: Some("Carol".into()),
                age: Some(5),
                ..Default::default()
            },
        ]),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("struct_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "struct_array encode mismatch"
    );
}

#[test]
fn test_encode_int_array() {
    let sproto = load_sproto();
    let value = Person {
        numbers: Some(vec![1, 2, 3, 4, 5]),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("int_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "int_array encode mismatch"
    );
}

#[test]
fn test_encode_big_int_array() {
    let sproto = load_sproto();
    let base: i64 = 1 << 32;
    let value = Person {
        numbers: Some(vec![base + 1, base + 2, base + 3]),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("big_int_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "big_int_array encode mismatch"
    );
}

#[test]
fn test_encode_bool_array() {
    let sproto = load_sproto();
    let value = Person {
        flags: Some(vec![false, true, false]),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("bool_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "bool_array encode mismatch"
    );
}

#[test]
fn test_encode_number() {
    let sproto = load_sproto();
    let value = Person {
        age: Some(100000),
        id: Some(-10000000000),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("number_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "number encode mismatch"
    );
}

#[test]
fn test_encode_double() {
    let sproto = load_sproto();
    let value = Person {
        score: Some(0.01171875),
        values: Some(vec![0.01171875, 23.0, 4.0]),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("double_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "double encode mismatch"
    );
}

#[test]
fn test_encode_string_array() {
    let sproto = load_sproto();
    let value = Person {
        tags: Some(vec![
            "hello".into(),
            "world".into(),
            "\u{4f60}\u{597d}".into(),
        ]),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("string_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "string_array encode mismatch"
    );
}

#[test]
fn test_encode_fixed_point() {
    let sproto = load_sproto();
    // fpn is integer(2): 1.82 * 100 = 182 (pre-scaled)
    let value = Person {
        fpn: Some(182),
        ..Default::default()
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("fixed_point_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "fixed_point encode mismatch"
    );
}

#[test]
fn test_encode_full() {
    let sproto = load_sproto();
    let value = Person {
        name: Some("Alice".into()),
        age: Some(30),
        active: Some(true),
        score: Some(0.01171875),
        photo: Some(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        fpn: Some(182),
        id: Some(10000),
        phone: Some(PhoneNumber {
            number: Some("123456789".into()),
            r#type: Some(1),
        }),
        phones: Some(vec![
            PhoneNumber {
                number: Some("123456789".into()),
                r#type: Some(1),
            },
            PhoneNumber {
                number: Some("87654321".into()),
                r#type: Some(2),
            },
        ]),
        children: Some(vec![Person {
            name: Some("Bob".into()),
            age: Some(5),
            ..Default::default()
        }]),
        tags: Some(vec![
            "hello".into(),
            "world".into(),
            "\u{4f60}\u{597d}".into(),
        ]),
        numbers: Some(vec![1, 2, 3, 4, 5]),
        flags: Some(vec![false, true, false]),
        values: Some(vec![0.01171875, 23.0, 4.0]),
    };
    let encoded = encode(&sproto, "Person", &value);
    let expected = testdata("full_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "full encode mismatch"
    );
}

// =============================================================================
// Go reference inline byte tests (gosproto/sproto_test.go test vectors)
// =============================================================================

fn go_data_schema() -> sproto::Sproto {
    sproto::parser::parse(
        r#"
        .Data {
            numbers 0 : *integer
            bools 1 : *boolean
            number 2 : integer
            bignumber 3 : integer
            double 4 : double
            doubles 5 : *double
            strings 7 : *string
            bytes 8 : binary
        }
    "#,
    )
    .unwrap()
}

fn go_addressbook_schema() -> sproto::Sproto {
    sproto::parser::parse(
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
    .unwrap()
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct GoData {
    #[serde(skip_serializing_if = "Option::is_none")]
    numbers: Option<Vec<i64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bools: Option<Vec<bool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    bignumber: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    double: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    doubles: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    strings: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none", with = "opt_bytes")]
    bytes: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct GoPhoneNumber {
    number: String,
    r#type: i64,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct GoPerson {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phone: Option<Vec<GoPhoneNumber>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct GoAddressBook {
    #[serde(skip_serializing_if = "Option::is_none")]
    person: Option<Vec<GoPerson>>,
}

/// Bytes: Data{bytes:[0x28,0x29,0x30,0x31]}
const GO_BYTES_FIELD: &[u8] = &[
    0x02, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x28, 0x29, 0x30, 0x31,
];

/// StringArray: Data{strings:["Bob","Alice","Carol"]}
const GO_STRING_ARRAY: &[u8] = &[
    0x02, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x42, 0x6F,
    0x62, 0x05, 0x00, 0x00, 0x00, 0x41, 0x6C, 0x69, 0x63, 0x65, 0x05, 0x00, 0x00, 0x00, 0x43, 0x61,
    0x72, 0x6F, 0x6C,
];

/// EmptyIntSlice: Data{numbers:[]}
const GO_EMPTY_INT_SLICE: &[u8] = &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// EmptyDoubleSlice: Data{doubles:[]}
const GO_EMPTY_DOUBLE_SLICE: &[u8] = &[0x02, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// AddressBook encoded (from Go abData)
const GO_ADDRESSBOOK: &[u8] = &[
    1, 0, 0, 0, 122, 0, 0, 0, 68, 0, 0, 0, 4, 0, 0, 0, 34, 78, 1, 0, 0, 0, 5, 0, 0, 0, 65, 108,
    105, 99, 101, 45, 0, 0, 0, 19, 0, 0, 0, 2, 0, 0, 0, 4, 0, 9, 0, 0, 0, 49, 50, 51, 52, 53, 54,
    55, 56, 57, 18, 0, 0, 0, 2, 0, 0, 0, 6, 0, 8, 0, 0, 0, 56, 55, 54, 53, 52, 51, 50, 49, 46, 0,
    0, 0, 4, 0, 0, 0, 66, 156, 1, 0, 0, 0, 3, 0, 0, 0, 66, 111, 98, 25, 0, 0, 0, 21, 0, 0, 0, 2, 0,
    0, 0, 8, 0, 11, 0, 0, 0, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48,
];

#[test]
fn test_encode_bytes_field() {
    let schema = go_data_schema();
    let value = GoData {
        numbers: None,
        bools: None,
        number: None,
        bignumber: None,
        double: None,
        doubles: None,
        strings: None,
        bytes: Some(vec![0x28, 0x29, 0x30, 0x31]),
    };
    let encoded = encode(&schema, "Data", &value);
    assert_eq!(hexdump(&encoded), hexdump(GO_BYTES_FIELD));
}

#[test]
fn test_encode_go_string_array() {
    let schema = go_data_schema();
    let value = GoData {
        numbers: None,
        bools: None,
        number: None,
        bignumber: None,
        double: None,
        doubles: None,
        strings: Some(vec!["Bob".into(), "Alice".into(), "Carol".into()]),
        bytes: None,
    };
    let encoded = encode(&schema, "Data", &value);
    assert_eq!(hexdump(&encoded), hexdump(GO_STRING_ARRAY));
}

#[test]
fn test_encode_empty_int_slice() {
    let schema = go_data_schema();
    let value = GoData {
        numbers: Some(vec![]),
        bools: None,
        number: None,
        bignumber: None,
        double: None,
        doubles: None,
        strings: None,
        bytes: None,
    };
    let encoded = encode(&schema, "Data", &value);
    assert_eq!(hexdump(&encoded), hexdump(GO_EMPTY_INT_SLICE));
}

#[test]
fn test_encode_empty_double_slice() {
    let schema = go_data_schema();
    let value = GoData {
        numbers: None,
        bools: None,
        number: None,
        bignumber: None,
        double: None,
        doubles: Some(vec![]),
        strings: None,
        bytes: None,
    };
    let encoded = encode(&schema, "Data", &value);
    assert_eq!(hexdump(&encoded), hexdump(GO_EMPTY_DOUBLE_SLICE));
}

#[test]
fn test_encode_addressbook() {
    let schema = go_addressbook_schema();
    let value = GoAddressBook {
        person: Some(vec![
            GoPerson {
                name: Some("Alice".into()),
                id: Some(10000),
                email: None,
                phone: Some(vec![
                    GoPhoneNumber {
                        number: "123456789".into(),
                        r#type: 1,
                    },
                    GoPhoneNumber {
                        number: "87654321".into(),
                        r#type: 2,
                    },
                ]),
            },
            GoPerson {
                name: Some("Bob".into()),
                id: Some(20000),
                email: None,
                phone: Some(vec![GoPhoneNumber {
                    number: "01234567890".into(),
                    r#type: 3,
                }]),
            },
        ]),
    };
    let encoded = encode(&schema, "AddressBook", &value);
    assert_eq!(hexdump(&encoded), hexdump(GO_ADDRESSBOOK));
}

// Round-trip: decode binary -> re-encode -> assert identical bytes
#[test]
fn test_encode_decode_roundtrip_all() {
    let sproto = load_sproto();
    let cases: Vec<&str> = vec![
        "simple_struct_encoded.bin",
        "all_scalars_encoded.bin",
        "nested_struct_encoded.bin",
        "struct_array_encoded.bin",
        "int_array_encoded.bin",
        "big_int_array_encoded.bin",
        "bool_array_encoded.bin",
        "number_encoded.bin",
        "double_encoded.bin",
        "string_array_encoded.bin",
        "fixed_point_encoded.bin",
        "full_encoded.bin",
    ];

    for file in cases {
        let original = testdata(file);
        let decoded: Person = decode(&sproto, "Person", &original);
        let reencoded = encode(&sproto, "Person", &decoded);
        assert_eq!(
            hexdump(&reencoded),
            hexdump(&original),
            "round-trip failed for {}",
            file
        );
    }
}
