# AGENTS.md

This file provides guidance for AI coding assistants working on sproto-rust.

## Project Overview

sproto-rust is a pure-Rust implementation of the [sproto](https://github.com/cloudwu/sproto) binary serialization protocol. It is wire-compatible with the reference C/Lua implementation.

## Repository Structure

```
src/                    Main library source
  lib.rs                Crate root, re-exports
  types.rs              Core types: Sproto, SprotoType, Field, Protocol (+ Builder API)
  error.rs              Error types (thiserror)
  derive_traits.rs      SprotoEncode/SprotoDecode traits + to_bytes/from_bytes helpers
  codec/                Low-level wire format encode/decode
    wire.rs             Little-endian read/write primitives
    encoder.rs          StructEncoder: tag-based struct encoding engine
    decoder.rs          StructDecoder: tag-based struct decoding engine
  pack.rs               Zero-packing compression
  binary_schema.rs      Binary schema loader (C toolchain compat)
  rpc/                  RPC request/response dispatch with session tracking
sproto-derive/          Proc-macro crate for #[derive(SprotoEncode, SprotoDecode)]
  src/
    lib.rs              Macro entry points
    parse.rs            #[sproto(tag = N, decimal = M)] attribute parsing
    classify.rs         Rust type → wire kind classification
    encode_gen.rs       SprotoEncode impl code generation
    decode_gen.rs       SprotoDecode impl code generation
sproto-lua/             Lua FFI binding (cdylib, independent crate)
tests/                  Integration tests
  direct_tests.rs       StructEncoder/StructDecoder encode/decode (with C binary comparison)
  derive_tests.rs       Derive API roundtrip and fixture tests
  pack_tests.rs         Pack/unpack with C-generated fixtures
  binary_schema_tests.rs  Load C-generated binary schemas
  rpc_tests.rs          RPC dispatch and session management
  testdata/             C/Lua-generated binary fixture files (.bin)
    generate.lua        Lua script to regenerate fixtures
    build.sh            Build script (requires Lua 5.3+ and C sproto)
benches/                Performance benchmarks
  sproto_bench.rs       Criterion micro-benchmarks (cargo bench)
  benchmark.rs          Cross-language comparison (cargo example, --api=direct|derive)
  benchall.sh           Automated Rust (direct+derive) vs Go benchmark runner
docs/                   Project documentation
```

## Build & Test Commands

```bash
# Build
cargo build                      # dev build
cargo build --release            # release build

# Test (run all workspace tests)
cargo test --workspace

# Lint & format
cargo clippy --workspace -- -D warnings
cargo fmt -- --check

# Full CI check (format + lint + test)
make ci

# Benchmarks
cargo bench --bench sproto_bench            # criterion benchmarks
bash benches/benchall.sh [COUNT]            # Rust vs Go comparison
```

## API

The library provides two levels of API:

### Derive API (feature: `derive`, default on)

- `#[derive(SprotoEncode, SprotoDecode)]` — proc-macro for struct serialization
- `sproto::to_bytes(&schema, "TypeName", &value)` — encode struct to bytes
- `sproto::from_bytes::<T>(&schema, "TypeName", &data)` — decode bytes to struct
- Schema is passed at runtime (not embedded at compile time)
- `#[sproto(tag = N)]` attribute maps struct fields to sproto tags
- `#[sproto(tag = N, decimal = M)]` for fixed-point decimals

### Direct API (always available)

- `StructEncoder` / `StructDecoder` for tag-based field-by-field encoding/decoding
- Builder API: `Sproto::new()`, `Sproto::add_type()`, `Sproto::add_protocol()` for programmatic schema construction
- `Field::new()` / `Field::array()` / `Field::decimal()` for field definitions
- Binary schema loading via `binary_schema::load_binary()`
- `pack::pack()` / `pack::unpack()` for zero-packing compression
- RPC module: `Host`, `RequestSender`, `Responder` for RPC dispatch

### Feature Flags

- `derive` (default): Enables `sproto-derive` proc-macro crate, `SprotoEncode`/`SprotoDecode` traits, and `to_bytes`/`from_bytes` functions
- No features (`default-features = false`): Only Direct API, no proc-macro dependencies

## Coding Conventions

### General

- Rust edition 2021, stable toolchain
- No `unsafe` code in the main crate
- Error types use `thiserror` derive
- Avoid unnecessary allocations; prefer `extend_from_slice` over intermediate `Vec` temp buffers

### Sproto Wire Format Key Points

- Header: `field_count: u16 LE` + field descriptors (`u16 LE` each)
- Field descriptor: even = value (`value/2 - 1` is inline or -1 for data section), odd = skip (`tag += value/2`)
- Data section: length-prefixed (`u32 LE`) values appended sequentially
- Struct arrays: outer length + sequence of (length-prefixed encoded struct bytes)
- Integer: inline if `0 <= value < 0x7fff`, otherwise 4 or 8 bytes in data section
- Boolean: always inline (0 or 1)
- Double: always 8 bytes in data section

### Test Organization

- `tests/direct_tests.rs` — StructEncoder/StructDecoder encode/decode, C binary comparison
- `tests/derive_tests.rs` — Derive API roundtrip, fixture decode, encode-decode consistency
- `tests/pack_tests.rs` — Pack/unpack with C-generated fixtures
- `tests/binary_schema_tests.rs` — Load C-generated binary schemas
- `tests/rpc_tests.rs` — RPC dispatch and session management

Binary fixtures in `tests/testdata/` are generated by `generate.lua` using the C/Lua reference implementation. Regenerate with `cd tests/testdata && bash build.sh`.

## Architecture Decisions

### Encode Optimization

`StructEncoder` uses a single shared output buffer pattern. Small structs (<= 32 fields) use a stack-allocated `[Option<FieldEntry>; 32]` array to avoid heap allocation. Data section write order is tracked for in-place compaction.

Nested struct encoding writes a length placeholder, encodes the child struct into the same buffer, then backfills the length. This eliminates all intermediate allocations.

### Field Lookup Optimization

`SprotoType` stores fields sorted by tag and computes `base_tag` and `maxn`:
- Contiguous tags: O(1) direct index `fields[tag - base_tag]`
- Non-contiguous: binary search fallback
- Name lookup: linear scan for <= 8 fields, `HashMap` for larger types

## Common Tasks

### Adding a New Field Type

1. Add variant to `FieldType` enum in `src/types.rs`
2. Add `set_*` encoding method in `src/codec/encoder.rs`
3. Add `as_*` decoding method in `src/codec/decoder.rs`
4. Add tests in `tests/direct_tests.rs`

### Modifying the Wire Format

1. Update `src/codec/wire.rs`
2. Update encoder.rs and decoder.rs
3. Regenerate test fixtures: `cd tests/testdata && bash build.sh`
4. Run full test suite: `cargo test --workspace`

### Adding RPC Features

1. Modify `src/rpc/mod.rs`
2. Add tests in `tests/rpc_tests.rs`
3. Update `tests/testdata/` RPC fixtures if header format changes

## Post-Change Checklist

After every code modification, you MUST:

1. **Run tests**: `cargo test --workspace` — all tests must pass
2. **Run clippy**: `cargo clippy --workspace -- -D warnings` — no warnings allowed
3. **Update documentation**: If the change affects public API, features, project structure, or behavior, update the relevant documentation files:
   - `README.md` — user-facing feature list and examples
   - `AGENTS.md` — repository structure, API descriptions, test organization
   - `docs/usage.md` — usage examples and type mappings
   - `docs/design.md` — architecture and design decisions
   - `docs/development.md` — project structure, workspace members, test counts, benchmark commands
4. **Verify no-default-features**: If feature-gated code is changed, ensure `cargo build --no-default-features` still compiles
