//! Sproto: A Rust implementation of the sproto binary serialization protocol.
//!
//! Sproto is an efficient binary serialization library, similar to Protocol Buffers
//! but designed for simplicity. It supports a small set of types and provides
//! fast encoding/decoding with optional zero-packing compression.
//!
//! # Quick Start
//!
//! Use the low-level `StructEncoder` / `StructDecoder` API:
//!
//! ```ignore
//! let st = schema.get_type("Person").unwrap();
//! let mut buf = Vec::new();
//! let mut enc = sproto::codec::StructEncoder::new(&schema, st, &mut buf);
//! enc.set_string(0, "Alice").unwrap();
//! enc.set_integer(1, 30).unwrap();
//! enc.finish();
//! ```

pub mod binary_schema;
pub mod codec;
pub mod error;
pub mod pack;
pub mod rpc;
pub mod types;

pub use error::SprotoError;
pub use types::Sproto;
