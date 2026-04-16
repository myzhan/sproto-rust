//! Benchmark for sproto encode/decode/pack/unpack operations.
//!
//! Supports both Serde API and Derive API benchmarks.
//!
//! Usage:
//!   cargo build --release --example benchmark
//!   ./target/release/examples/benchmark [--count N] [--mode MODE] [--api API]
//!
//! Modes: encode, decode, encode_pack, unpack_decode
//! APIs:  serde      -- AddressBook + Serde (Go comparison, default)
//!        derive     -- AddressBook + Derive (nested struct benchmark)
//!        compare    -- AddressBook + both APIs (side-by-side comparison)

use std::hint::black_box;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use sproto::pack;
use sproto::types::SprotoType;
use sproto::Sproto;
use sproto::{SprotoDecode, SprotoEncode};

// ============================================================================
// Schema Definitions
// ============================================================================

/// AddressBook schema (matching Go benchmark)
const ADDRESSBOOK_SCHEMA: &str = r#"
.PhoneNumber {
    number 0 : string
    type 1 : integer
    real 2 : double
}
.Person {
    name 0 : string
    id 1 : integer
    email 2 : string
    phone 3 : *PhoneNumber
}
.Human {
    name 0 : string
    age 1 : integer
    marital 2 : boolean
    children 3 : *Human
}
.AddressBook {
    person 0 : *Person
    human 1 : *Human
}
"#;

// ============================================================================
// AddressBook Data Structures (both Serde and Derive)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, SprotoEncode, SprotoDecode)]
struct PhoneNumber {
    #[sproto(tag = 0)]
    number: String,
    #[serde(rename = "type")]
    #[sproto(tag = 1)]
    phone_type: i64,
    #[sproto(tag = 2)]
    real: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    id: i64,
    #[sproto(tag = 3)]
    phone: Vec<PhoneNumber>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SprotoEncode, SprotoDecode)]
struct Human {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    marital: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[sproto(tag = 3)]
    children: Vec<Human>,
}

#[derive(Debug, Clone, Serialize, Deserialize, SprotoEncode, SprotoDecode)]
struct AddressBook {
    #[sproto(tag = 0)]
    person: Vec<Person>,
    #[sproto(tag = 1)]
    human: Vec<Human>,
}

// ============================================================================
// Test Data Construction
// ============================================================================

fn build_addressbook() -> AddressBook {
    AddressBook {
        person: vec![
            Person {
                name: "Alice".to_string(),
                id: 10000,
                phone: vec![
                    PhoneNumber {
                        number: "123456789".to_string(),
                        phone_type: 1,
                        real: 1.234567,
                    },
                    PhoneNumber {
                        number: "87654321".to_string(),
                        phone_type: 2,
                        real: 5567.12345,
                    },
                ],
            },
            Person {
                name: "Bob".to_string(),
                id: 20000,
                phone: vec![PhoneNumber {
                    number: "01234567890".to_string(),
                    phone_type: 3,
                    real: 567.1378,
                }],
            },
        ],
        human: vec![
            Human {
                name: "kkkk".to_string(),
                age: 11,
                marital: true,
                children: vec![],
            },
            Human {
                name: "dddd".to_string(),
                age: 22,
                marital: false,
                children: vec![
                    Human {
                        name: "cccc".to_string(),
                        age: 33,
                        marital: false,
                        children: vec![],
                    },
                    Human {
                        name: "ffff".to_string(),
                        age: 44,
                        marital: false,
                        children: vec![],
                    },
                ],
            },
        ],
    }
}

// ============================================================================
// Benchmark Functions - Serde API (AddressBook)
// ============================================================================

fn bench_serde_ab_encode(schema: &Sproto, st: &SprotoType, ab: &AddressBook, count: usize) {
    for _ in 0..count {
        let _ = black_box(sproto::serde::to_bytes(schema, st, ab).unwrap());
    }
}

fn bench_serde_ab_decode(schema: &Sproto, st: &SprotoType, data: &[u8], count: usize) {
    for _ in 0..count {
        let _: AddressBook = black_box(sproto::serde::from_bytes(schema, st, data).unwrap());
    }
}

fn bench_serde_ab_encode_pack(schema: &Sproto, st: &SprotoType, ab: &AddressBook, count: usize) {
    for _ in 0..count {
        let encoded = sproto::serde::to_bytes(schema, st, ab).unwrap();
        let _ = black_box(pack::pack(&encoded));
    }
}

fn bench_serde_ab_unpack_decode(schema: &Sproto, st: &SprotoType, packed: &[u8], count: usize) {
    for _ in 0..count {
        let unpacked = pack::unpack(packed).unwrap();
        let _: AddressBook = black_box(sproto::serde::from_bytes(schema, st, &unpacked).unwrap());
    }
}

// ============================================================================
// Benchmark Functions - Derive API (AddressBook)
// ============================================================================

fn bench_derive_ab_encode(ab: &AddressBook, count: usize) {
    for _ in 0..count {
        let _ = black_box(ab.sproto_encode().unwrap());
    }
}

fn bench_derive_ab_decode(data: &[u8], count: usize) {
    for _ in 0..count {
        let _ = black_box(AddressBook::sproto_decode(data).unwrap());
    }
}

fn bench_derive_ab_encode_pack(ab: &AddressBook, count: usize) {
    for _ in 0..count {
        let encoded = ab.sproto_encode().unwrap();
        let _ = black_box(pack::pack(&encoded));
    }
}

fn bench_derive_ab_unpack_decode(packed: &[u8], count: usize) {
    for _ in 0..count {
        let unpacked = pack::unpack(packed).unwrap();
        let _ = black_box(AddressBook::sproto_decode(&unpacked).unwrap());
    }
}

// ============================================================================
// Benchmark Runner
// ============================================================================

fn run_benchmark(label: &str, mode: &str, count: usize, f: impl FnOnce()) {
    let start = Instant::now();
    f();
    let elapsed = start.elapsed();
    println!(
        "count:\t{}\tcost:\t{:.6}s\tmode:\t{}\tapi:\t{}",
        count,
        elapsed.as_secs_f64(),
        mode,
        label,
    );
}

// ============================================================================
// CLI Argument Parsing
// ============================================================================

fn parse_arg(args: &[String], name: &str, default: &str) -> String {
    for arg in args {
        if let Some(val) = arg.strip_prefix(&format!("{}=", name)) {
            return val.to_string();
        }
    }
    for i in 0..args.len().saturating_sub(1) {
        if args[i] == name {
            return args[i + 1].clone();
        }
    }
    default.to_string()
}

fn print_usage() {
    eprintln!("Usage: benchmark [OPTIONS]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --count N              Iteration count (default: 1000000)");
    eprintln!("  --mode MODE            Benchmark mode (default: encode_pack)");
    eprintln!("                         encode, decode, encode_pack, unpack_decode");
    eprintln!("  --api API              API to benchmark (default: serde)");
    eprintln!("                         serde   -- AddressBook + Serde (Go comparison)");
    eprintln!("                         derive  -- AddressBook + Derive (nested structs)");
    eprintln!("                         compare -- AddressBook + both APIs (side-by-side)");
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_usage();
        return;
    }

    let count: usize = parse_arg(&args, "--count", "1000000")
        .parse()
        .expect("--count must be a positive integer");
    let mode = parse_arg(&args, "--mode", "encode_pack");
    let api = parse_arg(&args, "--api", "serde");

    // Validate arguments
    if !["encode", "decode", "encode_pack", "unpack_decode"].contains(&mode.as_str()) {
        eprintln!(
            "Unknown --mode: {}. Use: encode, decode, encode_pack, unpack_decode",
            mode
        );
        std::process::exit(1);
    }
    if !["serde", "derive", "compare"].contains(&api.as_str()) {
        eprintln!("Unknown --api: {}. Use: serde, derive, compare", api);
        std::process::exit(1);
    }

    match api.as_str() {
        "serde" => run_serde_addressbook(&mode, count),
        "derive" => run_derive_addressbook(&mode, count),
        "compare" => run_compare_addressbook(&mode, count),
        _ => unreachable!(),
    }
}

fn run_serde_addressbook(mode: &str, count: usize) {
    let schema = sproto::parser::parse(ADDRESSBOOK_SCHEMA).unwrap();
    let st = schema.get_type("AddressBook").unwrap();

    let ab = build_addressbook();
    let encoded = sproto::serde::to_bytes(&schema, st, &ab).unwrap();
    let packed = pack::pack(&encoded);

    let decoded: AddressBook = sproto::serde::from_bytes(&schema, st, &encoded).unwrap();
    assert_eq!(
        decoded.person.len(),
        ab.person.len(),
        "serde roundtrip failed"
    );

    eprintln!(
        "AddressBook(serde): encoded {} bytes, packed {} bytes",
        encoded.len(),
        packed.len()
    );

    run_benchmark("serde", mode, count, || match mode {
        "encode" => bench_serde_ab_encode(&schema, st, &ab, count),
        "decode" => bench_serde_ab_decode(&schema, st, &encoded, count),
        "encode_pack" => bench_serde_ab_encode_pack(&schema, st, &ab, count),
        "unpack_decode" => bench_serde_ab_unpack_decode(&schema, st, &packed, count),
        _ => unreachable!(),
    });
}

fn run_derive_addressbook(mode: &str, count: usize) {
    let ab = build_addressbook();
    let encoded = ab.sproto_encode().unwrap();
    let packed = pack::pack(&encoded);

    let decoded = AddressBook::sproto_decode(&encoded).unwrap();
    assert_eq!(
        decoded.person.len(),
        ab.person.len(),
        "derive roundtrip failed"
    );

    eprintln!(
        "AddressBook(derive): encoded {} bytes, packed {} bytes",
        encoded.len(),
        packed.len()
    );

    run_benchmark("derive", mode, count, || match mode {
        "encode" => bench_derive_ab_encode(&ab, count),
        "decode" => bench_derive_ab_decode(&encoded, count),
        "encode_pack" => bench_derive_ab_encode_pack(&ab, count),
        "unpack_decode" => bench_derive_ab_unpack_decode(&packed, count),
        _ => unreachable!(),
    });
}

fn run_compare_addressbook(mode: &str, count: usize) {
    let schema = sproto::parser::parse(ADDRESSBOOK_SCHEMA).unwrap();
    let st = schema.get_type("AddressBook").unwrap();
    let ab = build_addressbook();

    // Serde path
    let serde_encoded = sproto::serde::to_bytes(&schema, st, &ab).unwrap();
    let serde_packed = pack::pack(&serde_encoded);

    // Derive path
    let derive_encoded = ab.sproto_encode().unwrap();
    let derive_packed = pack::pack(&derive_encoded);

    // Verify roundtrip for each API independently
    let serde_decoded: AddressBook =
        sproto::serde::from_bytes(&schema, st, &serde_encoded).unwrap();
    assert_eq!(
        serde_decoded.person.len(),
        ab.person.len(),
        "serde roundtrip failed"
    );

    let derive_decoded = AddressBook::sproto_decode(&derive_encoded).unwrap();
    assert_eq!(
        derive_decoded.person.len(),
        ab.person.len(),
        "derive roundtrip failed"
    );

    eprintln!(
        "AddressBook: serde {} bytes (packed {}), derive {} bytes (packed {})",
        serde_encoded.len(),
        serde_packed.len(),
        derive_encoded.len(),
        derive_packed.len(),
    );

    // Run Serde benchmark
    run_benchmark("serde", mode, count, || match mode {
        "encode" => bench_serde_ab_encode(&schema, st, &ab, count),
        "decode" => bench_serde_ab_decode(&schema, st, &serde_encoded, count),
        "encode_pack" => bench_serde_ab_encode_pack(&schema, st, &ab, count),
        "unpack_decode" => bench_serde_ab_unpack_decode(&schema, st, &serde_packed, count),
        _ => unreachable!(),
    });

    // Run Derive benchmark
    run_benchmark("derive", mode, count, || match mode {
        "encode" => bench_derive_ab_encode(&ab, count),
        "decode" => bench_derive_ab_decode(&derive_encoded, count),
        "encode_pack" => bench_derive_ab_encode_pack(&ab, count),
        "unpack_decode" => bench_derive_ab_unpack_decode(&derive_packed, count),
        _ => unreachable!(),
    });
}
