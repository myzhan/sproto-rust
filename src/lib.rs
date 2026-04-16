//! Sproto: A Rust implementation of the sproto binary serialization protocol.
//!
//! Sproto is an efficient binary serialization library, similar to Protocol Buffers
//! but designed for simplicity. It supports a small set of types and provides
//! fast encoding/decoding with optional zero-packing compression.
//!
//! # Quick Start
//!
//! Two encoding approaches are available:
//!
//! - **Serde** (feature `serde`): Use `#[derive(Serialize, Deserialize)]` with
//!   schema-driven `to_bytes` / `from_bytes`.
//! - **Derive macros** (feature `derive`): Use `#[derive(SprotoEncode, SprotoDecode)]`
//!   for compile-time inline wire format code with zero runtime overhead.

pub mod error;
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
pub use derive_traits::{SprotoEncode, SprotoDecode};

// Re-export derive macros when the feature is enabled
#[cfg(feature = "derive")]
pub use sproto_derive::{SprotoEncode, SprotoDecode};
