//! Schema-driven encode/decode traits for the Direct API.
//!
//! These traits are the Rust equivalent of Go's runtime reflection: they map
//! struct fields to sproto tags at compile time via derive macros, enabling
//! runtime schema-driven encoding without serde.
//!
//! `#[derive(SprotoEncode)]` generates both `SprotoEncode` and `SchemaEncode`.
//! `#[derive(SprotoDecode)]` generates both `SprotoDecode` and `SchemaDecode`.

use crate::codec::{StructDecoder, StructEncoder};
use crate::error::{DecodeError, EncodeError};

/// Trait for types that can be schema-encoded to sproto wire format.
///
/// Used by the Direct API: `sproto::to_bytes(&schema, &sproto_type, &value)`.
/// The caller provides the `SprotoType` explicitly for full runtime flexibility.
pub trait SchemaEncode {
    /// Encode this value's fields into the given `StructEncoder`.
    fn schema_encode(&self, encoder: &mut StructEncoder) -> Result<(), EncodeError>;
}

/// Trait for types that can be schema-decoded from sproto wire format.
///
/// Used by the Direct API: `sproto::from_bytes::<T>(&schema, &sproto_type, &data)`.
/// The caller provides the `SprotoType` explicitly for full runtime flexibility.
pub trait SchemaDecode: Sized {
    /// Decode this type's fields from the given `StructDecoder`.
    fn schema_decode(decoder: &mut StructDecoder) -> Result<Self, DecodeError>;
}
