//! Serde integration for sproto serialization.
//!
//! This module provides schema-driven serialization and deserialization
//! using standard serde `#[derive(Serialize, Deserialize)]` traits.
//!
//! # Example
//!
//! ```
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

use crate::types::{Sproto, SprotoType};

/// Serialize a value to sproto binary format using a schema.
///
/// The `sproto` parameter provides the schema metadata, and `sproto_type`
/// specifies which type definition to use for encoding. Field names in
/// the struct must match field names in the schema.
pub fn to_bytes<T: Serialize>(
    sproto: &Sproto,
    sproto_type: &SprotoType,
    value: &T,
) -> Result<Vec<u8>, SerdeError> {
    let mut buf = Vec::with_capacity(256);
    ser::direct_encode(sproto, sproto_type, value, &mut buf)?;
    Ok(buf)
}

/// Deserialize a value from sproto binary format using a schema.
///
/// The `sproto` parameter provides the schema metadata, and `sproto_type`
/// specifies which type definition to use for decoding. Field names in
/// the target struct must match field names in the schema.
pub fn from_bytes<T: for<'de> Deserialize<'de>>(
    sproto: &Sproto,
    sproto_type: &SprotoType,
    data: &[u8],
) -> Result<T, SerdeError> {
    de::direct_decode(sproto, sproto_type, data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
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
}
