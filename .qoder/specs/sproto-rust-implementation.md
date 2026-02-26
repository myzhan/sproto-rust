# Sproto-Rust Implementation Plan

**Status: COMPLETED**

## Goal
Implement a complete Rust port of the sproto binary serialization library, including:
- Schema text parser (like sprotoparser.lua) ✓
- Dynamic Value-based API for encoding/decoding ✓
- Zero-compression packing/unpacking ✓
- RPC host/attach/dispatch pattern ✓
- Serde integration ✓
- Derive macros ✓
- Comprehensive unit tests ✓
- Design document (DESIGN.md) ✓

Reference: `/Users/zhanqp/github/sproto` (C + Lua implementation)

---

## Project Structure (Final)

```
sproto-rust/
  Cargo.toml
  DESIGN.md
  src/
    lib.rs                  # Public API re-exports
    error.rs                # Error types (thiserror)
    value.rs                # SprotoValue enum + From/TryFrom impls
    types.rs                # Field, SprotoType, Protocol, Sproto
    derive_traits.rs        # SprotoEncode, SprotoDecode traits
    parser/
      mod.rs                # parse() entry point
      lexer.rs              # Tokenizer with line/col tracking
      ast.rs                # AST node types
      grammar.rs            # Recursive descent parser
      schema_builder.rs     # AST -> Sproto conversion, validation
    codec/
      mod.rs                # encode/decode entry points
      wire.rs               # Little-endian read/write helpers
      encoder.rs            # sproto_encode equivalent
      decoder.rs            # sproto_decode equivalent
    pack.rs                 # pack/unpack (0-compression)
    binary_schema.rs        # Load pre-compiled binary schemas
    rpc/
      mod.rs                # Host, RequestSender, DispatchResult
    serde/                  # (feature = "serde")
      mod.rs                # to_bytes, from_bytes, to_value, from_value
      ser.rs                # SprotoSerializer
      de.rs                 # SprotoDeserializer
      error.rs              # SerdeError
  sproto-derive/            # (feature = "derive")
    Cargo.toml
    src/
      lib.rs                # Proc-macro entry points
      attr.rs               # Attribute parsing
      encode.rs             # SprotoEncode derive
      decode.rs             # SprotoDecode derive
  testdata/
    generate.lua            # Lua script that uses C/Lua sproto to produce fixtures
    *.bin                   # Generated binary fixtures (committed to repo)
  tests/
    binary_schema_tests.rs  # Load C-generated .bin schemas
    decode_tests.rs         # Decode C-generated encoded .bin
    encode_tests.rs         # Encode in Rust, compare with C .bin
    pack_tests.rs           # Pack/unpack cross-validation
    derive_tests.rs         # Derive macro tests
    roundtrip_tests.rs      # Self-consistency round-trip tests
    rpc_tests.rs            # RPC functionality tests
```

---

## Core Types (Implemented)

### SprotoValue (`src/value.rs`)
```rust
pub enum SprotoValue {
    Integer(i64),
    Boolean(bool),
    Str(String),
    Binary(Vec<u8>),
    Double(f64),
    Struct(HashMap<String, SprotoValue>),
    Array(Vec<SprotoValue>),
}
```
- `From<T>` for common Rust types (i64, bool, &str, f64, Vec<T>, etc.)
- `TryFrom<SprotoValue>` for type-safe extraction
- `PartialEq`, `Eq`, `Debug`, `Display`
- Builder: `SprotoValue::from_fields(vec![("name", value), ...])`

### Schema Types (`src/types.rs`)
```rust
pub struct Field {
    pub name: String,
    pub tag: u16,
    pub field_type: FieldType,
    pub is_array: bool,
    pub key_tag: i32,               // For map: tag of key field (-1 if not map)
    pub is_map: bool,               // For *Type() two-field maps
    pub decimal_precision: u32,     // For integer(N), 0 if not decimal
}

pub enum FieldType {
    Integer, Boolean, String, Binary, Double,
    Struct(usize),  // index into Sproto.types_list
}

pub struct SprotoType {
    pub name: String,
    pub fields: Vec<Field>,         // sorted by tag
    pub base_tag: i32,              // -1 if non-contiguous
    pub maxn: usize,                // max field slots including skips
}

pub struct Protocol {
    pub name: String,
    pub tag: u16,
    pub request: Option<usize>,     // index into types_list
    pub response: Option<usize>,    // index into types_list
    pub confirm: bool,              // response nil
}

pub struct Sproto {
    pub types_list: Vec<SprotoType>,
    pub types_by_name: HashMap<String, usize>,
    pub protocols: Vec<Protocol>,
    pub protocols_by_name: HashMap<String, usize>,
    pub protocols_by_tag: HashMap<u16, usize>,
}
```

---

## Implementation Tasks (All Completed)

| Task | Status | Description |
|------|--------|-------------|
| 1. Project scaffold | ✓ | Cargo.toml, module structure |
| 2. Error types | ✓ | SprotoError, ParseError, EncodeError, DecodeError, PackError, RpcError |
| 3. SprotoValue | ✓ | Enum with From/TryFrom traits |
| 4. Schema types | ✓ | Field, FieldType, SprotoType, Protocol, Sproto |
| 5. Wire utilities | ✓ | LE read/write, constants |
| 6. Encoder | ✓ | encode(sproto, type, value) -> bytes |
| 7. Decoder | ✓ | decode(sproto, type, bytes) -> SprotoValue |
| 8. Schema parser | ✓ | Lexer, AST, grammar, schema builder |
| 9. Pack/Unpack | ✓ | Zero-compression algorithm |
| 10. Binary schema loader | ✓ | Bootstrap decoder for C-generated schemas |
| 11. RPC module | ✓ | Host, RequestSender, Responder, DispatchResult |
| 12. Cross-validation tests | ✓ | C-generated fixtures comparison |
| 13. DESIGN.md | ✓ | Architecture documentation |
| 14. Serde integration | ✓ | Schema-driven serialization |
| 15. Derive macros | ✓ | SprotoEncode, SprotoDecode |
| 16. Round-trip tests | ✓ | Self-consistency verification |
| 17. RPC tests | ✓ | RPC functionality verification |

---

## Test Fixtures (testdata/)

| File | Content |
|------|---------|
| `person_data_schema.bin` | Binary schema for Person + Data types |
| `example1_encoded.bin` | Person { name="Alice", age=13, marital=false } |
| `example2_encoded.bin` | Person { name="Bob", age=40, children=[...] } |
| `example3_encoded.bin` | Data { numbers=[1,2,3,4,5] } |
| `example4_encoded.bin` | Data { numbers=[(1<<32)+1, (1<<32)+2, (1<<32)+3] } |
| `example5_encoded.bin` | Data { bools=[false, true, false] } |
| `example6_encoded.bin` | Data { number=100000, bignumber=-10000000000 } |
| `example7_encoded.bin` | Data { double=0.01171875, doubles=[...] } |
| `example8_encoded.bin` | Data { fpn=1.82 } |
| `example1_packed.bin` .. `example8_packed.bin` | Packed versions |
| `addressbook_schema.bin` | Schema with maps and indexed arrays |
| `addressbook_encoded.bin` | Encoded AddressBook with nested data |
| `addressbook_packed.bin` | Packed AddressBook |
| `rpc_schema.bin` | RPC schema (package, protocols) |
| `rpc_foobar_request.bin` | Packed RPC request for foobar protocol |
| `rpc_foobar_response.bin` | Packed RPC response for foobar protocol |
| `rpc_foo_request.bin` | Request with no body (foo protocol) |
| `rpc_bar_request.bin` | Request for bar protocol |
| `rpc_bar_response.bin` | Response nil (bar protocol) |

---

## Test Summary

| Test File | Count | Description |
|-----------|-------|-------------|
| Unit tests (src/) | 36 | Component-level tests in modules |
| binary_schema_tests.rs | 3 | Load C-generated schemas |
| decode_tests.rs | 8 | Decode C-generated binaries |
| encode_tests.rs | 9 | Encode and compare with C output |
| pack_tests.rs | 17 | Pack/unpack compression |
| derive_tests.rs | 5 | Derive macro functionality |
| roundtrip_tests.rs | 48 | Self-consistency tests |
| rpc_tests.rs | 19 | RPC functionality |
| Doc tests | 1 | Documentation examples |
| **Total** | **146** | All passing |

### Test Categories

**Cross-validation tests** - Verify byte-level compatibility with C reference:
- Decode C-generated binaries, verify field values
- Encode Rust values, assert bytes identical to C output
- Pack/unpack against C-generated compressed data
- Load C-generated binary schemas

**Round-trip tests** - Self-consistency without external dependencies:
- Encode → decode → verify equality
- Pack → unpack → verify equality
- Full pipeline: encode → pack → unpack → decode
- Serde: to_bytes → from_bytes
- Derive: sproto_encode → sproto_decode

**RPC tests** - Protocol functionality:
- Host creation and dispatch
- Request/response round-trip
- Session tracking
- Protocol configurations (with/without request/response)
- Error handling

---

## Dependencies (Final)

```toml
[package]
name = "sproto"
version = "0.1.0"
edition = "2021"

[features]
default = ["derive"]
serde = ["dep:serde"]
derive = ["dep:sproto-derive", "serde"]

[dependencies]
thiserror = "2"
serde = { version = "1", optional = true }
sproto-derive = { path = "sproto-derive", optional = true }

[dev-dependencies]
pretty_assertions = "1"
serde = { version = "1", features = ["derive"] }

[workspace]
members = ["sproto-derive"]
```

**sproto-derive/Cargo.toml**:
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

---

## Verification Results

### All Tests Passing
```
cargo test
   running 146 tests
   test result: ok. 146 passed; 0 failed
```

### Clippy Clean
```
cargo clippy
   Finished dev [unoptimized + debuginfo] target(s)
```

### Features Working
- `cargo test` - Uses default features (derive + serde)
- `cargo test --no-default-features` - Core only
- `cargo test --features serde` - Serde without derive
- `cargo test --features derive` - Full functionality

---

## Public API

```rust
// Core types
pub use sproto::{Sproto, SprotoValue, SprotoError};

// Parsing
pub use sproto::parser::parse;
pub use sproto::binary_schema::load_binary;

// Encoding/Decoding
pub use sproto::codec::{encode, decode};
pub use sproto::pack::{pack, unpack};

// RPC
pub use sproto::rpc::{Host, DispatchResult, Responder, RequestSender};

// Serde (feature = "serde")
pub use sproto::serde::{to_bytes, from_bytes, to_value, from_value};

// Derive (feature = "derive")
pub use sproto::{SprotoEncode, SprotoDecode};
```
