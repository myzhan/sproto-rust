//! Cross-validation: decode C-generated encoded binaries and verify values.

use serde::{Deserialize, Serialize};
use sproto::binary_schema;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

fn load_sproto() -> sproto::Sproto {
    binary_schema::load_binary(&testdata("schema.bin")).unwrap()
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

    #[allow(dead_code)]
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

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct PhoneNumber {
    #[serde(default)]
    number: Option<String>,
    #[serde(default)]
    r#type: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Person {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    age: Option<i64>,
    #[serde(default)]
    active: Option<bool>,
    #[serde(default)]
    score: Option<f64>,
    #[serde(default, with = "opt_bytes")]
    photo: Option<Vec<u8>>,
    #[serde(default)]
    fpn: Option<i64>,
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    phone: Option<PhoneNumber>,
    #[serde(default)]
    phones: Option<Vec<PhoneNumber>>,
    #[serde(default)]
    children: Option<Vec<Person>>,
    #[serde(default)]
    tags: Option<Vec<String>>,
    #[serde(default)]
    numbers: Option<Vec<i64>>,
    #[serde(default)]
    flags: Option<Vec<bool>>,
    #[serde(default)]
    values: Option<Vec<f64>>,
}

// ---------------------------------------------------------------------------
// Binary schema cross-validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_decode_simple_struct() {
    let sproto = load_sproto();
    let data = testdata("simple_struct_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.name.as_deref(), Some("Alice"));
    assert_eq!(decoded.age, Some(13));
    assert_eq!(decoded.active, Some(false));
}

#[test]
fn test_decode_all_scalars() {
    let sproto = load_sproto();
    let data = testdata("all_scalars_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.name.as_deref(), Some("Alice"));
    assert_eq!(decoded.age, Some(30));
    assert_eq!(decoded.active, Some(true));
    assert_eq!(decoded.score, Some(0.01171875));
    assert_eq!(decoded.photo, Some(vec![0x28, 0x29, 0x30, 0x31]));
    // fpn is integer(2): 1.82 * 100 = 182
    assert_eq!(decoded.fpn, Some(182));
}

#[test]
fn test_decode_nested_struct() {
    let sproto = load_sproto();
    let data = testdata("nested_struct_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.name.as_deref(), Some("Alice"));
    let phone = decoded.phone.unwrap();
    assert_eq!(phone.number.as_deref(), Some("123456789"));
    assert_eq!(phone.r#type, Some(1));
}

#[test]
fn test_decode_struct_array() {
    let sproto = load_sproto();
    let data = testdata("struct_array_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.name.as_deref(), Some("Bob"));
    assert_eq!(decoded.age, Some(40));
    let children = decoded.children.unwrap();
    assert_eq!(children.len(), 2);
    assert_eq!(children[0].name.as_deref(), Some("Alice"));
    assert_eq!(children[0].age, Some(13));
    assert_eq!(children[1].name.as_deref(), Some("Carol"));
    assert_eq!(children[1].age, Some(5));
}

#[test]
fn test_decode_int_array() {
    let sproto = load_sproto();
    let data = testdata("int_array_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.numbers.unwrap(), vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_decode_big_int_array() {
    let sproto = load_sproto();
    let data = testdata("big_int_array_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    let base: i64 = 1 << 32;
    assert_eq!(decoded.numbers.unwrap(), vec![base + 1, base + 2, base + 3]);
}

#[test]
fn test_decode_bool_array() {
    let sproto = load_sproto();
    let data = testdata("bool_array_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.flags.unwrap(), vec![false, true, false]);
}

#[test]
fn test_decode_number() {
    let sproto = load_sproto();
    let data = testdata("number_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.age, Some(100000));
    assert_eq!(decoded.id, Some(-10000000000));
}

#[test]
fn test_decode_double() {
    let sproto = load_sproto();
    let data = testdata("double_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.score, Some(0.01171875));
    let values = decoded.values.unwrap();
    assert_eq!(values, vec![0.01171875, 23.0, 4.0]);
}

#[test]
fn test_decode_string_array() {
    let sproto = load_sproto();
    let data = testdata("string_array_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(
        decoded.tags.unwrap(),
        vec![
            "hello".to_string(),
            "world".to_string(),
            "\u{4f60}\u{597d}".to_string()
        ]
    );
}

#[test]
fn test_decode_fixed_point() {
    let sproto = load_sproto();
    let data = testdata("fixed_point_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    // fpn is integer(2): raw decoded value is 182
    assert_eq!(decoded.fpn, Some(182));
}

#[test]
fn test_decode_full() {
    let sproto = load_sproto();
    let data = testdata("full_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.name.as_deref(), Some("Alice"));
    assert_eq!(decoded.age, Some(30));
    assert_eq!(decoded.active, Some(true));
    assert_eq!(decoded.score, Some(0.01171875));
    assert_eq!(decoded.photo, Some(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    assert_eq!(decoded.fpn, Some(182));
    assert_eq!(decoded.id, Some(10000));

    let phone = decoded.phone.unwrap();
    assert_eq!(phone.number.as_deref(), Some("123456789"));
    assert_eq!(phone.r#type, Some(1));

    let phones = decoded.phones.unwrap();
    assert_eq!(phones.len(), 2);
    assert_eq!(phones[0].number.as_deref(), Some("123456789"));
    assert_eq!(phones[0].r#type, Some(1));
    assert_eq!(phones[1].number.as_deref(), Some("87654321"));
    assert_eq!(phones[1].r#type, Some(2));

    let children = decoded.children.unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name.as_deref(), Some("Bob"));
    assert_eq!(children[0].age, Some(5));

    assert_eq!(
        decoded.tags,
        Some(vec![
            "hello".to_string(),
            "world".to_string(),
            "\u{4f60}\u{597d}".to_string()
        ])
    );
    assert_eq!(decoded.numbers, Some(vec![1, 2, 3, 4, 5]));
    assert_eq!(decoded.flags, Some(vec![false, true, false]));
    assert_eq!(decoded.values, Some(vec![0.01171875, 23.0, 4.0]));
}

// =============================================================================
// Go reference inline byte tests
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

#[derive(Deserialize, Debug, PartialEq)]
struct GoData {
    #[serde(default)]
    numbers: Option<Vec<i64>>,
    #[serde(default)]
    bools: Option<Vec<bool>>,
    #[serde(default)]
    number: Option<i64>,
    #[serde(default)]
    bignumber: Option<i64>,
    #[serde(default)]
    double: Option<f64>,
    #[serde(default)]
    doubles: Option<Vec<f64>>,
    #[serde(default)]
    strings: Option<Vec<String>>,
    #[serde(default, with = "opt_bytes")]
    bytes: Option<Vec<u8>>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct GoPhoneNumber {
    number: String,
    r#type: i64,
}

#[derive(Deserialize, Debug, PartialEq)]
struct GoPerson {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: Option<i64>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    phone: Option<Vec<GoPhoneNumber>>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct GoAddressBook {
    #[serde(default)]
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
fn test_decode_bytes_field() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded: GoData = sproto::serde::from_bytes(&schema, st, GO_BYTES_FIELD).unwrap();
    assert_eq!(decoded.bytes, Some(vec![0x28, 0x29, 0x30, 0x31]));
}

#[test]
fn test_decode_go_string_array() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded: GoData = sproto::serde::from_bytes(&schema, st, GO_STRING_ARRAY).unwrap();
    let strings = decoded.strings.unwrap();
    assert_eq!(strings, vec!["Bob", "Alice", "Carol"]);
}

#[test]
fn test_decode_empty_int_slice() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded: GoData = sproto::serde::from_bytes(&schema, st, GO_EMPTY_INT_SLICE).unwrap();
    assert_eq!(decoded.numbers, Some(vec![]));
}

#[test]
fn test_decode_empty_double_slice() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded: GoData = sproto::serde::from_bytes(&schema, st, GO_EMPTY_DOUBLE_SLICE).unwrap();
    assert_eq!(decoded.doubles, Some(vec![]));
}

#[test]
fn test_decode_empty_int_slice_alternate_format() {
    let data: &[u8] = &[
        0x01, 0x00, // fn = 1
        0x00, 0x00, // id = 0, value in data part
        0x01, 0x00, 0x00, 0x00, // sizeof numbers = 1
        0x04, // sizeof int32 (element size header, but 0 elements)
    ];
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded: GoData = sproto::serde::from_bytes(&schema, st, data).unwrap();
    assert_eq!(decoded.numbers, Some(vec![]));
}

#[test]
fn test_decode_addressbook() {
    let schema = go_addressbook_schema();
    let st = schema.get_type("AddressBook").unwrap();
    let decoded: GoAddressBook = sproto::serde::from_bytes(&schema, st, GO_ADDRESSBOOK).unwrap();

    let persons = decoded.person.unwrap();
    assert_eq!(persons.len(), 2);

    // Alice
    assert_eq!(persons[0].name.as_deref(), Some("Alice"));
    assert_eq!(persons[0].id, Some(10000));
    let alice_phones = persons[0].phone.as_ref().unwrap();
    assert_eq!(alice_phones.len(), 2);
    assert_eq!(alice_phones[0].number, "123456789");
    assert_eq!(alice_phones[0].r#type, 1);
    assert_eq!(alice_phones[1].number, "87654321");
    assert_eq!(alice_phones[1].r#type, 2);

    // Bob
    assert_eq!(persons[1].name.as_deref(), Some("Bob"));
    assert_eq!(persons[1].id, Some(20000));
    let bob_phones = persons[1].phone.as_ref().unwrap();
    assert_eq!(bob_phones.len(), 1);
    assert_eq!(bob_phones[0].number, "01234567890");
    assert_eq!(bob_phones[0].r#type, 3);
}
