//! Sproto: A Rust implementation of the sproto binary serialization protocol.
//!
//! Sproto is an efficient binary serialization library, similar to Protocol Buffers
//! but designed for simplicity. It supports a small set of types and provides
//! fast encoding/decoding with optional zero-packing compression.
//!
//! # Quick Start
//!
//! Use the derive macros for automatic struct serialization:
//!
//! ```ignore
//! use sproto::{SprotoEncode, SprotoDecode};
//!
//! #[derive(SprotoEncode, SprotoDecode)]
//! struct Person {
//!     #[sproto(tag = 0)]
//!     name: String,
//!     #[sproto(tag = 1)]
//!     age: i64,
//! }
//!
//! let bytes = sproto::to_bytes(&schema, "Person", &person)?;
//! let decoded: Person = sproto::from_bytes(&schema, "Person", &bytes)?;
//! ```

pub mod binary_schema;
pub mod codec;
pub mod derive_traits;
pub mod error;
pub mod pack;
pub mod rpc;
pub mod types;

pub use derive_traits::{from_bytes, to_bytes, SprotoDecode, SprotoEncode};
pub use error::SprotoError;
pub use types::Sproto;

// Re-export derive macros when the `derive` feature is enabled.
// Derive macros and traits share the same name but live in different namespaces.
#[cfg(feature = "derive")]
pub use sproto_derive::{SprotoDecode, SprotoEncode};
