# sproto-rust

A pure-Rust implementation of the [sproto](https://github.com/cloudwu/sproto) binary serialization protocol.

Sproto is a compact, schema-driven serialization format designed for simplicity and efficiency, similar to Protocol Buffers but with a smaller feature set.

## Features

- **Schema text parser** - Parse `.sproto` schema definitions at runtime
- **Binary schema loader** - Load pre-compiled binary schemas from C/Lua toolchain
- **Pack/Unpack** - Zero-packing compression for wire efficiency
- **RPC** - Request/response dispatch with session tracking
- **Serde integration** - Standard `#[derive(Serialize, Deserialize)]` support with direct struct-to-wire encoding
- **Derive macros** - `#[derive(SprotoEncode, SprotoDecode)]` for compile-time code generation

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
sproto = "0.1"
```

By default, both `serde` and `derive` features are enabled. To use only the core functionality:

```toml
[dependencies]
sproto = { version = "0.1", default-features = false }
```

## Quick Start

### Using Serde Integration

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Person {
    name: String,
    age: i64,
}

let sproto = sproto::parser::parse(r#"
    .Person { name 0 : string  age 1 : integer }
"#).unwrap();
let person_type = sproto.get_type("Person").unwrap();

let person = Person { name: "Alice".into(), age: 30 };

// Serialize to sproto binary
let bytes = sproto::serde::to_bytes(&sproto, person_type, &person).unwrap();

// Deserialize from sproto binary
let decoded: Person = sproto::serde::from_bytes(&sproto, person_type, &bytes).unwrap();
assert_eq!(person, decoded);
```

### Using Derive Macros

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

let person = Person {
    name: "Alice".into(),
    age: 30,
    active: true,
};

// Encode without external schema
let bytes = person.sproto_encode().unwrap();

// Decode without external schema
let decoded = Person::sproto_decode(&bytes).unwrap();
assert_eq!(person, decoded);
```

### Using Pack/Unpack Compression

```rust
use sproto::pack;

// Compress encoded data for transmission
let packed = pack::pack(&encoded);

// Decompress after receiving
let unpacked = pack::unpack(&packed).unwrap();
```

### Using RPC

```rust
use sproto::rpc::{Host, DispatchResult};

// Load RPC schema
let sproto = sproto::binary_schema::load_binary(&schema_bytes).unwrap();

// Create RPC host
let mut host = Host::new(sproto.clone());

// Dispatch incoming packet
match host.dispatch(&packed_data).unwrap() {
    DispatchResult::Request { name, body, responder, ud } => {
        // Decode body with serde, handle request
        if let Some(resp) = responder {
            let response = resp.respond(&response_body, None).unwrap();
        }
    }
    DispatchResult::Response { session, body, ud } => {
        // Decode body with serde, handle response
    }
}
```

## Schema Syntax

```sproto
# Type definition
.Person {
    name 0 : string
    age 1 : integer
    email 2 : string
    active 3 : boolean
    score 4 : double
    data 5 : binary
    
    # Nested type
    .Address {
        city 0 : string
        zip 1 : integer
    }
    
    address 6 : Address
    friends 7 : *Person        # Array of Person
    tags 8 : *string           # Array of strings
}

# Protocol definition (RPC)
login 1 {
    request LoginRequest
    response LoginResponse
}

ping 2 {
    response PingResponse      # No request body
}

logout 3 {
    response nil               # Confirm, no response data
}

notify 4 {
    # One-way, no response
}
```

## Type Mappings

| Sproto Type | Rust Type |
|-------------|-----------|
| `integer` | `i64` |
| `boolean` | `bool` |
| `string` | `String` |
| `binary` | `Vec<u8>` |
| `double` | `f64` |
| `*type` | `Vec<T>` |
| `.Type` | struct |

## Features

| Feature | Description | Default |
|---------|-------------|---------|
| `serde` | Enable serde integration | Yes (via derive) |
| `derive` | Enable derive macros | Yes |

```toml
# Full functionality (default)
sproto = "0.1"

# Core only (no serde, no derive)
sproto = { version = "0.1", default-features = false }

# Serde only (no derive macros)
sproto = { version = "0.1", default-features = false, features = ["serde"] }
```

## Wire Protocol Compatibility

This implementation is wire-compatible with the [reference C/Lua implementation](https://github.com/cloudwu/sproto). Binary data encoded by either implementation can be decoded by the other.

## Documentation

- [Design](docs/design.md) - Architecture and implementation details
- [Usage Guide](docs/usage.md) - Detailed usage guide
- [Development](docs/development.md) - Development guide

## License

MIT

## Acknowledgments

- [cloudwu/sproto](https://github.com/cloudwu/sproto) - Original C/Lua implementation
