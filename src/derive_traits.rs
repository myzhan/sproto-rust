//! Traits for derive macro-based serialization.
//!
//! These traits are implemented by the `#[derive(SprotoEncode)]` and
//! `#[derive(SprotoDecode)]` macros from the `sproto-derive` crate.

use crate::error::{DecodeError, EncodeError};

/// Trait for types that can be encoded to sproto binary format.
///
/// This trait is typically derived using `#[derive(SprotoEncode)]` from
/// the `sproto-derive` crate.
///
/// # Example
///
/// ```rust,ignore
/// use sproto_derive::SprotoEncode;
///
/// #[derive(SprotoEncode)]
/// struct Person {
///     #[sproto(tag = 0)]
///     name: String,
///     #[sproto(tag = 1)]
///     age: i64,
/// }
///
/// let person = Person { name: "Alice".into(), age: 30 };
/// let bytes = person.sproto_encode().unwrap();
/// ```
pub trait SprotoEncode {
    /// Encode this value to sproto binary format.
    fn sproto_encode(&self) -> Result<Vec<u8>, EncodeError>;
}

/// Trait for types that can be decoded from sproto binary format.
///
/// This trait is typically derived using `#[derive(SprotoDecode)]` from
/// the `sproto-derive` crate.
///
/// # Example
///
/// ```rust,ignore
/// use sproto_derive::SprotoDecode;
///
/// #[derive(SprotoDecode)]
/// struct Person {
///     #[sproto(tag = 0)]
///     name: String,
///     #[sproto(tag = 1)]
///     age: i64,
/// }
///
/// let bytes = /* ... */;
/// let person = Person::sproto_decode(&bytes).unwrap();
/// ```
pub trait SprotoDecode: Sized {
    /// Decode a value from sproto binary format.
    fn sproto_decode(data: &[u8]) -> Result<Self, DecodeError>;
}
