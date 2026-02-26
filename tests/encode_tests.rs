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

// Example 1: Person { name="Alice", age=13, marital=false }
#[test]
fn test_encode_example1() {
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

// Example 2: Person with children
#[test]
fn test_encode_example2() {
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

// Example 3: Data { numbers=[1,2,3,4,5] }
#[test]
fn test_encode_example3() {
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

// Example 4: Data { numbers=[(1<<32)+1, (1<<32)+2, (1<<32)+3] }
#[test]
fn test_encode_example4() {
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

// Example 5: Data { bools=[false, true, false] }
#[test]
fn test_encode_example5() {
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

// Example 6: Data { number=100000, bignumber=-10000000000 }
#[test]
fn test_encode_example6() {
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

// Example 7: Data { double=0.01171875, doubles=[0.01171875, 23, 4] }
#[test]
fn test_encode_example7() {
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

// Example 8: Data { fpn=1.82 } -- integer(2), encoded as 182
#[test]
fn test_encode_example8() {
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

// Full round-trip: decode C binary -> re-encode -> assert identical bytes
#[test]
fn test_roundtrip_all_examples() {
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
