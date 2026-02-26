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

// Example 1: Person { name="Alice", age=13, marital=false }
#[test]
fn test_decode_example1() {
    let sproto = load_person_data_sproto();
    let person_type = sproto.get_type("Person").unwrap();
    let data = testdata("example1_encoded.bin");

    let decoded = codec::decode(&sproto, person_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();

    assert_eq!(map.get("name"), Some(&SprotoValue::Str("Alice".into())));
    assert_eq!(map.get("age"), Some(&SprotoValue::Integer(13)));
    assert_eq!(map.get("marital"), Some(&SprotoValue::Boolean(false)));
}

// Example 2: Person { name="Bob", age=40, children=[{name="Alice",age=13},{name="Carol",age=5}] }
#[test]
fn test_decode_example2() {
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

// Example 3: Data { numbers=[1,2,3,4,5] }
#[test]
fn test_decode_example3() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();
    let data = testdata("example3_encoded.bin");

    let decoded = codec::decode(&sproto, data_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();
    let numbers = map.get("numbers").unwrap().as_array().unwrap();

    let expected: Vec<SprotoValue> = (1..=5).map(|i| SprotoValue::Integer(i)).collect();
    assert_eq!(numbers, &expected);
}

// Example 4: Data { numbers=[(1<<32)+1, (1<<32)+2, (1<<32)+3] }
#[test]
fn test_decode_example4() {
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

// Example 5: Data { bools=[false, true, false] }
#[test]
fn test_decode_example5() {
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

// Example 6: Data { number=100000, bignumber=-10000000000 }
#[test]
fn test_decode_example6() {
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

// Example 7: Data { double=0.01171875, doubles=[0.01171875, 23, 4] }
#[test]
fn test_decode_example7() {
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

// Example 8: Data { fpn=1.82 }
// fpn is integer(2), so 1.82 * 100 = 182 encoded as inline value
#[test]
fn test_decode_example8() {
    let sproto = load_person_data_sproto();
    let data_type = sproto.get_type("Data").unwrap();
    let data = testdata("example8_encoded.bin");

    let decoded = codec::decode(&sproto, data_type, &data).unwrap();
    let map = decoded.as_struct().unwrap();

    // fpn is integer(2), so the raw decoded value is 182
    assert_eq!(map.get("fpn"), Some(&SprotoValue::Integer(182)));
}
