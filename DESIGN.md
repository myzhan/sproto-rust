# sproto-rust Design Document

## Overview

A pure-Rust implementation of the [sproto](https://github.com/cloudwu/sproto) binary serialization protocol. Sproto is a compact, schema-driven serialization format designed for simplicity and efficiency, similar to Protocol Buffers but with a smaller feature set.

This library provides:

- **Schema text parser** -- parse `.sproto` schema definitions into runtime type metadata
- **Binary schema loader** -- load pre-compiled binary schemas (from C/Lua toolchain)
- **Encode/Decode** -- serialize and deserialize structured data using a dynamic `SprotoValue` API
- **Pack/Unpack** -- zero-packing compression (Cap'n Proto-style) for wire efficiency
- **RPC** -- request/response dispatch with session tracking
- **Serde integration** -- schema-driven serialization using standard `#[derive(Serialize, Deserialize)]`
- **Derive macros** -- `#[derive(SprotoEncode, SprotoDecode)]` for compile-time code generation

## Module Architecture

```
src/
  lib.rs                  -- Public re-exports
  error.rs                -- Error types (ParseError, EncodeError, DecodeError, PackError, RpcError)
  value.rs                -- SprotoValue: dynamic value type (Integer, Boolean, Str, Binary, Double, Struct, Array)
  types.rs                -- Schema metadata: Sproto, SprotoType, Field, Protocol, FieldType
  derive_traits.rs        -- SprotoEncode and SprotoDecode trait definitions
  codec/
    mod.rs                -- Re-exports encode() and decode()
    wire.rs               -- Little-endian read/write primitives, constants
    encoder.rs            -- encode(sproto, type, value) -> bytes
    decoder.rs            -- decode(sproto, type, bytes) -> SprotoValue
  pack.rs                 -- pack() and unpack() zero-compression
  parser/
    mod.rs                -- parse(text) -> Sproto
    lexer.rs              -- Tokenizer
    ast.rs                -- AST node types
    grammar.rs            -- Recursive descent parser
    schema_builder.rs     -- AST -> Sproto (type resolution, validation)
  binary_schema.rs        -- load_binary(bytes) -> Sproto
  rpc/
    mod.rs                -- Host, RequestSender, Responder, DispatchResult
  serde/                  -- (feature = "serde")
    mod.rs                -- to_bytes(), from_bytes(), to_value(), from_value()
    ser.rs                -- SprotoSerializer implementing serde::Serializer
    de.rs                 -- SprotoDeserializer implementing serde::Deserializer
    error.rs              -- SerdeError type

sproto-derive/            -- Proc-macro crate (feature = "derive")
  src/
    lib.rs                -- Proc-macro entry points
    attr.rs               -- Attribute parsing (#[sproto(tag = N)])
    encode.rs             -- SprotoEncode derive implementation
    decode.rs             -- SprotoDecode derive implementation
```

## Wire Protocol

Sproto uses a tag-based binary format. Each encoded struct consists of:

1. **Header**: `u16` field count, followed by that many `u16` field descriptors
2. **Data part**: variable-length data for fields that don't fit inline

Field descriptors use a compact encoding:
- **Even value `v`**: inline value = `v / 2 - 1`. Used for small integers (0..0x7ffe) and booleans.
- **Odd value `v`**: skip marker = `(v - 1) / 2` tags. Encodes gaps between non-contiguous tags.
- **Zero**: the field's data is in the data part (prefixed with a `u32` length).

All integers are little-endian. Integer fields auto-select 32-bit or 64-bit encoding based on value range. Arrays include a size-marker byte (4 or 8) before integer elements.

## Zero-Packing Compression

The pack algorithm processes 8-byte chunks:

- For each chunk, compute a **tag byte** where bit `i` indicates byte `i` is non-zero.
- Output: tag byte followed by only the non-zero bytes.
- **0xFF optimization**: When a chunk has all 8 bytes non-zero, start a raw batch. Consecutive all-nonzero chunks (and chunks with 6-7 nonzero bytes if already in a batch) are written as: `0xFF | count-1 | raw_bytes...`. Maximum batch: 256 chunks.

This achieves good compression for sproto's typical output which contains many zero bytes in headers and length prefixes.

## Schema Parser

The parser is a hand-written recursive descent parser (no external parser combinator dependencies) with three phases:

1. **Lexer** (`lexer.rs`): Tokenizes schema text into `Token` variants (Name, Number, punctuation). Tracks line numbers for error reporting. Skips `#` comments.

2. **Grammar** (`grammar.rs`): Parses token stream into an AST. Handles type definitions (`.TypeName { fields... }`), protocol definitions (`ProtocolName tag { request/response }`), nested types, array fields (`*type`), and map/decimal extras.

3. **Schema Builder** (`schema_builder.rs`): Three-pass resolution:
   - Collect all types and protocols, flattening nested types with dot-separated names (e.g., `Person.PhoneNumber`)
   - Sort type names alphabetically (matching Lua reference for binary compatibility)
   - Resolve type references, validate uniqueness of tags/names, compute optimized lookup structures

## Field Lookup Optimization

`SprotoType` stores fields sorted by tag and computes `base_tag` and `maxn`:

- If tags are contiguous starting from `base_tag`, field lookup is O(1) via direct indexing: `fields[tag - base_tag]`
- Otherwise, falls back to binary search on the sorted field array

This matches the C reference implementation's optimization strategy.

## Binary Schema Loading

`binary_schema.rs` implements a bootstrap decoder that loads pre-compiled binary schemas without requiring a schema-for-the-schema. It hardcodes knowledge of the meta-schema structure (`.type`, `.field`, `.protocol` definitions) and decodes the binary group message into `Sproto` metadata.

This enables interoperability with the C/Lua toolchain: schemas compiled by `sprotodump` can be loaded directly.

## RPC Layer

The RPC module implements the sproto RPC pattern:

- **`Host`**: Manages an RPC endpoint with a local schema. `dispatch(packed_data)` decodes incoming packets and returns either a `Request` (with a `Responder` for sending back replies) or a `Response` (matching a previous outbound request by session ID).

- **`RequestSender`**: Created via `Host::attach(remote_schema)`. Encodes outbound requests with protocol tag and tracks pending sessions for response matching.

- **`Responder`**: Encodes a response for a specific request, preserving the session ID.

Package headers follow the standard sproto `package` type with `type`, `session`, and optional `ud` fields.

## Dynamic Value API

`SprotoValue` is an enum similar to `serde_json::Value`:

```rust
enum SprotoValue {
    Integer(i64),
    Boolean(bool),
    Str(String),
    Binary(Vec<u8>),
    Double(f64),
    Struct(HashMap<String, SprotoValue>),
    Array(Vec<SprotoValue>),
}
```

This provides a schema-agnostic way to construct and inspect sproto messages without code generation. The `From` trait implementations allow ergonomic construction:

```rust
let person = SprotoValue::from_fields(vec![
    ("name", "Alice".into()),
    ("age", 30i64.into()),
]);
```

`TryFrom` implementations enable type-safe extraction from `SprotoValue`:

```rust
let name: String = String::try_from(value)?;
let numbers: Vec<i64> = Vec::<i64>::try_from(array_value)?;
```

## Serde Integration

The `serde` feature provides schema-driven serialization using standard serde traits:

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: i64,
}

// Serialize to sproto binary
let bytes = sproto::serde::to_bytes(&sproto, person_type, &person)?;

// Deserialize from sproto binary
let decoded: Person = sproto::serde::from_bytes(&sproto, person_type, &bytes)?;
```

The implementation bridges through `SprotoValue` as an intermediate representation:
1. **Serialization**: Rust value → `SprotoValue` (via `SprotoSerializer`) → binary (via `codec::encode`)
2. **Deserialization**: binary → `SprotoValue` (via `codec::decode`) → Rust value (via `SprotoDeserializer`)

Key design decisions:
- Uses higher-ranked trait bounds (`for<'de> Deserialize<'de>`) to handle owned data
- Optional fields map to `Option<T>` in Rust structs
- Field names must match between Rust struct and sproto schema

## Derive Macros

The `derive` feature provides compile-time code generation:

```rust
use sproto::{SprotoEncode, SprotoDecode};

#[derive(SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    active: bool,
}

// Encode without external schema
let bytes = person.sproto_encode()?;

// Decode without external schema
let decoded = Person::sproto_decode(&bytes)?;
```

Key features:
- **Inline schema generation**: The macro generates schema metadata at compile time, eliminating runtime schema lookup
- **Compile-time type detection**: Rust types are mapped to sproto `FieldType` at macro expansion time
- **Support for `Option<T>`**: Optional fields are automatically handled
- **Support for `Vec<T>`**: Arrays of primitive types are supported
- **Non-contiguous tags**: Gap markers are calculated at compile time for efficient encoding
- **Attribute syntax**: `#[sproto(tag = N)]` specifies the wire tag, `#[sproto(skip)]` excludes fields

## Testing Strategy

Tests are structured in multiple layers:

### 1. Unit Tests (in-module `#[cfg(test)]`)
Test individual components -- lexer tokenization, parser grammar, wire format read/write, pack/unpack operations.

### 2. Cross-validation Integration Tests (`tests/`)
Verify byte-level compatibility with the C reference implementation:
- `testdata/generate.lua` uses the C/Lua sproto library to produce binary fixture files (`.bin`)
- `tests/decode_tests.rs` decodes C-generated binaries and verifies field values
- `tests/encode_tests.rs` encodes Rust values and asserts byte-identical output to C
- `tests/pack_tests.rs` validates pack/unpack against C-generated compressed data
- `tests/binary_schema_tests.rs` loads C-generated binary schemas and verifies structure

### 3. Round-trip Tests (`tests/roundtrip_tests.rs`)
Self-consistency tests without external dependencies:
- **Encode/Decode round-trip**: Verify `decode(encode(value)) == value` for all types
- **Pack/Unpack round-trip**: Verify `unpack(pack(data)) == data`
- **Full pipeline**: `encode → pack → unpack → decode`
- **Serde round-trip**: Verify serde serialization/deserialization
- **Derive round-trip**: Verify derive macro encode/decode
- Coverage includes: primitives, arrays, nested structs, optional fields, boundary values, Unicode strings

### 4. RPC Tests (`tests/rpc_tests.rs`)
RPC functionality tests:
- Host creation and protocol dispatch
- Request/response round-trip
- Session tracking and management
- Protocol configurations (with/without request/response types)
- Error handling (unknown protocol, unknown session)
- Edge cases (large session IDs, Unicode data)

### 5. Derive Macro Tests (`tests/derive_tests.rs`)
Derive macro specific tests:
- Basic struct encoding/decoding
- Optional field handling
- Array field handling
- Non-contiguous tag support

### Test Summary

| Test Category | Count | Description |
|---------------|-------|-------------|
| Unit tests | 36 | Component-level tests |
| Binary schema | 3 | Schema loading from C-generated files |
| Decode | 8 | Decode C-generated binaries |
| Encode | 9 | Encode and compare with C output |
| Pack | 17 | Pack/unpack compression |
| Round-trip | 48 | Self-consistency tests |
| RPC | 19 | RPC functionality |
| Derive | 5 | Derive macro tests |
| **Total** | **146** | All tests passing |

## Features

```toml
[features]
default = ["derive"]
serde = ["dep:serde"]
derive = ["dep:sproto-derive", "serde"]
```

- **`serde`**: Enables schema-driven serde integration (`sproto::serde` module)
- **`derive`**: Enables derive macros (`#[derive(SprotoEncode, SprotoDecode)]`), includes `serde`
- Default: `derive` is enabled, providing full functionality out of the box

## Dependencies

### Required
- `thiserror` -- Ergonomic error type derivation

### Optional
- `serde` (feature = "serde") -- Serde framework for serialization
- `sproto-derive` (feature = "derive") -- Proc-macro crate for derive macros

### Dev Dependencies
- `pretty_assertions` -- Readable test failure diffs
- `serde` (with "derive" feature) -- For serde integration tests

### sproto-derive Dependencies
- `syn` -- Rust syntax parsing
- `quote` -- Quasi-quoting for code generation
- `proc-macro2` -- Proc-macro utilities
