//! Cross-validation: encode in Rust and compare bytes with C-generated .bin files.

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

fn hexdump(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")
}

// SimpleStruct: Person { name="Alice", age=13, marital=false }
#[test]
fn test_encode_simple_struct() {
    let sproto = load_person_data_sproto();
    let person_type = sproto.get_type("Person").unwrap();

    let value = SprotoValue::from_fields(vec![
        ("name", "Alice".into()),
        ("age", 13i64.into()),
        ("marital", false.into()),
    ]);

    let encoded = codec::encode(&sproto, person_type, &value).unwrap();
    let expected = testdata("example1_encoded.bin");

    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "example1 encode mismatch"
    );
}

// StructArray: Person with children
#[test]
fn test_encode_struct_array() {
    let sproto = load_person_data_sproto();
    let person_type = sproto.get_type("Person").unwrap();

    let value = SprotoValue::from_fields(vec![
        ("name", "Bob".into()),
        ("age", 40i64.into()),
        (
            "children",
            SprotoValue::Array(vec![
                SprotoValue::from_fields(vec![
                    ("name", "Alice".into()),
                    ("age", 13i64.into()),
                ]),
                SprotoValue::from_fields(vec![
                    ("name", "Carol".into()),
                    ("age", 5i64.into()),
                ]),
            ]),
        ),
    ]);

    let encoded = codec::encode(&sproto, person_type, &value).unwrap();
    let expected = testdata("example2_encoded.bin");

    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "example2 encode mismatch"
    );
}

// NumberArray: Data { numbers=[1,2,3,4,5] }
#[test]
fn test_encode_number_array() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();

    let value = SprotoValue::from_fields(vec![(
        "numbers",
        SprotoValue::Array(
            (1..=5).map(|i| SprotoValue::Integer(i)).collect(),
        ),
    )]);

    let encoded = codec::encode(&sproto, data_type, &value).unwrap();
    let expected = testdata("example3_encoded.bin");

    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "example3 encode mismatch"
    );
}

// BigNumberArray: Data { numbers=[(1<<32)+1, (1<<32)+2, (1<<32)+3] }
#[test]
fn test_encode_big_number_array() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();

    let base: i64 = 1 << 32;
    let value = SprotoValue::from_fields(vec![(
        "numbers",
        SprotoValue::Array(vec![
            SprotoValue::Integer(base + 1),
            SprotoValue::Integer(base + 2),
            SprotoValue::Integer(base + 3),
        ]),
    )]);

    let encoded = codec::encode(&sproto, data_type, &value).unwrap();
    let expected = testdata("example4_encoded.bin");

    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "example4 encode mismatch"
    );
}

// BoolArray: Data { bools=[false, true, false] }
#[test]
fn test_encode_bool_array() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();

    let value = SprotoValue::from_fields(vec![(
        "bools",
        SprotoValue::Array(vec![
            SprotoValue::Boolean(false),
            SprotoValue::Boolean(true),
            SprotoValue::Boolean(false),
        ]),
    )]);

    let encoded = codec::encode(&sproto, data_type, &value).unwrap();
    let expected = testdata("example5_encoded.bin");

    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "example5 encode mismatch"
    );
}

// Number: Data { number=100000, bignumber=-10000000000 }
#[test]
fn test_encode_number() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();

    let value = SprotoValue::from_fields(vec![
        ("number", SprotoValue::Integer(100000)),
        ("bignumber", SprotoValue::Integer(-10000000000)),
    ]);

    let encoded = codec::encode(&sproto, data_type, &value).unwrap();
    let expected = testdata("example6_encoded.bin");

    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "example6 encode mismatch"
    );
}

// Double: Data { double=0.01171875, doubles=[0.01171875, 23, 4] }
#[test]
fn test_encode_double() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();

    let value = SprotoValue::from_fields(vec![
        ("double", SprotoValue::Double(0.01171875)),
        (
            "doubles",
            SprotoValue::Array(vec![
                SprotoValue::Double(0.01171875),
                SprotoValue::Double(23.0),
                SprotoValue::Double(4.0),
            ]),
        ),
    ]);

    let encoded = codec::encode(&sproto, data_type, &value).unwrap();
    let expected = testdata("example7_encoded.bin");

    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "example7 encode mismatch"
    );
}

// FixedPoint: Data { fpn=1.82 } -- integer(2), encoded as 182
#[test]
fn test_encode_fixed_point() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();

    // fpn is integer(2), the C/Lua encoder receives 1.82 and multiplies by 100 -> 182
    // We pass the already-scaled integer value since our API is dynamic
    let value = SprotoValue::from_fields(vec![
        ("fpn", SprotoValue::Integer(182)),
    ]);

    let encoded = codec::encode(&sproto, data_type, &value).unwrap();
    let expected = testdata("example8_encoded.bin");

    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "example8 encode mismatch"
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

/// Bytes: Data{Bytes:[0x28,0x29,0x30,0x31]}
const GO_BYTES_FIELD: &[u8] = &[
    0x02, 0x00, // fn = 2
    0x0f, 0x00, // skip to id = 8
    0x00, 0x00, // id = 8, value in data part
    0x04, 0x00, 0x00, 0x00, // sizeof bytes
    0x28, 0x29, 0x30, 0x31,
];

/// StringArray: Data{Strings:["Bob","Alice","Carol"]}
const GO_STRING_ARRAY: &[u8] = &[
    0x02, 0x00, // fn = 2
    0x0d, 0x00, // skip to id = 7
    0x00, 0x00, // id = 7, value in data part
    0x19, 0x00, 0x00, 0x00, // sizeof []string
    0x03, 0x00, 0x00, 0x00, 0x42, 0x6F, 0x62, // "Bob"
    0x05, 0x00, 0x00, 0x00, 0x41, 0x6C, 0x69, 0x63, 0x65, // "Alice"
    0x05, 0x00, 0x00, 0x00, 0x43, 0x61, 0x72, 0x6F, 0x6C, // "Carol"
];

/// EmptyIntSlice: Data{Numbers:[]}
const GO_EMPTY_INT_SLICE: &[u8] = &[
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

/// EmptyDoubleSlice: Data{Doubles:[]}
const GO_EMPTY_DOUBLE_SLICE: &[u8] = &[
    0x02, 0x00, // fn = 2
    0x09, 0x00, // skip id = 4
    0x00, 0x00, // id = 5, value in data part
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
fn test_encode_bytes_field() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let value = SprotoValue::from_fields(vec![(
        "bytes",
        SprotoValue::Binary(vec![0x28, 0x29, 0x30, 0x31]),
    )]);
    let encoded = codec::encode(&schema, st, &value).unwrap();
    assert_eq!(hexdump(&encoded), hexdump(GO_BYTES_FIELD));
}

#[test]
fn test_encode_string_array() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let value = SprotoValue::from_fields(vec![(
        "strings",
        SprotoValue::Array(vec![
            SprotoValue::Str("Bob".into()),
            SprotoValue::Str("Alice".into()),
            SprotoValue::Str("Carol".into()),
        ]),
    )]);
    let encoded = codec::encode(&schema, st, &value).unwrap();
    assert_eq!(hexdump(&encoded), hexdump(GO_STRING_ARRAY));
}

#[test]
fn test_encode_empty_int_slice() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let value = SprotoValue::from_fields(vec![("numbers", SprotoValue::Array(vec![]))]);
    let encoded = codec::encode(&schema, st, &value).unwrap();
    assert_eq!(hexdump(&encoded), hexdump(GO_EMPTY_INT_SLICE));
}

#[test]
fn test_encode_empty_double_slice() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let value = SprotoValue::from_fields(vec![("doubles", SprotoValue::Array(vec![]))]);
    let encoded = codec::encode(&schema, st, &value).unwrap();
    assert_eq!(hexdump(&encoded), hexdump(GO_EMPTY_DOUBLE_SLICE));
}

#[test]
fn test_encode_addressbook() {
    let schema = go_addressbook_schema();
    let st = schema.get_type("AddressBook").unwrap();
    let value = SprotoValue::from_fields(vec![(
        "person",
        SprotoValue::Array(vec![
            SprotoValue::from_fields(vec![
                ("name", "Alice".into()),
                ("id", 10000i64.into()),
                (
                    "phone",
                    SprotoValue::Array(vec![
                        SprotoValue::from_fields(vec![
                            ("number", SprotoValue::Str("123456789".into())),
                            ("type", SprotoValue::Integer(1)),
                        ]),
                        SprotoValue::from_fields(vec![
                            ("number", SprotoValue::Str("87654321".into())),
                            ("type", SprotoValue::Integer(2)),
                        ]),
                    ]),
                ),
            ]),
            SprotoValue::from_fields(vec![
                ("name", "Bob".into()),
                ("id", 20000i64.into()),
                (
                    "phone",
                    SprotoValue::Array(vec![SprotoValue::from_fields(vec![
                        ("number", SprotoValue::Str("01234567890".into())),
                        ("type", SprotoValue::Integer(3)),
                    ])]),
                ),
            ]),
        ]),
    )]);
    let encoded = codec::encode(&schema, st, &value).unwrap();
    assert_eq!(hexdump(&encoded), hexdump(GO_ADDRESSBOOK));
}

// Full round-trip: decode binary -> re-encode -> assert identical bytes
#[test]
fn test_encode_decode_roundtrip_all() {
    let sproto = load_person_data_sproto();

    let cases: Vec<(&str, &str)> = vec![
        ("Person", "example1_encoded.bin"),
        ("Person", "example2_encoded.bin"),
        ("Data", "example3_encoded.bin"),
        ("Data", "example4_encoded.bin"),
        ("Data", "example5_encoded.bin"),
        ("Data", "example6_encoded.bin"),
        ("Data", "example7_encoded.bin"),
        ("Data", "example8_encoded.bin"),
    ];

    for (type_name, file) in cases {
        let st = sproto.get_type(type_name).unwrap();
        let original = testdata(file);
        let decoded = codec::decode(&sproto, st, &original).unwrap();
        let reencoded = codec::encode(&sproto, st, &decoded).unwrap();

        assert_eq!(
            hexdump(&reencoded),
            hexdump(&original),
            "round-trip failed for {}",
            file
        );
    }
}
