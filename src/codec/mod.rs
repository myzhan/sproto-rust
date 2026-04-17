pub mod wire;

pub mod decoder;
pub mod encoder;

pub use decoder::{DecodedField, StructArrayIter, StructDecoder};
pub use encoder::{StructArrayEncoder, StructEncoder};
