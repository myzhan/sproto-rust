//! Cross-validation: decode C-generated encoded binaries and verify values.

use serde::{Deserialize, Serialize};
use sproto::binary_schema;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

fn load_person_data_sproto() -> sproto::Sproto {
    binary_schema::load_binary(&testdata("person_data_schema.bin")).unwrap()
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
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    age: Option<i64>,
    #[serde(default)]
    marital: Option<bool>,
    #[serde(default)]
    children: Option<Vec<Person>>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Data {
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
    fpn: Option<i64>,
}

// ---------------------------------------------------------------------------
// Binary schema cross-validation tests
// ---------------------------------------------------------------------------

#[test]
fn test_decode_simple_struct() {
    let sproto = load_person_data_sproto();
    let data = testdata("simple_struct_encoded.bin");
    let decoded: Person = decode(&sproto, "Person", &data);

    assert_eq!(decoded.name.as_deref(), Some("Alice"));
    assert_eq!(decoded.age, Some(13));
    assert_eq!(decoded.marital, Some(false));
}

#[test]
fn test_decode_struct_array() {
    let sproto = load_person_data_sproto();
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
fn test_decode_number_array() {
    let sproto = load_person_data_sproto();
    let data = testdata("number_array_encoded.bin");
    let decoded: Data = decode(&sproto, "Data", &data);

    assert_eq!(decoded.numbers.unwrap(), vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_decode_big_number_array() {
    let sproto = load_person_data_sproto();
    let data = testdata("big_number_array_encoded.bin");
    let decoded: Data = decode(&sproto, "Data", &data);

    let base: i64 = 1 << 32;
    assert_eq!(decoded.numbers.unwrap(), vec![base + 1, base + 2, base + 3]);
}

#[test]
fn test_decode_bool_array() {
    let sproto = load_person_data_sproto();
    let data = testdata("bool_array_encoded.bin");
    let decoded: Data = decode(&sproto, "Data", &data);

    assert_eq!(decoded.bools.unwrap(), vec![false, true, false]);
}

#[test]
fn test_decode_number() {
    let sproto = load_person_data_sproto();
    let data = testdata("number_encoded.bin");
    let decoded: Data = decode(&sproto, "Data", &data);

    assert_eq!(decoded.number, Some(100000));
    assert_eq!(decoded.bignumber, Some(-10000000000));
}

#[test]
fn test_decode_double() {
    let sproto = load_person_data_sproto();
    let data = testdata("double_encoded.bin");
    let decoded: Data = decode(&sproto, "Data", &data);

    assert_eq!(decoded.double, Some(0.01171875));
    let doubles = decoded.doubles.unwrap();
    assert_eq!(doubles, vec![0.01171875, 23.0, 4.0]);
}

#[test]
fn test_decode_fixed_point() {
    let sproto = load_person_data_sproto();
    let data = testdata("fixed_point_encoded.bin");
    let decoded: Data = decode(&sproto, "Data", &data);

    // fpn is integer(2): raw decoded value is 182
    assert_eq!(decoded.fpn, Some(182));
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

#[derive(Deserialize, Debug, PartialEq)]
struct PhoneNumber {
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
    phone: Option<Vec<PhoneNumber>>,
}

#[derive(Deserialize, Debug, PartialEq)]
struct GoAddressBook {
    #[serde(default)]
    person: Option<Vec<GoPerson>>,
}

/// Bytes: Data{bytes:[0x28,0x29,0x30,0x31]}
const GO_BYTES_FIELD: &[u8] = &[
    0x02, 0x00, 0x0f, 0x00, 0x00, 0x00,
    0x04, 0x00, 0x00, 0x00, 0x28, 0x29, 0x30, 0x31,
];

/// StringArray: Data{strings:["Bob","Alice","Carol"]}
const GO_STRING_ARRAY: &[u8] = &[
    0x02, 0x00, 0x0d, 0x00, 0x00, 0x00,
    0x19, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00, 0x42, 0x6F, 0x62,
    0x05, 0x00, 0x00, 0x00, 0x41, 0x6C, 0x69, 0x63, 0x65,
    0x05, 0x00, 0x00, 0x00, 0x43, 0x61, 0x72, 0x6F, 0x6C,
];

/// EmptyIntSlice: Data{numbers:[]}
const GO_EMPTY_INT_SLICE: &[u8] = &[
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// EmptyDoubleSlice: Data{doubles:[]}
const GO_EMPTY_DOUBLE_SLICE: &[u8] = &[
    0x02, 0x00, 0x09, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00,
];

/// AddressBook encoded (from Go abData)
const GO_ADDRESSBOOK: &[u8] = &[
    1, 0, 0, 0, 122, 0, 0, 0, 68, 0, 0, 0, 4, 0, 0, 0, 34, 78, 1, 0, 0, 0, 5, 0, 0, 0, 65,
    108, 105, 99, 101, 45, 0, 0, 0, 19, 0, 0, 0, 2, 0, 0, 0, 4, 0, 9, 0, 0, 0, 49, 50, 51, 52,
    53, 54, 55, 56, 57, 18, 0, 0, 0, 2, 0, 0, 0, 6, 0, 8, 0, 0, 0, 56, 55, 54, 53, 52, 51, 50,
    49, 46, 0, 0, 0, 4, 0, 0, 0, 66, 156, 1, 0, 0, 0, 3, 0, 0, 0, 66, 111, 98, 25, 0, 0, 0, 21,
    0, 0, 0, 2, 0, 0, 0, 8, 0, 11, 0, 0, 0, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48,
];

#[test]
fn test_decode_bytes_field() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded: GoData = sproto::serde::from_bytes(&schema, st, GO_BYTES_FIELD).unwrap();
    assert_eq!(decoded.bytes, Some(vec![0x28, 0x29, 0x30, 0x31]));
}

#[test]
fn test_decode_string_array() {
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
