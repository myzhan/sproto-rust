//! Derive macros for sproto serialization.
//!
//! This crate provides `#[derive(SprotoEncode)]` and `#[derive(SprotoDecode)]`
//! macros for automatic sproto serialization without runtime schema lookup.
//!
//! `#[derive(SprotoEncode)]` also generates `SchemaEncode` for the Direct API.
//! `#[derive(SprotoDecode)]` also generates `SchemaDecode` for the Direct API.
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
//! // Derive API (no schema needed):
//! let bytes = person.sproto_encode().unwrap();
//! let decoded = Person::sproto_decode(&bytes).unwrap();
//!
//! // Direct API (schema-driven, 3-parameter):
//! let st = schema.get_type("Person").unwrap();
//! let bytes = sproto::to_bytes(&schema, st, &person).unwrap();
//! let decoded: Person = sproto::from_bytes(&schema, st, &bytes).unwrap();
//! ```

mod attr;
mod decode;
mod encode;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for generating `SprotoEncode` and `SchemaEncode` implementations.
///
/// # Attributes
///
/// - `#[sproto(tag = N)]` - Required on each field, specifies the field tag (u16).
/// - `#[sproto(skip)]` - Optional, skip this field during serialization.
/// - `#[sproto(name = "TypeName")]` - Optional on struct, custom sproto type name.
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

    let encode_impl = encode::derive_encode(&input).unwrap_or_else(|e| e.to_compile_error());
    let schema_impl =
        encode::generate_schema_encode(&input).unwrap_or_else(|e| e.to_compile_error());

    quote! { #encode_impl #schema_impl }.into()
}

/// Derive macro for generating `SprotoDecode` and `SchemaDecode` implementations.
///
/// # Attributes
///
/// - `#[sproto(tag = N)]` - Required on each field, specifies the field tag (u16).
/// - `#[sproto(skip)]` - Optional, skip this field during deserialization (uses Default).
/// - `#[sproto(default)]` - Optional, use Default::default() if field is missing.
/// - `#[sproto(name = "TypeName")]` - Optional on struct, custom sproto type name.
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

    let decode_impl = decode::derive_decode(&input).unwrap_or_else(|e| e.to_compile_error());
    let schema_impl =
        decode::generate_schema_decode(&input).unwrap_or_else(|e| e.to_compile_error());

    quote! { #decode_impl #schema_impl }.into()
}
