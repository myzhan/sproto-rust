//! Shared helpers for sproto fuzz targets.
#![allow(dead_code)]

use arbitrary::{Arbitrary, Unstructured};
use sproto::types::{Field, FieldType, Sproto};
use sproto::{SprotoDecode, SprotoEncode};

/// Schema used by the derive round-trip and direct decode fuzz targets.
pub fn build_fuzz_schema() -> Sproto {
    let mut s = Sproto::new();
    let phone_idx = s.add_type(
        "FuzzPhone",
        vec![
            Field::new("number", 0, FieldType::String),
            Field::new("type", 1, FieldType::Integer),
        ],
    );
    s.add_type(
        "FuzzPerson",
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
            // Tag 9 intentionally left empty to exercise skip descriptors.
            Field::array("tags", 10, FieldType::String),
            Field::array("numbers", 11, FieldType::Integer),
            Field::array("flags", 12, FieldType::Boolean),
            Field::array("values", 13, FieldType::Double),
            Field::array("chunks", 14, FieldType::Binary),
        ],
    );
    s
}

/// Non-recursive derive struct used for round-trip fuzzing.
#[derive(Debug, Clone, SprotoEncode, SprotoDecode)]
pub struct FuzzPhone {
    #[sproto(tag = 0)]
    pub number: String,
    #[sproto(tag = 1)]
    pub r#type: i64,
}

/// Non-recursive derive struct used for round-trip fuzzing.
#[derive(Debug, Clone, SprotoEncode, SprotoDecode)]
pub struct FuzzPerson {
    #[sproto(tag = 0)]
    pub name: String,
    #[sproto(tag = 1)]
    pub age: i64,
    #[sproto(tag = 2)]
    pub active: bool,
    #[sproto(tag = 3)]
    pub score: Option<f64>,
    #[sproto(tag = 4)]
    pub photo: Option<Vec<u8>>,
    #[sproto(tag = 5, decimal = 2)]
    pub fpn: f64,
    #[sproto(tag = 6)]
    pub id: i64,
    #[sproto(tag = 7)]
    pub phone: Option<FuzzPhone>,
    #[sproto(tag = 8)]
    pub phones: Vec<FuzzPhone>,
    #[sproto(tag = 10)]
    pub tags: Vec<String>,
    #[sproto(tag = 11)]
    pub numbers: Vec<i64>,
    #[sproto(tag = 12)]
    pub flags: Vec<bool>,
    #[sproto(tag = 13)]
    pub values: Vec<f64>,
    #[sproto(tag = 14)]
    pub chunks: Vec<Vec<u8>>,
}

impl<'a> Arbitrary<'a> for FuzzPhone {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(FuzzPhone {
            number: String::arbitrary(u)?,
            r#type: i64::arbitrary(u)?,
        })
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (0, None)
    }
}

impl<'a> Arbitrary<'a> for FuzzPerson {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(FuzzPerson {
            name: String::arbitrary(u)?,
            age: i64::arbitrary(u)?,
            active: bool::arbitrary(u)?,
            score: Option::<f64>::arbitrary(u)?,
            photo: Option::<Vec<u8>>::arbitrary(u)?,
            fpn: arbitrary_fpn(u)?,
            id: i64::arbitrary(u)?,
            phone: Option::<FuzzPhone>::arbitrary(u)?,
            phones: Vec::<FuzzPhone>::arbitrary(u)?,
            tags: Vec::<String>::arbitrary(u)?,
            numbers: Vec::<i64>::arbitrary(u)?,
            flags: Vec::<bool>::arbitrary(u)?,
            values: Vec::<f64>::arbitrary(u)?,
            chunks: Vec::<Vec<u8>>::arbitrary(u)?,
        })
    }

    fn size_hint(_depth: usize) -> (usize, Option<usize>) {
        (0, None)
    }
}

fn arbitrary_fpn(u: &mut Unstructured) -> arbitrary::Result<f64> {
    // Generate a cent-precision value whose scaled integer fits exactly in an
    // f64 mantissa (<= 2^53). This avoids precision loss during the decimal
    // encode/decode roundtrip.
    let max_scaled = (1_i64 << 53) - 1;
    let mut scaled = i64::arbitrary(u)?;
    scaled = scaled.clamp(-max_scaled, max_scaled);
    Ok(scaled as f64 / 100.0)
}

fn f64_eq(a: f64, b: f64) -> bool {
    if a == b {
        // Treats -0.0 and 0.0 as equal, matching IEEE 754 numeric equality.
        return true;
    }
    // NaN payloads are preserved through the wire format, so fall back to bit
    // equality for NaN values (including differing payloads).
    a.to_bits() == b.to_bits()
}

/// Compare two decimal=2 values by their scaled integer representation.
///
/// Decimal fields round to the nearest cent on encode, so the meaningful
/// roundtrip equality is that both values encode to the same integer.
fn decimal2_eq(a: f64, b: f64) -> bool {
    (a * 100.0).round() as i64 == (b * 100.0).round() as i64
}

fn opt_f64_eq(a: Option<f64>, b: Option<f64>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => f64_eq(x, y),
        (None, None) => true,
        _ => false,
    }
}

fn vec_f64_eq(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| f64_eq(*x, *y))
}

/// Assert that two `FuzzPhone` values are wire-equal.
pub fn assert_phone_eq(a: &FuzzPhone, b: &FuzzPhone) {
    assert_eq!(a.number, b.number, "phone.number mismatch");
    assert_eq!(a.r#type, b.r#type, "phone.type mismatch");
}

/// Assert that two `FuzzPerson` values are wire-equal.
pub fn assert_person_eq(a: &FuzzPerson, b: &FuzzPerson) {
    assert_eq!(a.name, b.name, "name mismatch");
    assert_eq!(a.age, b.age, "age mismatch");
    assert_eq!(a.active, b.active, "active mismatch");
    assert!(opt_f64_eq(a.score, b.score), "score mismatch");
    assert_eq!(a.photo, b.photo, "photo mismatch");
    assert!(decimal2_eq(a.fpn, b.fpn), "fpn mismatch: {} vs {}", a.fpn, b.fpn);
    assert_eq!(a.id, b.id, "id mismatch");
    match (&a.phone, &b.phone) {
        (Some(x), Some(y)) => assert_phone_eq(x, y),
        (None, None) => {}
        _ => panic!("phone mismatch"),
    }
    assert_eq!(a.phones.len(), b.phones.len(), "phones length mismatch");
    for (x, y) in a.phones.iter().zip(b.phones.iter()) {
        assert_phone_eq(x, y);
    }
    assert_eq!(a.tags, b.tags, "tags mismatch");
    assert_eq!(a.numbers, b.numbers, "numbers mismatch");
    assert_eq!(a.flags, b.flags, "flags mismatch");
    assert!(vec_f64_eq(&a.values, &b.values), "values mismatch");
    assert_eq!(a.chunks, b.chunks, "chunks mismatch");
}

/// Schema used by the RPC dispatch fuzz target.
pub fn build_rpc_schema() -> Sproto {
    let mut s = Sproto::new();
    let phone_idx = s.add_type(
        "PhoneNumber",
        vec![
            Field::new("number", 0, FieldType::String),
            Field::new("type", 1, FieldType::Integer),
        ],
    );
    let person_idx = s.add_type(
        "Person",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("id", 1, FieldType::Integer),
            Field::new("phone", 2, FieldType::Struct(phone_idx)),
        ],
    );
    let addressbook_idx = s.add_type(
        "AddressBook",
        vec![Field::array("person", 0, FieldType::Struct(person_idx))],
    );
    s.add_protocol("foo", 0, Some(person_idx), Some(person_idx), false);
    s.add_protocol("bar", 1, Some(addressbook_idx), None, true);
    s
}
