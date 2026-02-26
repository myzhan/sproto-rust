//! Derive macros for sproto serialization.
//!
//! This crate provides `#[derive(SprotoEncode)]` and `#[derive(SprotoDecode)]`
//! macros for automatic sproto serialization without runtime schema lookup.
//!
//! # Example
//!
//! ```rust,ignore
//! use sproto_derive::{SprotoEncode, SprotoDecode};
//!
//! #[derive(SprotoEncode, SprotoDecode)]
//! struct Person {
//!     #[sproto(tag = 0)]
//!     name: String,
//!     #[sproto(tag = 1)]
//!     age: i64,
//! }
//!
//! let person = Person { name: "Alice".into(), age: 30 };
//! let bytes = person.sproto_encode().unwrap();
//! let decoded = Person::sproto_decode(&bytes).unwrap();
//! ```

mod attr;
mod decode;
mod encode;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for generating `SprotoEncode` implementation.
///
/// # Attributes
///
/// - `#[sproto(tag = N)]` - Required on each field, specifies the field tag (u16).
/// - `#[sproto(skip)]` - Optional, skip this field during serialization.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(SprotoEncode)]
/// struct Person {
///     #[sproto(tag = 0)]
///     name: String,
///     #[sproto(tag = 1)]
///     age: i64,
///     #[sproto(tag = 2)]
///     email: Option<String>,
/// }
/// ```
#[proc_macro_derive(SprotoEncode, attributes(sproto))]
pub fn derive_sproto_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    encode::derive_encode(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for generating `SprotoDecode` implementation.
///
/// # Attributes
///
/// - `#[sproto(tag = N)]` - Required on each field, specifies the field tag (u16).
/// - `#[sproto(skip)]` - Optional, skip this field during deserialization (uses Default).
/// - `#[sproto(default)]` - Optional, use Default::default() if field is missing.
///
/// # Example
///
/// ```rust,ignore
/// #[derive(SprotoDecode)]
/// struct Person {
///     #[sproto(tag = 0)]
///     name: String,
///     #[sproto(tag = 1)]
///     age: i64,
///     #[sproto(tag = 2)]
///     email: Option<String>,
/// }
/// ```
#[proc_macro_derive(SprotoDecode, attributes(sproto))]
pub fn derive_sproto_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    decode::derive_decode(&input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}
