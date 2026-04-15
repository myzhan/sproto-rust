//! Cross-validation: decode C-generated encoded binaries and verify SprotoValue.

use sproto::binary_schema;
use sproto::codec;
use sproto::value::SprotoValue;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

fn load_person_data_sproto() -> sproto::Sproto {
    binary_schema::load_binary(&testdata("person_data_schema.bin")).unwrap()
}

// SimpleStruct: Person { name="Alice", age=13, marital=false }
#[test]
fn test_decode_simple_struct() {
    let sproto = load_person_data_sproto();
    let person_type = sproto.get_type("Person").unwrap();
    let data = testdata("example1_encoded.bin");

    let decoded = codec::decode(&sproto, person_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();

    assert_eq!(map.get("name"), Some(&SprotoValue::Str("Alice".into())));
    assert_eq!(map.get("age"), Some(&SprotoValue::Integer(13)));
    assert_eq!(map.get("marital"), Some(&SprotoValue::Boolean(false)));
}

// StructArray: Person { name="Bob", age=40, children=[{name="Alice",age=13},{name="Carol",age=5}] }
#[test]
fn test_decode_struct_array() {
    let sproto = load_person_data_sproto();
    let person_type = sproto.get_type("Person").unwrap();
    let data = testdata("example2_encoded.bin");

    let decoded = codec::decode(&sproto, person_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();

    assert_eq!(map.get("name"), Some(&SprotoValue::Str("Bob".into())));
    assert_eq!(map.get("age"), Some(&SprotoValue::Integer(40)));

    let children = map.get("children").unwrap().as_array().unwrap();
    assert_eq!(children.len(), 2);

    let child1 = children[0].as_struct().unwrap();
    assert_eq!(child1.get("name"), Some(&SprotoValue::Str("Alice".into())));
    assert_eq!(child1.get("age"), Some(&SprotoValue::Integer(13)));

    let child2 = children[1].as_struct().unwrap();
    assert_eq!(child2.get("name"), Some(&SprotoValue::Str("Carol".into())));
    assert_eq!(child2.get("age"), Some(&SprotoValue::Integer(5)));
}

// NumberArray: Data { numbers=[1,2,3,4,5] }
#[test]
fn test_decode_number_array() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();
    let data = testdata("example3_encoded.bin");

    let decoded = codec::decode(&sproto, data_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();
    let numbers = map.get("numbers").unwrap().as_array().unwrap();

    let expected: Vec<SprotoValue> = (1..=5).map(|i| SprotoValue::Integer(i)).collect();
    assert_eq!(numbers, &expected);
}

// BigNumberArray: Data { numbers=[(1<<32)+1, (1<<32)+2, (1<<32)+3] }
#[test]
fn test_decode_big_number_array() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();
    let data = testdata("example4_encoded.bin");

    let decoded = codec::decode(&sproto, data_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();
    let numbers = map.get("numbers").unwrap().as_array().unwrap();

    let base: i64 = 1 << 32;
    let expected: Vec<SprotoValue> = vec![
        SprotoValue::Integer(base + 1),
        SprotoValue::Integer(base + 2),
        SprotoValue::Integer(base + 3),
    ];
    assert_eq!(numbers, &expected);
}

// BoolArray: Data { bools=[false, true, false] }
#[test]
fn test_decode_bool_array() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();
    let data = testdata("example5_encoded.bin");

    let decoded = codec::decode(&sproto, data_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();
    let bools = map.get("bools").unwrap().as_array().unwrap();

    let expected = vec![
        SprotoValue::Boolean(false),
        SprotoValue::Boolean(true),
        SprotoValue::Boolean(false),
    ];
    assert_eq!(bools, &expected);
}

// Number: Data { number=100000, bignumber=-10000000000 }
#[test]
fn test_decode_number() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();
    let data = testdata("example6_encoded.bin");

    let decoded = codec::decode(&sproto, data_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();

    assert_eq!(map.get("number"), Some(&SprotoValue::Integer(100000)));
    assert_eq!(
        map.get("bignumber"),
        Some(&SprotoValue::Integer(-10000000000))
    );
}

// Double: Data { double=0.01171875, doubles=[0.01171875, 23, 4] }
#[test]
fn test_decode_double() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();
    let data = testdata("example7_encoded.bin");

    let decoded = codec::decode(&sproto, data_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();

    assert_eq!(
        map.get("double"),
        Some(&SprotoValue::Double(0.01171875))
    );

    let doubles = map.get("doubles").unwrap().as_array().unwrap();
    assert_eq!(doubles.len(), 3);
    assert_eq!(doubles[0], SprotoValue::Double(0.01171875));
    assert_eq!(doubles[1], SprotoValue::Double(23.0));
    assert_eq!(doubles[2], SprotoValue::Double(4.0));
}

// FixedPoint: Data { fpn=1.82 }
// fpn is integer(2), so 1.82 * 100 = 182 encoded as inline value
#[test]
fn test_decode_fixed_point() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();
    let data = testdata("example8_encoded.bin");

    let decoded = codec::decode(&sproto, data_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();

    // fpn is integer(2), so the raw decoded value is 182
    assert_eq!(map.get("fpn"), Some(&SprotoValue::Integer(182)));
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

/// Bytes: Data{Bytes:[0x28,0x29,0x30,0x31]}
const GO_BYTES_FIELD: &[u8] = &[
    0x02, 0x00, 0x0f, 0x00, 0x00, 0x00,
    0x04, 0x00, 0x00, 0x00, 0x28, 0x29, 0x30, 0x31,
];

/// StringArray: Data{Strings:["Bob","Alice","Carol"]}
const GO_STRING_ARRAY: &[u8] = &[
    0x02, 0x00, 0x0d, 0x00, 0x00, 0x00,
    0x19, 0x00, 0x00, 0x00,
    0x03, 0x00, 0x00, 0x00, 0x42, 0x6F, 0x62,
    0x05, 0x00, 0x00, 0x00, 0x41, 0x6C, 0x69, 0x63, 0x65,
    0x05, 0x00, 0x00, 0x00, 0x43, 0x61, 0x72, 0x6F, 0x6C,
];

/// EmptyIntSlice: Data{Numbers:[]}
const GO_EMPTY_INT_SLICE: &[u8] = &[
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// EmptyDoubleSlice: Data{Doubles:[]}
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
    let decoded = codec::decode(&schema, st, GO_BYTES_FIELD).unwrap();
    assert_eq!(
        decoded.get("bytes"),
        Some(&SprotoValue::Binary(vec![0x28, 0x29, 0x30, 0x31]))
    );
}

#[test]
fn test_decode_string_array() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded = codec::decode(&schema, st, GO_STRING_ARRAY).unwrap();
    let strings = decoded.get("strings").unwrap().as_array().unwrap();
    assert_eq!(strings.len(), 3);
    assert_eq!(strings[0], SprotoValue::Str("Bob".into()));
    assert_eq!(strings[1], SprotoValue::Str("Alice".into()));
    assert_eq!(strings[2], SprotoValue::Str("Carol".into()));
}

#[test]
fn test_decode_empty_int_slice() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded = codec::decode(&schema, st, GO_EMPTY_INT_SLICE).unwrap();
    if let Some(numbers) = decoded.get("numbers") {
        assert!(numbers.as_array().unwrap().is_empty());
    }
}

#[test]
fn test_decode_empty_double_slice() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let decoded = codec::decode(&schema, st, GO_EMPTY_DOUBLE_SLICE).unwrap();
    if let Some(doubles) = decoded.get("doubles") {
        assert!(doubles.as_array().unwrap().is_empty());
    }
}

// Go TestEmptyIntSliceDecode: alternate encoding format with element size header
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
    let decoded = codec::decode(&schema, st, data).unwrap();
    if let Some(numbers) = decoded.get("numbers") {
        assert!(
            numbers.as_array().unwrap().is_empty(),
            "expected empty array, got {:?}",
            numbers
        );
    }
}

#[test]
fn test_decode_addressbook() {
    let schema = go_addressbook_schema();
    let st = schema.get_type("AddressBook").unwrap();
    let decoded = codec::decode(&schema, st, GO_ADDRESSBOOK).unwrap();
    let persons = decoded.get("person").unwrap().as_array().unwrap();

    assert_eq!(persons.len(), 2);

    let alice = &persons[0];
    assert_eq!(alice.get("name"), Some(&SprotoValue::Str("Alice".into())));
    assert_eq!(alice.get("id"), Some(&SprotoValue::Integer(10000)));
    let alice_phones = alice.get("phone").unwrap().as_array().unwrap();
    assert_eq!(alice_phones.len(), 2);
    assert_eq!(
        alice_phones[0].get("number"),
        Some(&SprotoValue::Str("123456789".into()))
    );
    assert_eq!(alice_phones[0].get("type"), Some(&SprotoValue::Integer(1)));
    assert_eq!(
        alice_phones[1].get("number"),
        Some(&SprotoValue::Str("87654321".into()))
    );
    assert_eq!(alice_phones[1].get("type"), Some(&SprotoValue::Integer(2)));

    let bob = &persons[1];
    assert_eq!(bob.get("name"), Some(&SprotoValue::Str("Bob".into())));
    assert_eq!(bob.get("id"), Some(&SprotoValue::Integer(20000)));
    let bob_phones = bob.get("phone").unwrap().as_array().unwrap();
    assert_eq!(bob_phones.len(), 1);
    assert_eq!(
        bob_phones[0].get("number"),
        Some(&SprotoValue::Str("01234567890".into()))
    );
    assert_eq!(bob_phones[0].get("type"), Some(&SprotoValue::Integer(3)));
}
