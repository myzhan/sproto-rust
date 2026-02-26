//! Tests for sproto derive macros.

use sproto::{SprotoDecode, SprotoEncode};

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    active: bool,
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct Data {
    #[sproto(tag = 0)]
    numbers: Vec<i64>,
    #[sproto(tag = 1)]
    value: f64,
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct OptionalFields {
    #[sproto(tag = 0)]
    required: String,
    #[sproto(tag = 1)]
    optional: Option<i64>,
}

#[test]
fn test_derive_encode_decode_primitives() {
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
    };

    let bytes = person.sproto_encode().unwrap();
    assert!(!bytes.is_empty());

    let decoded = Person::sproto_decode(&bytes).unwrap();
    assert_eq!(person, decoded);
}

#[test]
fn test_derive_encode_decode_arrays() {
    let data = Data {
        numbers: vec![1, 2, 3, 4, 5],
        value: 3.14,
    };

    let bytes = data.sproto_encode().unwrap();
    let decoded = Data::sproto_decode(&bytes).unwrap();
    assert_eq!(data, decoded);
}

#[test]
fn test_derive_optional_some() {
    let obj = OptionalFields {
        required: "test".into(),
        optional: Some(42),
    };

    let bytes = obj.sproto_encode().unwrap();
    let decoded = OptionalFields::sproto_decode(&bytes).unwrap();
    assert_eq!(obj, decoded);
}

#[test]
fn test_derive_optional_none() {
    let obj = OptionalFields {
        required: "test".into(),
        optional: None,
    };

    let bytes = obj.sproto_encode().unwrap();
    let decoded = OptionalFields::sproto_decode(&bytes).unwrap();
    assert_eq!(obj, decoded);
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct NonContiguousTags {
    #[sproto(tag = 0)]
    first: i64,
    #[sproto(tag = 5)]
    second: i64,
    #[sproto(tag = 10)]
    third: i64,
}

#[test]
fn test_derive_non_contiguous_tags() {
    let obj = NonContiguousTags {
        first: 1,
        second: 2,
        third: 3,
    };

    let bytes = obj.sproto_encode().unwrap();
    let decoded = NonContiguousTags::sproto_decode(&bytes).unwrap();
    assert_eq!(obj, decoded);
}
