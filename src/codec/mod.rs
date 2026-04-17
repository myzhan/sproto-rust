pub mod wire;

pub mod decoder;
pub mod encoder;

pub use decoder::{DecodedField, StructArrayIter, StructDecoder};
pub use encoder::{StructArrayEncoder, StructEncoder};

use crate::error::{DecodeError, EncodeError};
use crate::schema_traits::{SchemaDecode, SchemaEncode};
use crate::types::{Sproto, SprotoType};

/// Encode a value to sproto binary format using the Direct API.
///
/// The caller provides the `SprotoType` explicitly, preserving full
/// runtime flexibility (no recompilation needed when schema changes).
///
/// # Example
///
/// ```ignore
/// let st = schema.get_type("Person").unwrap();
/// let bytes = sproto::to_bytes(&schema, st, &person)?;
/// ```
pub fn to_bytes<T: SchemaEncode>(
    schema: &Sproto,
    sproto_type: &SprotoType,
    value: &T,
) -> Result<Vec<u8>, EncodeError> {
    let mut buf = Vec::with_capacity(256);
    {
        let mut enc = StructEncoder::new(schema, sproto_type, &mut buf);
        value.schema_encode(&mut enc)?;
        enc.finish();
    }
    Ok(buf)
}

/// Decode a value from sproto binary format using the Direct API.
///
/// The caller provides the `SprotoType` explicitly, preserving full
/// runtime flexibility (no recompilation needed when schema changes).
///
/// # Example
///
/// ```ignore
/// let st = schema.get_type("Person").unwrap();
/// let person: Person = sproto::from_bytes(&schema, st, &data)?;
/// ```
pub fn from_bytes<T: SchemaDecode>(
    schema: &Sproto,
    sproto_type: &SprotoType,
    data: &[u8],
) -> Result<T, DecodeError> {
    let mut dec = StructDecoder::new(schema, sproto_type, data)?;
    T::schema_decode(&mut dec)
}
