//! Sproto: A Rust implementation of the sproto binary serialization protocol.
//!
//! Sproto is an efficient binary serialization library, similar to Protocol Buffers
//! but designed for simplicity. It supports a small set of types and provides
//! fast encoding/decoding with optional zero-packing compression.
//!
//! # Quick Start
//!
//! Three encoding approaches are available:
//!
//! - **Derive macros** (feature `derive`): Use `#[derive(SprotoEncode, SprotoDecode)]`
//!   for compile-time inline wire format code with zero runtime overhead.
//! - **Direct API**: Use `sproto::to_bytes(&schema, &sproto_type, &value)` for
//!   runtime schema-driven encoding without serde dependency.
//! - **Serde** (feature `serde`): Use `#[derive(Serialize, Deserialize)]` with
//!   schema-driven `to_bytes` / `from_bytes`.

pub mod binary_schema;
pub mod codec;
pub mod derive_traits;
pub mod error;
pub mod pack;
pub mod parser;
pub mod rpc;
pub mod schema_traits;
pub mod types;

#[cfg(feature = "serde")]
pub mod serde;

pub use derive_traits::{SprotoDecode, SprotoEncode};
pub use error::SprotoError;
pub use schema_traits::{SchemaDecode, SchemaEncode};
pub use types::Sproto;

// Direct API: 3-parameter encode/decode (schema, type, value)
pub use codec::{from_bytes, to_bytes};

// Re-export derive macros when the feature is enabled
#[cfg(feature = "derive")]
pub use sproto_derive::{SprotoDecode, SprotoEncode};
