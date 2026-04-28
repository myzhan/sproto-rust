//! Benchmark for sproto encode/decode/pack/unpack operations.
//!
//! Usage:
//!   cargo build --release --example benchmark
//!   ./target/release/examples/benchmark [--count N] [--mode MODE] [--api API]
//!
//! Modes: encode, decode, encode_pack, unpack_decode
//! APIs:  direct, derive

use std::hint::black_box;
use std::time::Instant;

use sproto::codec::{StructDecoder, StructEncoder};
use sproto::pack;
use sproto::types::{Field, FieldType, SprotoType};
use sproto::{Sproto, SprotoDecode, SprotoEncode};

// ============================================================================
// Schema Creation
// ============================================================================

fn create_addressbook_schema() -> Sproto {
    let mut s = Sproto::new();
    let phone_idx = s.add_type(
        "PhoneNumber",
        vec![
            Field::new("number", 0, FieldType::String),
            Field::new("type", 1, FieldType::Integer),
            Field::new("real", 2, FieldType::Double),
        ],
    );
    let person_idx = s.add_type(
        "Person",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("id", 1, FieldType::Integer),
            Field::new("email", 2, FieldType::String),
            Field::array("phone", 3, FieldType::Struct(phone_idx)),
        ],
    );
    let human_idx = s.add_type(
        "Human",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("age", 1, FieldType::Integer),
            Field::new("marital", 2, FieldType::Boolean),
            // children references Human itself (self-referential, index = 2)
            Field::array("children", 3, FieldType::Struct(2)),
        ],
    );
    let _ = human_idx;
    s.add_type(
        "AddressBook",
        vec![
            Field::array("person", 0, FieldType::Struct(person_idx)),
            Field::array("human", 1, FieldType::Struct(human_idx)),
        ],
    );
    s
}

// ============================================================================
// Encode/Decode using low-level API
// ============================================================================

fn encode_addressbook(schema: &Sproto, st: &SprotoType) -> Vec<u8> {
    let mut buf = Vec::with_capacity(512);
    let mut enc = StructEncoder::new(schema, st, &mut buf);

    // person array
    enc.encode_struct_array(0, |arr| {
        // Alice
        arr.encode_element(|p| {
            p.set_string(0, "Alice")?;
            p.set_integer(1, 10000)?;
            p.encode_struct_array(3, |phones| {
                phones.encode_element(|ph| {
                    ph.set_string(0, "123456789")?;
                    ph.set_integer(1, 1)?;
                    ph.set_double(2, 1.234567)?;
                    Ok(())
                })?;
                phones.encode_element(|ph| {
                    ph.set_string(0, "87654321")?;
                    ph.set_integer(1, 2)?;
                    ph.set_double(2, 5567.12345)?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?;
        // Bob
        arr.encode_element(|p| {
            p.set_string(0, "Bob")?;
            p.set_integer(1, 20000)?;
            p.encode_struct_array(3, |phones| {
                phones.encode_element(|ph| {
                    ph.set_string(0, "01234567890")?;
                    ph.set_integer(1, 3)?;
                    ph.set_double(2, 567.1378)?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })
    .unwrap();

    // human array
    enc.encode_struct_array(1, |arr| {
        // kkkk (no children)
        arr.encode_element(|h| {
            h.set_string(0, "kkkk")?;
            h.set_integer(1, 11)?;
            h.set_bool(2, true)?;
            Ok(())
        })?;
        // dddd (with children)
        arr.encode_element(|h| {
            h.set_string(0, "dddd")?;
            h.set_integer(1, 22)?;
            h.set_bool(2, false)?;
            h.encode_struct_array(3, |children| {
                children.encode_element(|c| {
                    c.set_string(0, "cccc")?;
                    c.set_integer(1, 33)?;
                    c.set_bool(2, false)?;
                    Ok(())
                })?;
                children.encode_element(|c| {
                    c.set_string(0, "ffff")?;
                    c.set_integer(1, 44)?;
                    c.set_bool(2, false)?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    })
    .unwrap();

    enc.finish();
    buf
}

fn decode_addressbook(schema: &Sproto, st: &SprotoType, data: &[u8]) -> usize {
    let mut dec = StructDecoder::new(schema, st, data).unwrap();
    let mut person_count = 0;
    while let Some(f) = dec.next_field().unwrap() {
        match f.tag() {
            0 => {
                // person array
                for elem in f.as_struct_iter().unwrap() {
                    let mut sub = elem.unwrap();
                    while let Some(sf) = sub.next_field().unwrap() {
                        match sf.tag() {
                            0 => {
                                let _ = sf.as_string().unwrap();
                            }
                            1 => {
                                let _ = sf.as_integer().unwrap();
                            }
                            3 => {
                                for phone in sf.as_struct_iter().unwrap() {
                                    let mut ph = phone.unwrap();
                                    while let Some(pf) = ph.next_field().unwrap() {
                                        match pf.tag() {
                                            0 => {
                                                let _ = pf.as_string().unwrap();
                                            }
                                            1 => {
                                                let _ = pf.as_integer().unwrap();
                                            }
                                            2 => {
                                                let _ = pf.as_double().unwrap();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                    person_count += 1;
                }
            }
            1 => {
                // human array - just iterate
                for elem in f.as_struct_iter().unwrap() {
                    let mut sub = elem.unwrap();
                    while let Some(sf) = sub.next_field().unwrap() {
                        match sf.tag() {
                            0 => {
                                let _ = sf.as_string().unwrap();
                            }
                            1 => {
                                let _ = sf.as_integer().unwrap();
                            }
                            2 => {
                                let _ = sf.as_bool().unwrap();
                            }
                            3 => {
                                // children
                                for child in sf.as_struct_iter().unwrap() {
                                    let mut c = child.unwrap();
                                    while let Some(cf) = c.next_field().unwrap() {
                                        match cf.tag() {
                                            0 => {
                                                let _ = cf.as_string().unwrap();
                                            }
                                            1 => {
                                                let _ = cf.as_integer().unwrap();
                                            }
                                            2 => {
                                                let _ = cf.as_bool().unwrap();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }
    person_count
}

// ============================================================================
// Derive API Structs
// ============================================================================

#[derive(SprotoEncode, SprotoDecode, Clone)]
struct PhoneNumberDerive {
    #[sproto(tag = 0)]
    number: String,
    #[sproto(tag = 1)]
    r#type: i64,
    #[sproto(tag = 2)]
    real: f64,
}

#[derive(SprotoEncode, SprotoDecode, Clone)]
struct PersonDerive {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    id: i64,
    #[sproto(tag = 2)]
    email: String,
    #[sproto(tag = 3)]
    phone: Vec<PhoneNumberDerive>,
}

#[derive(SprotoEncode, SprotoDecode, Clone)]
struct HumanDerive {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    marital: bool,
    #[sproto(tag = 3)]
    children: Vec<HumanDerive>,
}

#[derive(SprotoEncode, SprotoDecode, Clone)]
struct AddressBookDerive {
    #[sproto(tag = 0)]
    person: Vec<PersonDerive>,
    #[sproto(tag = 1)]
    human: Vec<HumanDerive>,
}

fn create_addressbook_value() -> AddressBookDerive {
    AddressBookDerive {
        person: vec![
            PersonDerive {
                name: "Alice".to_string(),
                id: 10000,
                email: String::new(),
                phone: vec![
                    PhoneNumberDerive {
                        number: "123456789".to_string(),
                        r#type: 1,
                        real: 1.234567,
                    },
                    PhoneNumberDerive {
                        number: "87654321".to_string(),
                        r#type: 2,
                        real: 5567.12345,
                    },
                ],
            },
            PersonDerive {
                name: "Bob".to_string(),
                id: 20000,
                email: String::new(),
                phone: vec![PhoneNumberDerive {
                    number: "01234567890".to_string(),
                    r#type: 3,
                    real: 567.1378,
                }],
            },
        ],
        human: vec![
            HumanDerive {
                name: "kkkk".to_string(),
                age: 11,
                marital: true,
                children: vec![],
            },
            HumanDerive {
                name: "dddd".to_string(),
                age: 22,
                marital: false,
                children: vec![
                    HumanDerive {
                        name: "cccc".to_string(),
                        age: 33,
                        marital: false,
                        children: vec![],
                    },
                    HumanDerive {
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

fn encode_addressbook_derive(schema: &Sproto, value: &AddressBookDerive) -> Vec<u8> {
    sproto::to_bytes(schema, "AddressBook", value).unwrap()
}

fn decode_addressbook_derive(schema: &Sproto, data: &[u8]) -> AddressBookDerive {
    sproto::from_bytes::<AddressBookDerive>(schema, "AddressBook", data).unwrap()
}

// ============================================================================
// Benchmark Functions
// ============================================================================

fn bench_ab_encode(schema: &Sproto, st: &SprotoType, count: usize) {
    for _ in 0..count {
        let _ = black_box(encode_addressbook(schema, st));
    }
}

fn bench_ab_decode(schema: &Sproto, st: &SprotoType, data: &[u8], count: usize) {
    for _ in 0..count {
        let _ = black_box(decode_addressbook(schema, st, data));
    }
}

fn bench_ab_encode_pack(schema: &Sproto, st: &SprotoType, count: usize) {
    for _ in 0..count {
        let encoded = encode_addressbook(schema, st);
        let _ = black_box(pack::pack(&encoded));
    }
}

fn bench_ab_unpack_decode(schema: &Sproto, st: &SprotoType, packed: &[u8], count: usize) {
    for _ in 0..count {
        let unpacked = pack::unpack(packed).unwrap();
        let _ = black_box(decode_addressbook(schema, st, &unpacked));
    }
}

fn bench_derive_encode(schema: &Sproto, value: &AddressBookDerive, count: usize) {
    for _ in 0..count {
        let _ = black_box(encode_addressbook_derive(schema, black_box(value)));
    }
}

fn bench_derive_decode(schema: &Sproto, data: &[u8], count: usize) {
    for _ in 0..count {
        let _ = black_box(decode_addressbook_derive(schema, black_box(data)));
    }
}

fn bench_derive_encode_pack(schema: &Sproto, value: &AddressBookDerive, count: usize) {
    for _ in 0..count {
        let encoded = encode_addressbook_derive(schema, value);
        let _ = black_box(pack::pack(&encoded));
    }
}

fn bench_derive_unpack_decode(schema: &Sproto, packed: &[u8], count: usize) {
    for _ in 0..count {
        let unpacked = pack::unpack(packed).unwrap();
        let _ = black_box(decode_addressbook_derive(schema, &unpacked));
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
    eprintln!("  --api API              API to benchmark (default: direct)");
    eprintln!("                         direct, derive");
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
    let api = parse_arg(&args, "--api", "direct");

    // Validate arguments
    if !["encode", "decode", "encode_pack", "unpack_decode"].contains(&mode.as_str()) {
        eprintln!(
            "Unknown --mode: {}. Use: encode, decode, encode_pack, unpack_decode",
            mode
        );
        std::process::exit(1);
    }
    if !["direct", "derive"].contains(&api.as_str()) {
        eprintln!("Unknown --api: {}. Use: direct, derive", api);
        std::process::exit(1);
    }

    let schema = create_addressbook_schema();
    let st = schema.get_type("AddressBook").unwrap();

    let encoded = encode_addressbook(&schema, st);
    let packed = pack::pack(&encoded);

    let person_count = decode_addressbook(&schema, st, &encoded);
    assert_eq!(person_count, 2, "roundtrip failed");

    let ab_value = create_addressbook_value();
    let derive_encoded = encode_addressbook_derive(&schema, &ab_value);
    let derive_packed = pack::pack(&derive_encoded);

    eprintln!(
        "AddressBook: encoded {} bytes, packed {} bytes",
        encoded.len(),
        packed.len()
    );

    match api.as_str() {
        "direct" => {
            run_benchmark("direct", &mode, count, || match mode.as_str() {
                "encode" => bench_ab_encode(&schema, st, count),
                "decode" => bench_ab_decode(&schema, st, &encoded, count),
                "encode_pack" => bench_ab_encode_pack(&schema, st, count),
                "unpack_decode" => bench_ab_unpack_decode(&schema, st, &packed, count),
                _ => unreachable!(),
            });
        }
        "derive" => {
            run_benchmark("derive", &mode, count, || match mode.as_str() {
                "encode" => bench_derive_encode(&schema, &ab_value, count),
                "decode" => bench_derive_decode(&schema, &derive_encoded, count),
                "encode_pack" => bench_derive_encode_pack(&schema, &ab_value, count),
                "unpack_decode" => bench_derive_unpack_decode(&schema, &derive_packed, count),
                _ => unreachable!(),
            });
        }
        _ => unreachable!(),
    }
}
