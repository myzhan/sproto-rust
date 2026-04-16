//! Cross-validation: encode in Rust and compare bytes with C-generated .bin files.

use serde::{Deserialize, Serialize};
use sproto::binary_schema;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

fn load_person_data_sproto() -> sproto::Sproto {
    binary_schema::load_binary(&testdata("person_data_schema.bin")).unwrap()
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
// Serde structs matching the person_data binary schema
// ---------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Person {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    age: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    marital: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    children: Option<Vec<Person>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Data {
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
    fpn: Option<i64>,
}

// ---------------------------------------------------------------------------
// Binary schema cross-validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_encode_simple_struct() {
    let sproto = load_person_data_sproto();
    let value = Person {
        name: Some("Alice".into()),
        age: Some(13),
        marital: Some(false),
        children: None,
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
fn test_encode_struct_array() {
    let sproto = load_person_data_sproto();
    let value = Person {
        name: Some("Bob".into()),
        age: Some(40),
        marital: None,
        children: Some(vec![
            Person {
                name: Some("Alice".into()),
                age: Some(13),
                marital: None,
                children: None,
            },
            Person {
                name: Some("Carol".into()),
                age: Some(5),
                marital: None,
                children: None,
            },
        ]),
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
fn test_encode_number_array() {
    let sproto = load_person_data_sproto();
    let value = Data {
        numbers: Some(vec![1, 2, 3, 4, 5]),
        bools: None,
        number: None,
        bignumber: None,
        double: None,
        doubles: None,
        fpn: None,
    };
    let encoded = encode(&sproto, "Data", &value);
    let expected = testdata("number_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "number_array encode mismatch"
    );
}

#[test]
fn test_encode_big_number_array() {
    let sproto = load_person_data_sproto();
    let base: i64 = 1 << 32;
    let value = Data {
        numbers: Some(vec![base + 1, base + 2, base + 3]),
        bools: None,
        number: None,
        bignumber: None,
        double: None,
        doubles: None,
        fpn: None,
    };
    let encoded = encode(&sproto, "Data", &value);
    let expected = testdata("big_number_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "big_number_array encode mismatch"
    );
}

#[test]
fn test_encode_bool_array() {
    let sproto = load_person_data_sproto();
    let value = Data {
        numbers: None,
        bools: Some(vec![false, true, false]),
        number: None,
        bignumber: None,
        double: None,
        doubles: None,
        fpn: None,
    };
    let encoded = encode(&sproto, "Data", &value);
    let expected = testdata("bool_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "bool_array encode mismatch"
    );
}

#[test]
fn test_encode_number() {
    let sproto = load_person_data_sproto();
    let value = Data {
        numbers: None,
        bools: None,
        number: Some(100000),
        bignumber: Some(-10000000000),
        double: None,
        doubles: None,
        fpn: None,
    };
    let encoded = encode(&sproto, "Data", &value);
    let expected = testdata("number_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "number encode mismatch"
    );
}

#[test]
fn test_encode_double() {
    let sproto = load_person_data_sproto();
    let value = Data {
        numbers: None,
        bools: None,
        number: None,
        bignumber: None,
        double: Some(0.01171875),
        doubles: Some(vec![0.01171875, 23.0, 4.0]),
        fpn: None,
    };
    let encoded = encode(&sproto, "Data", &value);
    let expected = testdata("double_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "double encode mismatch"
    );
}

#[test]
fn test_encode_fixed_point() {
    let sproto = load_person_data_sproto();
    // fpn is integer(2): 1.82 * 100 = 182 (pre-scaled)
    let value = Data {
        numbers: None,
        bools: None,
        number: None,
        bignumber: None,
        double: None,
        doubles: None,
        fpn: Some(182),
    };
    let encoded = encode(&sproto, "Data", &value);
    let expected = testdata("fixed_point_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "fixed_point encode mismatch"
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

/// Custom serializer for Option<Vec<u8>> that uses serde_bytes when Some.
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PhoneNumber {
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
    phone: Option<Vec<PhoneNumber>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct AddressBook {
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
fn test_encode_string_array() {
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
    let value = AddressBook {
        person: Some(vec![
            GoPerson {
                name: Some("Alice".into()),
                id: Some(10000),
                email: None,
                phone: Some(vec![
                    PhoneNumber {
                        number: "123456789".into(),
                        r#type: 1,
                    },
                    PhoneNumber {
                        number: "87654321".into(),
                        r#type: 2,
                    },
                ]),
            },
            GoPerson {
                name: Some("Bob".into()),
                id: Some(20000),
                email: None,
                phone: Some(vec![PhoneNumber {
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
    let sproto = load_person_data_sproto();
    let cases: Vec<(&str, &str)> = vec![
        ("Person", "simple_struct_encoded.bin"),
        ("Person", "struct_array_encoded.bin"),
        ("Data", "number_array_encoded.bin"),
        ("Data", "big_number_array_encoded.bin"),
        ("Data", "bool_array_encoded.bin"),
        ("Data", "number_encoded.bin"),
        ("Data", "double_encoded.bin"),
        ("Data", "fixed_point_encoded.bin"),
    ];

    for (type_name, file) in cases {
        let original = testdata(file);
        match type_name {
            "Person" => {
                let decoded: Person = decode(&sproto, type_name, &original);
                let reencoded = encode(&sproto, type_name, &decoded);
                assert_eq!(
                    hexdump(&reencoded),
                    hexdump(&original),
                    "round-trip failed for {}",
                    file
                );
            }
            "Data" => {
                let decoded: Data = decode(&sproto, type_name, &original);
                let reencoded = encode(&sproto, type_name, &decoded);
                assert_eq!(
                    hexdump(&reencoded),
                    hexdump(&original),
                    "round-trip failed for {}",
                    file
                );
            }
            _ => unreachable!(),
        }
    }
}
