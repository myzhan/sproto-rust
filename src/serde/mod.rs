//! Serde integration for sproto serialization.
//!
//! This module provides schema-driven serialization and deserialization
//! using standard serde `#[derive(Serialize, Deserialize)]` traits.
//!
//! # Example
//!
//! ```rust,ignore
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! struct Person {
//!     name: String,
//!     age: i64,
//! }
//!
//! let sproto = sproto::parser::parse(r#"
//!     .Person {
//!         name 0 : string
//!         age 1 : integer
//!     }
//! "#).unwrap();
//!
//! let person_type = sproto.get_type("Person").unwrap();
//! let person = Person { name: "Alice".into(), age: 30 };
//!
//! let bytes = sproto::serde::to_bytes(&sproto, person_type, &person).unwrap();
//! let decoded: Person = sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();
//! assert_eq!(person, decoded);
//! ```

mod de;
mod error;
mod ser;

pub use error::SerdeError;

use serde::{Deserialize, Serialize};

use crate::codec;
use crate::types::{Sproto, SprotoType};

/// Serialize a value to sproto binary format using a schema.
///
/// The `sproto` parameter provides the schema metadata, and `sproto_type`
/// specifies which type definition to use for encoding. Field names in
/// the struct must match field names in the schema.
///
/// # Example
///
/// ```rust,ignore
/// let bytes = sproto::serde::to_bytes(&sproto, person_type, &person)?;
/// ```
pub fn to_bytes<T: Serialize>(
    sproto: &Sproto,
    sproto_type: &SprotoType,
    value: &T,
) -> Result<Vec<u8>, SerdeError> {
    // First serialize the Rust value to SprotoValue
    let sproto_value = ser::SprotoSerializer::serialize(value)?;

    // Then encode using the existing codec
    let bytes = codec::encode(sproto, sproto_type, &sproto_value)?;

    Ok(bytes)
}

/// Deserialize a value from sproto binary format using a schema.
///
/// The `sproto` parameter provides the schema metadata, and `sproto_type`
/// specifies which type definition to use for decoding. Field names in
/// the target struct must match field names in the schema.
///
/// # Example
///
/// ```rust,ignore
/// let person: Person = sproto::serde::from_bytes(&sproto, person_type, &bytes)?;
/// ```
pub fn from_bytes<T: for<'de> Deserialize<'de>>(
    sproto: &Sproto,
    sproto_type: &SprotoType,
    data: &[u8],
) -> Result<T, SerdeError> {
    // First decode using the existing codec to get SprotoValue
    let sproto_value = codec::decode(sproto, sproto_type, data)?;

    // Then deserialize the SprotoValue to the target Rust type
    de::SprotoDeserializer::deserialize(&sproto_value)
}

/// Serialize a value to SprotoValue without encoding to bytes.
///
/// This is useful when you want to inspect or manipulate the intermediate
/// representation before encoding.
pub fn to_value<T: Serialize>(value: &T) -> Result<crate::value::SprotoValue, SerdeError> {
    ser::SprotoSerializer::serialize(value)
}

/// Deserialize a SprotoValue to a Rust type.
///
/// This is useful when you already have a SprotoValue from another source
/// (e.g., decoded from bytes using the lower-level API).
pub fn from_value<T: for<'de> Deserialize<'de>>(
    value: &crate::value::SprotoValue,
) -> Result<T, SerdeError> {
    de::SprotoDeserializer::deserialize(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use crate::value::SprotoValue;
    use serde::{Deserialize, Serialize};

    fn test_schema() -> Sproto {
        parser::parse(
            r#"
            .Person {
                name 0 : string
                age 1 : integer
                active 2 : boolean
            }
            .Data {
                numbers 0 : *integer
                value 1 : double
            }
            .Nested {
                person 0 : Person
                count 1 : integer
            }
        "#,
        )
        .unwrap()
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Person {
        name: String,
        age: i64,
        active: bool,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Data {
        numbers: Vec<i64>,
        value: f64,
    }

    #[test]
    fn test_serialize_primitives() {
        let sproto = test_schema();
        let person_type = sproto.get_type("Person").unwrap();

        let person = Person {
            name: "Alice".into(),
            age: 30,
            active: true,
        };

        let bytes = to_bytes(&sproto, person_type, &person).unwrap();
        assert!(!bytes.is_empty());

        // Decode and verify
        let decoded: Person = from_bytes(&sproto, person_type, &bytes).unwrap();
        assert_eq!(person, decoded);
    }

    #[test]
    fn test_serialize_array() {
        let sproto = test_schema();
        let data_type = sproto.get_type("Data").unwrap();

        let data = Data {
            numbers: vec![1, 2, 3, 4, 5],
            value: 3.14,
        };

        let bytes = to_bytes(&sproto, data_type, &data).unwrap();
        let decoded: Data = from_bytes(&sproto, data_type, &bytes).unwrap();
        assert_eq!(data, decoded);
    }

    #[test]
    fn test_to_value() {
        let person = Person {
            name: "Bob".into(),
            age: 25,
            active: false,
        };

        let value = to_value(&person).unwrap();

        match value {
            SprotoValue::Struct(map) => {
                assert_eq!(map.get("name"), Some(&SprotoValue::Str("Bob".into())));
                assert_eq!(map.get("age"), Some(&SprotoValue::Integer(25)));
                assert_eq!(map.get("active"), Some(&SprotoValue::Boolean(false)));
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn test_from_value() {
        let value = SprotoValue::from_fields(vec![
            ("name", "Carol".into()),
            ("age", 35i64.into()),
            ("active", true.into()),
        ]);

        let person: Person = from_value(&value).unwrap();
        assert_eq!(person.name, "Carol");
        assert_eq!(person.age, 35);
        assert!(person.active);
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct OptionalFields {
        required: String,
        optional: Option<i64>,
    }

    #[test]
    fn test_optional_some() {
        let obj = OptionalFields {
            required: "test".into(),
            optional: Some(42),
        };

        let value = to_value(&obj).unwrap();
        match value {
            SprotoValue::Struct(map) => {
                assert_eq!(map.get("required"), Some(&SprotoValue::Str("test".into())));
                assert_eq!(map.get("optional"), Some(&SprotoValue::Integer(42)));
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn test_optional_none() {
        let obj = OptionalFields {
            required: "test".into(),
            optional: None,
        };

        let value = to_value(&obj).unwrap();
        match value {
            SprotoValue::Struct(map) => {
                assert_eq!(map.get("required"), Some(&SprotoValue::Str("test".into())));
                assert!(map.get("optional").is_none()); // None fields are omitted
            }
            _ => panic!("expected struct"),
        }
    }

    #[test]
    fn test_integer_types() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct IntTypes {
            i8_val: i8,
            i16_val: i16,
            i32_val: i32,
            i64_val: i64,
            u8_val: u8,
            u16_val: u16,
            u32_val: u32,
        }

        let obj = IntTypes {
            i8_val: -10,
            i16_val: -1000,
            i32_val: -100000,
            i64_val: -1000000000,
            u8_val: 200,
            u16_val: 50000,
            u32_val: 3000000000,
        };

        let value = to_value(&obj).unwrap();
        let decoded: IntTypes = from_value(&value).unwrap();
        assert_eq!(obj, decoded);
    }
}
