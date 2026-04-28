//! Derive traits and convenience functions for struct serialization.
//!
//! Provides `SprotoEncode` / `SprotoDecode` traits and `to_bytes` / `from_bytes`
//! convenience functions for encoding/decoding Rust structs to/from sproto binary.

use crate::codec::{StructDecoder, StructEncoder};
use crate::error::{DecodeError, EncodeError, SprotoError};
use crate::types::Sproto;

/// Trait for encoding a Rust struct into a sproto `StructEncoder`.
///
/// Typically derived via `#[derive(SprotoEncode)]`.
pub trait SprotoEncode {
    /// Encode all fields of this struct into the given encoder.
    fn sproto_encode(&self, enc: &mut StructEncoder) -> Result<(), EncodeError>;
}

/// Trait for decoding a Rust struct from a sproto `StructDecoder`.
///
/// Typically derived via `#[derive(SprotoDecode)]`.
pub trait SprotoDecode: Sized {
    /// Decode a struct by iterating over the decoder's fields.
    fn sproto_decode(dec: &mut StructDecoder) -> Result<Self, DecodeError>;
}

/// Encode a value to sproto binary format.
///
/// # Arguments
/// * `schema` - The sproto schema containing type definitions
/// * `type_name` - The sproto type name to encode as
/// * `value` - The value to encode (must implement `SprotoEncode`)
///
/// # Example
/// ```ignore
/// let bytes = sproto::to_bytes(&schema, "Person", &person)?;
/// ```
pub fn to_bytes<T: SprotoEncode>(
    schema: &Sproto,
    type_name: &str,
    value: &T,
) -> Result<Vec<u8>, SprotoError> {
    let st = schema
        .get_type(type_name)
        .ok_or_else(|| EncodeError::UnknownType(type_name.into()))?;
    let mut buf = Vec::with_capacity(128);
    let mut enc = StructEncoder::new(schema, st, &mut buf);
    value.sproto_encode(&mut enc)?;
    enc.finish();
    Ok(buf)
}

/// Decode a value from sproto binary format.
///
/// # Arguments
/// * `schema` - The sproto schema containing type definitions
/// * `type_name` - The sproto type name to decode from
/// * `data` - The binary data to decode
///
/// # Example
/// ```ignore
/// let person: Person = sproto::from_bytes(&schema, "Person", &bytes)?;
/// ```
pub fn from_bytes<T: SprotoDecode>(
    schema: &Sproto,
    type_name: &str,
    data: &[u8],
) -> Result<T, SprotoError> {
    let st = schema
        .get_type(type_name)
        .ok_or_else(|| DecodeError::UnknownType(type_name.into()))?;
    let mut dec = StructDecoder::new(schema, st, data)?;
    Ok(T::sproto_decode(&mut dec)?)
}
