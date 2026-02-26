//! Sproto: A Rust implementation of the sproto binary serialization protocol.
//!
//! Sproto is an efficient binary serialization library, similar to Protocol Buffers
//! but designed for simplicity. It supports a small set of types and provides
//! fast encoding/decoding with optional zero-packing compression.
//!
//! # Quick Start
//!
//! ```rust
//! use sproto::parser;
//! use sproto::value::SprotoValue;
//! use sproto::codec;
//!
//! let sproto = parser::parse(r#"
//!     .Person {
//!         name 0 : string
//!         age 1 : integer
//!     }
//! "#).unwrap();
//!
//! let person_type = sproto.get_type("Person").unwrap();
//! let value = SprotoValue::from_fields(vec![
//!     ("name", "Alice".into()),
//!     ("age", 30i64.into()),
//! ]);
//!
//! let encoded = codec::encode(&sproto, person_type, &value).unwrap();
//! let decoded = codec::decode(&sproto, person_type, &encoded).unwrap();
//! assert_eq!(value, decoded);
//! ```

pub mod error;
pub mod value;
pub mod types;
pub mod codec;
pub mod pack;
pub mod parser;
pub mod binary_schema;
pub mod rpc;
pub mod derive_traits;

#[cfg(feature = "serde")]
pub mod serde;

pub use error::SprotoError;
pub use types::Sproto;
pub use value::SprotoValue;
pub use derive_traits::{SprotoEncode, SprotoDecode};

// Re-export derive macros when the feature is enabled
#[cfg(feature = "derive")]
pub use sproto_derive::{SprotoEncode, SprotoDecode};
