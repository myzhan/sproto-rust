# Serde Integration for sproto-rust

**Status: COMPLETED**

## Overview

Added serde compatibility with two complementary approaches:
1. **Schema-Driven** - Standard `#[derive(Serialize, Deserialize)]` with runtime schema lookup
2. **Attribute-Based** - Custom `#[derive(SprotoEncode, SprotoDecode)]` with compile-time tags

Both approaches bridge through `SprotoValue` to reuse existing codec logic.

---

## Approach 1: Schema-Driven Serde (Implemented)

### Usage Example
```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Person {
    name: String,
    age: i64,
}

let sproto = parser::parse(".Person { name 0 : string  age 1 : integer }").unwrap();
let person_type = sproto.get_type("Person").unwrap();

let person = Person { name: "Alice".into(), age: 30 };

let bytes = sproto::serde::to_bytes(&sproto, person_type, &person).unwrap();
let decoded: Person = sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();
assert_eq!(person, decoded);
```

### Implementation Files

**src/serde/mod.rs** - Public API
```rust
pub fn to_bytes<T: Serialize>(sproto: &Sproto, ty: &SprotoType, value: &T) -> Result<Vec<u8>, SerdeError>
pub fn from_bytes<T: for<'de> Deserialize<'de>>(sproto: &Sproto, ty: &SprotoType, data: &[u8]) -> Result<T, SerdeError>
pub fn to_value<T: Serialize>(value: &T) -> Result<SprotoValue, SerdeError>
pub fn from_value<T: for<'de> Deserialize<'de>>(value: &SprotoValue) -> Result<T, SerdeError>
```

**src/serde/ser.rs** - SprotoSerializer
- Implements `serde::Serializer` trait
- `serialize_struct` collects fields into `HashMap<String, SprotoValue>`
- Type conversions: i64→Integer, bool→Boolean, f64→Double, String→Str, Vec<u8>→Binary
- `Option::None` = omit field, `Option::Some(v)` = serialize v
- Returns `SprotoValue::Struct`, then caller uses `codec::encode`

**src/serde/de.rs** - SprotoDeserializer
- Implements `serde::Deserializer` trait
- First calls `codec::decode` to get `SprotoValue::Struct(HashMap)`
- `deserialize_struct` visits HashMap fields by name
- Converts SprotoValue variants back to Rust types via visitors
- Uses `MapAccess` and `SeqAccess` for complex types

**src/serde/error.rs** - SerdeError
```rust
#[derive(Debug, thiserror::Error)]
pub enum SerdeError {
    #[error("type mismatch for field '{field}': expected {expected}, got {actual}")]
    TypeMismatch { field: String, expected: String, actual: String },
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("unsupported type: {0}")]
    UnsupportedType(String),
    #[error("encode error: {0}")]
    Encode(#[from] EncodeError),
    #[error("decode error: {0}")]
    Decode(#[from] DecodeError),
    #[error("{0}")]
    Custom(String),
}
impl serde::ser::Error + serde::de::Error for SerdeError
```

### Key Implementation Details
- Uses higher-ranked trait bounds (`for<'de> Deserialize<'de>`) to handle owned data
- Field names must match schema exactly
- Nested structs: recursively serialize/deserialize through SprotoValue
- Arrays: `Vec<T>` → `SprotoValue::Array`
- Optional fields: `Option<T>` with None values omitted

---

## Approach 2: Attribute-Based Proc-Macro (Implemented)

### Usage Example
```rust
use sproto::{SprotoEncode, SprotoDecode};

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    active: bool,
}

let person = Person { name: "Alice".into(), age: 30, active: true };
let bytes = person.sproto_encode().unwrap();
let decoded = Person::sproto_decode(&bytes).unwrap();
assert_eq!(person, decoded);
```

### Crate Structure

**sproto-derive/Cargo.toml**
```toml
[package]
name = "sproto-derive"
version = "0.1.0"
edition = "2021"

[lib]
proc-macro = true

[dependencies]
syn = { version = "2", features = ["full"] }
quote = "1"
proc-macro2 = "1"
```

**sproto-derive/src/lib.rs** - Entry points
```rust
#[proc_macro_derive(SprotoEncode, attributes(sproto))]
pub fn derive_encode(input: TokenStream) -> TokenStream

#[proc_macro_derive(SprotoDecode, attributes(sproto))]
pub fn derive_decode(input: TokenStream) -> TokenStream
```

**sproto-derive/src/attr.rs** - Attribute parsing
- Parse `#[sproto(tag = N)]`, `#[sproto(skip)]`, `#[sproto(default)]`
- Validate: all fields have tag, tags unique, tags fit u16
- `FieldInfo` struct with ident, tag, is_optional, is_vec, skip flags

**sproto-derive/src/encode.rs** - SprotoEncode implementation
- Compile-time type detection: Rust types → `FieldType`
- Build `SprotoValue::Struct(HashMap)` from struct fields
- Generate inline `SprotoType` with correct field types
- Calculate `maxn` including skip markers for non-contiguous tags
- Call `codec::encode` with generated schema

**sproto-derive/src/decode.rs** - SprotoDecode implementation
- Generate inline schema with compile-time types
- Call `codec::decode` to get `SprotoValue::Struct`
- Extract fields from HashMap by name using `TryFrom`
- Handle optional fields with `Option::transpose()`

### Main Crate Changes

**src/derive_traits.rs** - Trait definitions
```rust
pub trait SprotoEncode {
    fn sproto_encode(&self) -> Result<Vec<u8>, EncodeError>;
}

pub trait SprotoDecode: Sized {
    fn sproto_decode(data: &[u8]) -> Result<Self, DecodeError>;
}
```

**src/value.rs** - TryFrom implementations
```rust
impl TryFrom<SprotoValue> for i64 { ... }
impl TryFrom<SprotoValue> for String { ... }
impl TryFrom<SprotoValue> for bool { ... }
impl TryFrom<SprotoValue> for f64 { ... }
impl TryFrom<SprotoValue> for Vec<u8> { ... }
impl TryFrom<SprotoValue> for Vec<i64> { ... }
impl TryFrom<SprotoValue> for Vec<f64> { ... }
impl TryFrom<SprotoValue> for Vec<String> { ... }
impl TryFrom<SprotoValue> for Vec<bool> { ... }
```

**Cargo.toml** - Features
```toml
[features]
default = ["derive"]
serde = ["dep:serde"]
derive = ["dep:sproto-derive", "serde"]

[dependencies]
serde = { version = "1", optional = true }
sproto-derive = { path = "sproto-derive", optional = true }
```

---

## Type Mappings

| Rust Type | SprotoValue | FieldType | Wire Format |
|-----------|-------------|-----------|-------------|
| i8/i16/i32/i64/u8/u16/u32/u64 | Integer(i64) | Integer | inline or 4/8 byte |
| bool | Boolean(bool) | Boolean | inline 0/1 |
| f32/f64 | Double(f64) | Double | 8 byte IEEE 754 |
| String/&str | Str(String) | String | length-prefixed UTF-8 |
| Vec<u8> | Binary(Vec<u8>) | Binary | length-prefixed bytes |
| Vec<T> | Array(Vec<SprotoValue>) | (element type) | element-prefixed array |
| Option<T> | omit if None | - | absent field |

---

## Verification Results

### Unit Tests (src/serde/mod.rs)
- `test_serialize_primitives` - String, integer, boolean
- `test_serialize_array` - Array serialization
- `test_to_value` / `test_from_value` - SprotoValue conversion
- `test_optional_some` / `test_optional_none` - Optional handling
- `test_integer_types` - Various integer widths

### Integration Tests (tests/derive_tests.rs)
- `test_derive_encode_decode_primitives` - Basic struct round-trip
- `test_derive_encode_decode_arrays` - Vec fields
- `test_derive_optional_some` / `test_derive_optional_none` - Optional fields
- `test_derive_non_contiguous_tags` - Tags 0, 5, 10 with gaps

### Round-trip Tests (tests/roundtrip_tests.rs)
- Serde round-trip: `to_bytes` → `from_bytes` equality
- Derive round-trip: `sproto_encode` → `sproto_decode` equality
- Cross-validation with SprotoValue API

**All tests passing**: 146 total tests
