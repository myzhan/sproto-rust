//! Integration tests for derive macros (SprotoEncode / SprotoDecode).

use sproto::types::{Field, FieldType, Sproto};
use sproto::{from_bytes, to_bytes, SprotoDecode, SprotoEncode};
use std::collections::HashMap;

// ============================================================================
// Test Schema Setup
// ============================================================================

fn create_person_schema() -> Sproto {
    let mut s = Sproto::new();
    let phone_idx = s.add_type(
        "PhoneNumber",
        vec![
            Field::new("number", 0, FieldType::String),
            Field::new("type", 1, FieldType::Integer),
        ],
    );
    s.add_type(
        "Person",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("age", 1, FieldType::Integer),
            Field::new("active", 2, FieldType::Boolean),
            Field::new("score", 3, FieldType::Double),
            Field::new("photo", 4, FieldType::Binary),
            Field::decimal("fpn", 5, 2),
            Field::new("id", 6, FieldType::Integer),
            Field::new("phone", 7, FieldType::Struct(phone_idx)),
            Field::array("phones", 8, FieldType::Struct(phone_idx)),
            // children references Person itself (index = 1)
            Field::array("children", 9, FieldType::Struct(1)),
            Field::array("tags", 10, FieldType::String),
            Field::array("numbers", 11, FieldType::Integer),
            Field::array("flags", 12, FieldType::Boolean),
            Field::array("values", 13, FieldType::Double),
        ],
    );
    s
}

// ============================================================================
// Derive Structs
// ============================================================================

#[derive(Debug, PartialEq, Clone, SprotoEncode, SprotoDecode)]
struct PhoneNumber {
    #[sproto(tag = 0)]
    number: String,
    #[sproto(tag = 1)]
    r#type: i64,
}

#[derive(Debug, PartialEq, Clone, Default, SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    active: bool,
    #[sproto(tag = 3)]
    score: Option<f64>,
    #[sproto(tag = 4)]
    photo: Option<Vec<u8>>,
    #[sproto(tag = 5, decimal = 2)]
    fpn: f64,
    #[sproto(tag = 6)]
    id: i64,
    #[sproto(tag = 7)]
    phone: Option<PhoneNumber>,
    #[sproto(tag = 8)]
    phones: Vec<PhoneNumber>,
    #[sproto(tag = 9)]
    children: Vec<Person>,
    #[sproto(tag = 10)]
    tags: Vec<String>,
    #[sproto(tag = 11)]
    numbers: Vec<i64>,
    #[sproto(tag = 12)]
    flags: Vec<bool>,
    #[sproto(tag = 13)]
    values: Vec<f64>,
}

// ============================================================================
// Basic Roundtrip Tests
// ============================================================================

#[test]
fn test_derive_simple_scalars() {
    let schema = create_person_schema();
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);
    assert_eq!(decoded.active, true);
}

#[test]
fn test_derive_boolean_false() {
    let schema = create_person_schema();
    let person = Person {
        name: "Bob".into(),
        age: 25,
        active: false,
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.active, false);
}

#[test]
fn test_derive_optional_double() {
    let schema = create_person_schema();
    let person = Person {
        name: "Carol".into(),
        score: Some(3.14159),
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert!((decoded.score.unwrap() - 3.14159).abs() < 0.00001);
}

#[test]
fn test_derive_optional_none() {
    let schema = create_person_schema();
    let person = Person {
        name: "Dave".into(),
        score: None,
        photo: None,
        phone: None,
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.score, None);
    assert_eq!(decoded.photo, None);
    assert_eq!(decoded.phone, None);
}

#[test]
fn test_derive_binary_field() {
    let schema = create_person_schema();
    let person = Person {
        photo: Some(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.photo, Some(vec![0xDE, 0xAD, 0xBE, 0xEF]));
}

#[test]
fn test_derive_decimal_field() {
    let schema = create_person_schema();
    let person = Person {
        fpn: 1.82,
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert!((decoded.fpn - 1.82).abs() < 0.001);
}

#[test]
fn test_derive_large_integer() {
    let schema = create_person_schema();
    let person = Person {
        age: 100000,
        id: -10000000000,
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.age, 100000);
    assert_eq!(decoded.id, -10000000000);
}

// ============================================================================
// Nested Struct Tests
// ============================================================================

#[test]
fn test_derive_nested_struct() {
    let schema = create_person_schema();
    let person = Person {
        name: "Alice".into(),
        phone: Some(PhoneNumber {
            number: "123456789".into(),
            r#type: 1,
        }),
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    let phone = decoded.phone.unwrap();
    assert_eq!(phone.number, "123456789");
    assert_eq!(phone.r#type, 1);
}

#[test]
fn test_derive_struct_array() {
    let schema = create_person_schema();
    let person = Person {
        name: "Alice".into(),
        phones: vec![
            PhoneNumber {
                number: "111".into(),
                r#type: 1,
            },
            PhoneNumber {
                number: "222".into(),
                r#type: 2,
            },
        ],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.phones.len(), 2);
    assert_eq!(decoded.phones[0].number, "111");
    assert_eq!(decoded.phones[0].r#type, 1);
    assert_eq!(decoded.phones[1].number, "222");
    assert_eq!(decoded.phones[1].r#type, 2);
}

#[test]
fn test_derive_recursive_struct() {
    let schema = create_person_schema();
    let person = Person {
        name: "Bob".into(),
        age: 40,
        children: vec![
            Person {
                name: "Alice".into(),
                age: 13,
                ..Default::default()
            },
            Person {
                name: "Carol".into(),
                age: 5,
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.name, "Bob");
    assert_eq!(decoded.age, 40);
    assert_eq!(decoded.children.len(), 2);
    assert_eq!(decoded.children[0].name, "Alice");
    assert_eq!(decoded.children[0].age, 13);
    assert_eq!(decoded.children[1].name, "Carol");
    assert_eq!(decoded.children[1].age, 5);
}

// ============================================================================
// Array Tests
// ============================================================================

#[test]
fn test_derive_integer_array() {
    let schema = create_person_schema();
    let person = Person {
        numbers: vec![1, 2, 3, 4, 5],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.numbers, vec![1, 2, 3, 4, 5]);
}

#[test]
fn test_derive_big_integer_array() {
    let schema = create_person_schema();
    let person = Person {
        numbers: vec![(1 << 32) + 1, (1 << 32) + 2, (1 << 32) + 3],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(
        decoded.numbers,
        vec![(1 << 32) + 1, (1 << 32) + 2, (1 << 32) + 3]
    );
}

#[test]
fn test_derive_bool_array() {
    let schema = create_person_schema();
    let person = Person {
        flags: vec![true, false, true, false],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.flags, vec![true, false, true, false]);
}

#[test]
fn test_derive_double_array() {
    let schema = create_person_schema();
    let person = Person {
        values: vec![0.01171875, 23.0, 4.0],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.values.len(), 3);
    assert!((decoded.values[0] - 0.01171875).abs() < 0.0000001);
    assert!((decoded.values[1] - 23.0).abs() < 0.0000001);
    assert!((decoded.values[2] - 4.0).abs() < 0.0000001);
}

#[test]
fn test_derive_string_array() {
    let schema = create_person_schema();
    let person = Person {
        tags: vec!["hello".into(), "world".into(), "\u{4f60}\u{597d}".into()],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.tags, vec!["hello", "world", "\u{4f60}\u{597d}"]);
}

#[test]
fn test_derive_empty_arrays() {
    let schema = create_person_schema();
    let person = Person {
        numbers: vec![],
        flags: vec![],
        tags: vec![],
        values: vec![],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert!(decoded.numbers.is_empty());
    assert!(decoded.flags.is_empty());
    assert!(decoded.tags.is_empty());
    assert!(decoded.values.is_empty());
}

// ============================================================================
// Full Roundtrip
// ============================================================================

#[test]
fn test_derive_full_roundtrip() {
    let schema = create_person_schema();
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
        score: Some(0.01171875),
        photo: Some(vec![0xDE, 0xAD, 0xBE, 0xEF]),
        fpn: 1.82,
        id: 10000,
        phone: Some(PhoneNumber {
            number: "123456789".into(),
            r#type: 1,
        }),
        phones: vec![
            PhoneNumber {
                number: "123456789".into(),
                r#type: 1,
            },
            PhoneNumber {
                number: "87654321".into(),
                r#type: 2,
            },
        ],
        children: vec![Person {
            name: "Bob".into(),
            age: 5,
            ..Default::default()
        }],
        tags: vec!["hello".into(), "world".into(), "\u{4f60}\u{597d}".into()],
        numbers: vec![1, 2, 3, 4, 5],
        flags: vec![false, true, false],
        values: vec![0.01171875, 23.0, 4.0],
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &bytes).unwrap();

    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);
    assert_eq!(decoded.active, true);
    assert!((decoded.score.unwrap() - 0.01171875).abs() < 0.0000001);
    assert_eq!(decoded.photo, Some(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    assert!((decoded.fpn - 1.82).abs() < 0.001);
    assert_eq!(decoded.id, 10000);
    assert_eq!(decoded.phone.as_ref().unwrap().number, "123456789");
    assert_eq!(decoded.phones.len(), 2);
    assert_eq!(decoded.children.len(), 1);
    assert_eq!(decoded.children[0].name, "Bob");
    assert_eq!(decoded.tags, vec!["hello", "world", "\u{4f60}\u{597d}"]);
    assert_eq!(decoded.numbers, vec![1, 2, 3, 4, 5]);
    assert_eq!(decoded.flags, vec![false, true, false]);
    assert_eq!(decoded.values.len(), 3);
}

// ============================================================================
// Cross-compatibility with C fixtures
// ============================================================================

fn load_binary_schema() -> Sproto {
    let bytes = std::fs::read("tests/testdata/schema.bin").unwrap();
    sproto::binary_schema::load_binary(&bytes).unwrap()
}

#[test]
fn test_derive_decode_simple_struct_fixture() {
    let schema = load_binary_schema();
    let encoded = std::fs::read("tests/testdata/simple_struct_encoded.bin").unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &encoded).unwrap();

    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 13);
    assert_eq!(decoded.active, false);
}

#[test]
fn test_derive_decode_all_scalars_fixture() {
    let schema = load_binary_schema();
    let encoded = std::fs::read("tests/testdata/all_scalars_encoded.bin").unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &encoded).unwrap();

    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);
    assert_eq!(decoded.active, true);
    assert!((decoded.score.unwrap() - 0.01171875).abs() < 0.0000001);
    assert_eq!(decoded.photo, Some(vec![0x28, 0x29, 0x30, 0x31]));
    assert!((decoded.fpn - 1.82).abs() < 0.001);
}

#[test]
fn test_derive_decode_nested_struct_fixture() {
    let schema = load_binary_schema();
    let encoded = std::fs::read("tests/testdata/nested_struct_encoded.bin").unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &encoded).unwrap();

    assert_eq!(decoded.name, "Alice");
    let phone = decoded.phone.unwrap();
    assert_eq!(phone.number, "123456789");
    assert_eq!(phone.r#type, 1);
}

#[test]
fn test_derive_decode_struct_array_fixture() {
    let schema = load_binary_schema();
    let encoded = std::fs::read("tests/testdata/struct_array_encoded.bin").unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &encoded).unwrap();

    assert_eq!(decoded.name, "Bob");
    assert_eq!(decoded.age, 40);
    assert_eq!(decoded.children.len(), 2);
    assert_eq!(decoded.children[0].name, "Alice");
    assert_eq!(decoded.children[0].age, 13);
    assert_eq!(decoded.children[1].name, "Carol");
    assert_eq!(decoded.children[1].age, 5);
}

#[test]
fn test_derive_decode_full_fixture() {
    let schema = load_binary_schema();
    let encoded = std::fs::read("tests/testdata/full_encoded.bin").unwrap();
    let decoded: Person = from_bytes(&schema, "Person", &encoded).unwrap();

    assert_eq!(decoded.name, "Alice");
    assert_eq!(decoded.age, 30);
    assert_eq!(decoded.active, true);
    assert!((decoded.score.unwrap() - 0.01171875).abs() < 0.0000001);
    assert_eq!(decoded.photo, Some(vec![0xDE, 0xAD, 0xBE, 0xEF]));
    assert!((decoded.fpn - 1.82).abs() < 0.001);
    assert_eq!(decoded.id, 10000);
    assert_eq!(decoded.phone.as_ref().unwrap().number, "123456789");
    assert_eq!(decoded.phones.len(), 2);
    assert_eq!(decoded.children.len(), 1);
    assert_eq!(decoded.children[0].name, "Bob");
    assert_eq!(decoded.tags.len(), 3);
    assert_eq!(decoded.numbers, vec![1, 2, 3, 4, 5]);
    assert_eq!(decoded.flags, vec![false, true, false]);
    assert_eq!(decoded.values.len(), 3);
}

// ============================================================================
// Binary Compatibility: encode with derive then decode with direct API
// ============================================================================

#[test]
fn test_derive_encode_decode_consistency() {
    // Verify that derive-encoded data can be decoded by direct API
    // and matches the original values
    use sproto::codec::StructDecoder;

    let schema = load_binary_schema();
    let person = Person {
        name: "Alice".into(),
        age: 13,
        active: false,
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();

    // Decode with direct API to verify wire format is correct
    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &bytes).unwrap();
    let mut found_name = false;
    let mut found_age = false;
    let mut found_active = false;
    while let Some(f) = dec.next_field().unwrap() {
        match f.tag() {
            0 => {
                assert_eq!(f.as_string().unwrap(), "Alice");
                found_name = true;
            }
            1 => {
                assert_eq!(f.as_integer().unwrap(), 13);
                found_age = true;
            }
            2 => {
                assert_eq!(f.as_bool().unwrap(), false);
                found_active = true;
            }
            _ => {} // other default fields may be present
        }
    }
    assert!(found_name && found_age && found_active);
}

#[test]
fn test_derive_encode_struct_array_decode_consistency() {
    use sproto::codec::StructDecoder;

    let schema = load_binary_schema();
    let person = Person {
        name: "Bob".into(),
        age: 40,
        children: vec![
            Person {
                name: "Alice".into(),
                age: 13,
                ..Default::default()
            },
            Person {
                name: "Carol".into(),
                age: 5,
                ..Default::default()
            },
        ],
        ..Default::default()
    };

    let bytes = to_bytes(&schema, "Person", &person).unwrap();

    // Decode with direct API
    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &bytes).unwrap();
    let mut name = String::new();
    let mut children_names: Vec<String> = Vec::new();
    while let Some(f) = dec.next_field().unwrap() {
        match f.tag() {
            0 => name = f.as_string().unwrap().to_owned(),
            9 => {
                for elem in f.as_struct_iter().unwrap() {
                    let mut sub = elem.unwrap();
                    while let Some(sf) = sub.next_field().unwrap() {
                        if sf.tag() == 0 {
                            children_names.push(sf.as_string().unwrap().to_owned());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    assert_eq!(name, "Bob");
    assert_eq!(children_names, vec!["Alice", "Carol"]);
}

#[test]
fn test_derive_unknown_type_error() {
    let schema = create_person_schema();
    let person = Person {
        name: "Test".into(),
        ..Default::default()
    };

    let result = to_bytes(&schema, "Unknown", &person);
    assert!(result.is_err());
}

// ============================================================================
// Map Tests — *Type(key) indexed map and *Type() anonymous map
// ============================================================================

fn create_map_schema() -> Sproto {
    let mut s = Sproto::new();
    // PhoneNumber: number(tag=0, string), type(tag=1, integer)
    let phone_idx = s.add_type(
        "PhoneNumber",
        vec![
            Field::new("number", 0, FieldType::String),
            Field::new("type", 1, FieldType::Integer),
        ],
    );
    // Person: name(tag=0), id(tag=1)
    let person_idx = s.add_type(
        "Person",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("id", 1, FieldType::Integer),
        ],
    );
    // AddressBook: person(tag=0, *Person(id)), phonemap(tag=1, *PhoneNumber())
    let mut person_field = Field::array("person", 0, FieldType::Struct(person_idx));
    person_field.key_tag = 1; // key is Person.id
    let mut phonemap_field = Field::array("phonemap", 1, FieldType::Struct(phone_idx));
    phonemap_field.key_tag = 0; // key is PhoneNumber.number
    phonemap_field.is_map = true;
    s.add_type("AddressBook", vec![person_field, phonemap_field]);
    s
}

#[derive(Debug, PartialEq, Clone, Default, SprotoEncode, SprotoDecode)]
struct MapPerson {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    id: i64,
}

#[derive(Debug, PartialEq, Clone, Default, SprotoEncode, SprotoDecode)]
struct MapAddressBook {
    #[sproto(tag = 0, key = 1, key_field = "id")]
    person: HashMap<i64, MapPerson>,
    #[sproto(tag = 1, key = 0, value = 1)]
    phonemap: HashMap<String, i64>,
}

#[test]
fn test_derive_indexed_map_roundtrip() {
    let schema = create_map_schema();

    let mut person_map = HashMap::new();
    person_map.insert(
        10000,
        MapPerson {
            name: "Alice".into(),
            id: 10000,
        },
    );
    person_map.insert(
        20000,
        MapPerson {
            name: "Bob".into(),
            id: 20000,
        },
    );

    let book = MapAddressBook {
        person: person_map,
        phonemap: HashMap::new(),
    };

    let bytes = to_bytes(&schema, "AddressBook", &book).unwrap();
    let decoded: MapAddressBook = from_bytes(&schema, "AddressBook", &bytes).unwrap();

    assert_eq!(decoded.person.len(), 2);
    assert_eq!(decoded.person[&10000].name, "Alice");
    assert_eq!(decoded.person[&20000].name, "Bob");
}

#[test]
fn test_derive_anonymous_map_roundtrip() {
    let schema = create_map_schema();

    let mut phonemap = HashMap::new();
    phonemap.insert("123456789".to_string(), 1i64);
    phonemap.insert("87654321".to_string(), 2i64);

    let book = MapAddressBook {
        person: HashMap::new(),
        phonemap,
    };

    let bytes = to_bytes(&schema, "AddressBook", &book).unwrap();
    let decoded: MapAddressBook = from_bytes(&schema, "AddressBook", &bytes).unwrap();

    assert_eq!(decoded.phonemap.len(), 2);
    assert_eq!(decoded.phonemap["123456789"], 1);
    assert_eq!(decoded.phonemap["87654321"], 2);
}

#[test]
fn test_derive_both_maps_roundtrip() {
    let schema = create_map_schema();

    let mut person_map = HashMap::new();
    person_map.insert(
        10000,
        MapPerson {
            name: "Alice".into(),
            id: 10000,
        },
    );

    let mut phonemap = HashMap::new();
    phonemap.insert("123456789".to_string(), 1i64);

    let book = MapAddressBook {
        person: person_map,
        phonemap,
    };

    let bytes = to_bytes(&schema, "AddressBook", &book).unwrap();
    let decoded: MapAddressBook = from_bytes(&schema, "AddressBook", &bytes).unwrap();

    assert_eq!(decoded.person.len(), 1);
    assert_eq!(decoded.person[&10000].name, "Alice");
    assert_eq!(decoded.phonemap.len(), 1);
    assert_eq!(decoded.phonemap["123456789"], 1);
}

#[test]
fn test_derive_empty_maps() {
    let schema = create_map_schema();

    let book = MapAddressBook {
        person: HashMap::new(),
        phonemap: HashMap::new(),
    };

    let bytes = to_bytes(&schema, "AddressBook", &book).unwrap();
    let decoded: MapAddressBook = from_bytes(&schema, "AddressBook", &bytes).unwrap();

    assert!(decoded.person.is_empty());
    assert!(decoded.phonemap.is_empty());
}

// ============================================================================
// Cross-compatibility with C fixtures for maps
// ============================================================================

#[derive(Debug, PartialEq, Clone, Default, SprotoEncode, SprotoDecode)]
struct FixtureAddressBook {
    #[sproto(tag = 0, key = 6, key_field = "id")]
    person: HashMap<i64, Person>,
    #[sproto(tag = 1, key = 0, value = 1)]
    phonemap: HashMap<String, i64>,
}

#[test]
fn test_derive_decode_indexed_map_fixture() {
    let schema = load_binary_schema();
    let encoded = std::fs::read("tests/testdata/indexed_map_encoded.bin").unwrap();
    let decoded: FixtureAddressBook = from_bytes(&schema, "AddressBook", &encoded).unwrap();

    assert_eq!(decoded.person.len(), 2);
    assert_eq!(decoded.person[&10000].name, "Alice");
    assert_eq!(decoded.person[&10000].id, 10000);
    assert_eq!(decoded.person[&20000].name, "Bob");
    assert_eq!(decoded.person[&20000].id, 20000);
}

#[test]
fn test_derive_decode_anonymous_map_fixture() {
    let schema = load_binary_schema();
    let encoded = std::fs::read("tests/testdata/anonymous_map_encoded.bin").unwrap();
    let decoded: FixtureAddressBook = from_bytes(&schema, "AddressBook", &encoded).unwrap();

    assert_eq!(decoded.phonemap.len(), 2);
    assert_eq!(decoded.phonemap["123456789"], 1);
    assert_eq!(decoded.phonemap["87654321"], 2);
}

#[test]
fn test_derive_decode_both_maps_fixture() {
    let schema = load_binary_schema();
    let encoded = std::fs::read("tests/testdata/both_maps_encoded.bin").unwrap();
    let decoded: FixtureAddressBook = from_bytes(&schema, "AddressBook", &encoded).unwrap();

    assert_eq!(decoded.person.len(), 1);
    assert_eq!(decoded.person[&10000].name, "Alice");
    assert_eq!(decoded.phonemap.len(), 1);
    assert_eq!(decoded.phonemap["123456789"], 1);
}

#[test]
fn test_derive_encode_indexed_map_fixture() {
    let schema = load_binary_schema();

    let mut person_map = HashMap::new();
    person_map.insert(
        10000,
        Person {
            name: "Alice".into(),
            id: 10000,
            ..Default::default()
        },
    );
    person_map.insert(
        20000,
        Person {
            name: "Bob".into(),
            id: 20000,
            ..Default::default()
        },
    );

    let book = FixtureAddressBook {
        person: person_map,
        phonemap: HashMap::new(),
    };

    let bytes = to_bytes(&schema, "AddressBook", &book).unwrap();
    let decoded: FixtureAddressBook = from_bytes(&schema, "AddressBook", &bytes).unwrap();

    assert_eq!(decoded.person.len(), 2);
    assert_eq!(decoded.person[&10000].name, "Alice");
    assert_eq!(decoded.person[&20000].name, "Bob");
}

#[test]
fn test_derive_encode_anonymous_map_fixture() {
    let schema = load_binary_schema();

    let mut phonemap = HashMap::new();
    phonemap.insert("123456789".to_string(), 1i64);
    phonemap.insert("87654321".to_string(), 2i64);

    let book = FixtureAddressBook {
        person: HashMap::new(),
        phonemap,
    };

    let bytes = to_bytes(&schema, "AddressBook", &book).unwrap();
    let decoded: FixtureAddressBook = from_bytes(&schema, "AddressBook", &bytes).unwrap();

    assert_eq!(decoded.phonemap.len(), 2);
    assert_eq!(decoded.phonemap["123456789"], 1);
    assert_eq!(decoded.phonemap["87654321"], 2);
}
