//! Benchmarks for sproto encode/decode/pack/unpack operations.
//!
//! Run with: cargo bench
//! Or: make benchmark

use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use sproto::codec::{StructDecoder, StructEncoder};
use sproto::pack;
use sproto::types::{Field, FieldType};
use sproto::{SprotoDecode, SprotoEncode};

// ============================================================================
// Derive Structs
// ============================================================================

#[derive(SprotoEncode, SprotoDecode, Clone)]
struct PersonDerive {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    active: bool,
}

#[derive(SprotoEncode, SprotoDecode, Clone)]
struct UserProfileDerive {
    #[sproto(tag = 0)]
    id: i64,
    #[sproto(tag = 1)]
    username: String,
    #[sproto(tag = 2)]
    email: String,
    #[sproto(tag = 3)]
    age: i64,
    #[sproto(tag = 4)]
    verified: bool,
    #[sproto(tag = 5)]
    score: f64,
}

#[derive(SprotoEncode, SprotoDecode, Clone)]
struct MapPersonDerive {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    id: i64,
}

#[derive(SprotoEncode, SprotoDecode, Clone)]
struct AddressBookDerive {
    #[sproto(tag = 0, key = 1, key_field = "id")]
    person: HashMap<i64, MapPersonDerive>,
    #[sproto(tag = 1, key = 0, value = 1)]
    phonemap: HashMap<String, i64>,
}

// ============================================================================
// Schema Creation Helpers
// ============================================================================

fn create_person_schema() -> sproto::Sproto {
    let mut s = sproto::Sproto::new();
    s.add_type(
        "Person",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("age", 1, FieldType::Integer),
            Field::new("active", 2, FieldType::Boolean),
        ],
    );
    s
}

fn create_user_profile_schema() -> sproto::Sproto {
    let mut s = sproto::Sproto::new();
    s.add_type(
        "UserProfile",
        vec![
            Field::new("id", 0, FieldType::Integer),
            Field::new("username", 1, FieldType::String),
            Field::new("email", 2, FieldType::String),
            Field::new("age", 3, FieldType::Integer),
            Field::new("verified", 4, FieldType::Boolean),
            Field::new("score", 5, FieldType::Double),
        ],
    );
    s
}

fn create_addressbook_schema() -> sproto::Sproto {
    let mut s = sproto::Sproto::new();
    let phone_idx = s.add_type(
        "PhoneNumber",
        vec![
            Field::new("number", 0, FieldType::String),
            Field::new("type", 1, FieldType::Integer),
        ],
    );
    let person_idx = s.add_type(
        "Person",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("id", 1, FieldType::Integer),
        ],
    );
    let mut person_field = Field::array("person", 0, FieldType::Struct(person_idx));
    person_field.key_tag = 1;
    let mut phonemap_field = Field::array("phonemap", 1, FieldType::Struct(phone_idx));
    phonemap_field.key_tag = 0;
    phonemap_field.is_map = true;
    s.add_type("AddressBook", vec![person_field, phonemap_field]);
    s
}

fn create_dataset_schema() -> sproto::Sproto {
    let mut s = sproto::Sproto::new();
    s.add_type(
        "DataSet",
        vec![
            Field::array("numbers", 0, FieldType::Integer),
            Field::array("values", 1, FieldType::Double),
        ],
    );
    s
}

// ============================================================================
// Encode Helpers
// ============================================================================

fn encode_person(sproto: &sproto::Sproto) -> Vec<u8> {
    let st = sproto.get_type("Person").unwrap();
    let mut buf = Vec::with_capacity(64);
    let mut enc = StructEncoder::new(sproto, st, &mut buf);
    enc.set_string(0, "Alice").unwrap();
    enc.set_integer(1, 30).unwrap();
    enc.set_bool(2, true).unwrap();
    enc.finish();
    buf
}

fn encode_user_profile(sproto: &sproto::Sproto) -> Vec<u8> {
    let st = sproto.get_type("UserProfile").unwrap();
    let mut buf = Vec::with_capacity(128);
    let mut enc = StructEncoder::new(sproto, st, &mut buf);
    enc.set_integer(0, 12345).unwrap();
    enc.set_string(1, "alice_wonder").unwrap();
    enc.set_string(2, "alice@example.com").unwrap();
    enc.set_integer(3, 28).unwrap();
    enc.set_bool(4, true).unwrap();
    enc.set_double(5, 98.5).unwrap();
    enc.finish();
    buf
}

fn encode_dataset(sproto: &sproto::Sproto, numbers: &[i64], values: &[f64]) -> Vec<u8> {
    let st = sproto.get_type("DataSet").unwrap();
    let mut buf = Vec::with_capacity(numbers.len() * 8 + values.len() * 8 + 64);
    let mut enc = StructEncoder::new(sproto, st, &mut buf);
    enc.set_integer_array(0, numbers).unwrap();
    enc.set_double_array(1, values).unwrap();
    enc.finish();
    buf
}

fn encode_addressbook(sproto: &sproto::Sproto) -> Vec<u8> {
    let st = sproto.get_type("AddressBook").unwrap();
    let mut buf = Vec::with_capacity(256);
    let mut enc = StructEncoder::new(sproto, st, &mut buf);
    enc.encode_struct_array(0, |arr| {
        for i in 0..10 {
            arr.encode_element(|e| {
                e.set_string(0, &format!("User{}", i))?;
                e.set_integer(1, 10000 + i)?;
                Ok(())
            })?;
        }
        Ok(())
    })
    .unwrap();
    enc.encode_struct_array(1, |arr| {
        for i in 0..10 {
            arr.encode_element(|e| {
                e.set_string(0, &format!("1234567{:02}", i))?;
                e.set_integer(1, i)?;
                Ok(())
            })?;
        }
        Ok(())
    })
    .unwrap();
    enc.finish();
    buf
}

fn decode_addressbook(sproto: &sproto::Sproto, data: &[u8]) {
    let st = sproto.get_type("AddressBook").unwrap();
    let mut dec = StructDecoder::new(sproto, st, data).unwrap();
    while let Some(f) = dec.next_field().unwrap() {
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
                    _ => {}
                }
            }
        }
    }
}

fn decode_person(sproto: &sproto::Sproto, data: &[u8]) {
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(sproto, st, data).unwrap();
    while let Some(f) = dec.next_field().unwrap() {
        match f.tag() {
            0 => {
                let _ = f.as_string().unwrap();
            }
            1 => {
                let _ = f.as_integer().unwrap();
            }
            2 => {
                let _ = f.as_bool().unwrap();
            }
            _ => {}
        }
    }
}

fn decode_user_profile(sproto: &sproto::Sproto, data: &[u8]) {
    let st = sproto.get_type("UserProfile").unwrap();
    let mut dec = StructDecoder::new(sproto, st, data).unwrap();
    while let Some(f) = dec.next_field().unwrap() {
        match f.tag() {
            0 | 3 => {
                let _ = f.as_integer().unwrap();
            }
            1 | 2 => {
                let _ = f.as_string().unwrap();
            }
            4 => {
                let _ = f.as_bool().unwrap();
            }
            5 => {
                let _ = f.as_double().unwrap();
            }
            _ => {}
        }
    }
}

// ============================================================================
// Encode Benchmarks
// ============================================================================

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");

    let person_sproto = create_person_schema();
    group.throughput(Throughput::Elements(1));
    group.bench_function("person", |b| {
        b.iter(|| encode_person(black_box(&person_sproto)))
    });

    let user_sproto = create_user_profile_schema();
    group.bench_function("user_profile", |b| {
        b.iter(|| encode_user_profile(black_box(&user_sproto)))
    });

    let ab_sproto = create_addressbook_schema();
    group.bench_function("addressbook", |b| {
        b.iter(|| encode_addressbook(black_box(&ab_sproto)))
    });

    let data_sproto = create_dataset_schema();
    let numbers: Vec<i64> = (0..100).collect();
    let values: Vec<f64> = (0..100).map(|i| i as f64 * 0.1).collect();
    group.bench_function("dataset_100", |b| {
        b.iter(|| {
            encode_dataset(
                black_box(&data_sproto),
                black_box(&numbers),
                black_box(&values),
            )
        })
    });

    group.finish();
}

// ============================================================================
// Decode Benchmarks
// ============================================================================

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    let person_sproto = create_person_schema();
    let person_bytes = encode_person(&person_sproto);
    group.throughput(Throughput::Bytes(person_bytes.len() as u64));
    group.bench_function("person", |b| {
        b.iter(|| decode_person(black_box(&person_sproto), black_box(&person_bytes)))
    });

    let user_sproto = create_user_profile_schema();
    let user_bytes = encode_user_profile(&user_sproto);
    group.throughput(Throughput::Bytes(user_bytes.len() as u64));
    group.bench_function("user_profile", |b| {
        b.iter(|| decode_user_profile(black_box(&user_sproto), black_box(&user_bytes)))
    });

    let ab_sproto = create_addressbook_schema();
    let ab_bytes = encode_addressbook(&ab_sproto);
    group.throughput(Throughput::Bytes(ab_bytes.len() as u64));
    group.bench_function("addressbook", |b| {
        b.iter(|| decode_addressbook(black_box(&ab_sproto), black_box(&ab_bytes)))
    });

    group.finish();
}

// ============================================================================
// Pack/Unpack Benchmarks
// ============================================================================

fn bench_pack(c: &mut Criterion) {
    let mut group = c.benchmark_group("pack");

    // Small data (Person)
    let person_sproto = create_person_schema();
    let small_data = encode_person(&person_sproto);
    group.throughput(Throughput::Bytes(small_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("small", small_data.len()),
        &small_data,
        |b, data| b.iter(|| pack::pack(black_box(data))),
    );

    // Medium data (UserProfile)
    let user_sproto = create_user_profile_schema();
    let medium_data = encode_user_profile(&user_sproto);
    group.throughput(Throughput::Bytes(medium_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("medium", medium_data.len()),
        &medium_data,
        |b, data| b.iter(|| pack::pack(black_box(data))),
    );

    // Large data (DataSet with 1000 elements)
    let data_sproto = create_dataset_schema();
    let numbers: Vec<i64> = (0..1000).collect();
    let values: Vec<f64> = (0..1000).map(|i| i as f64 * 0.1).collect();
    let large_data = encode_dataset(&data_sproto, &numbers, &values);
    group.throughput(Throughput::Bytes(large_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("large", large_data.len()),
        &large_data,
        |b, data| b.iter(|| pack::pack(black_box(data))),
    );

    group.finish();
}

fn bench_unpack(c: &mut Criterion) {
    let mut group = c.benchmark_group("unpack");

    // Small data
    let person_sproto = create_person_schema();
    let small_encoded = encode_person(&person_sproto);
    let small_packed = pack::pack(&small_encoded);
    group.throughput(Throughput::Bytes(small_packed.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("small", small_packed.len()),
        &small_packed,
        |b, data| b.iter(|| pack::unpack(black_box(data)).unwrap()),
    );

    // Medium data
    let user_sproto = create_user_profile_schema();
    let medium_encoded = encode_user_profile(&user_sproto);
    let medium_packed = pack::pack(&medium_encoded);
    group.throughput(Throughput::Bytes(medium_packed.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("medium", medium_packed.len()),
        &medium_packed,
        |b, data| b.iter(|| pack::unpack(black_box(data)).unwrap()),
    );

    // Large data
    let data_sproto = create_dataset_schema();
    let numbers: Vec<i64> = (0..1000).collect();
    let values: Vec<f64> = (0..1000).map(|i| i as f64 * 0.1).collect();
    let large_encoded = encode_dataset(&data_sproto, &numbers, &values);
    let large_packed = pack::pack(&large_encoded);
    group.throughput(Throughput::Bytes(large_packed.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("large", large_packed.len()),
        &large_packed,
        |b, data| b.iter(|| pack::unpack(black_box(data)).unwrap()),
    );

    group.finish();
}

// ============================================================================
// Derive Encode Benchmarks
// ============================================================================

fn bench_derive_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("derive_encode");

    let person_sproto = create_person_schema();
    let person = PersonDerive {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };
    group.throughput(Throughput::Elements(1));
    group.bench_function("person", |b| {
        b.iter(|| {
            sproto::to_bytes(black_box(&person_sproto), "Person", black_box(&person)).unwrap()
        })
    });

    let user_sproto = create_user_profile_schema();
    let user = UserProfileDerive {
        id: 12345,
        username: "alice_wonder".to_string(),
        email: "alice@example.com".to_string(),
        age: 28,
        verified: true,
        score: 98.5,
    };
    group.bench_function("user_profile", |b| {
        b.iter(|| {
            sproto::to_bytes(black_box(&user_sproto), "UserProfile", black_box(&user)).unwrap()
        })
    });

    let ab_sproto = create_addressbook_schema();
    let mut person_map = HashMap::new();
    for i in 0..10 {
        person_map.insert(
            10000 + i,
            MapPersonDerive {
                name: format!("User{}", i),
                id: 10000 + i,
            },
        );
    }
    let mut phonemap = HashMap::new();
    for i in 0..10 {
        phonemap.insert(format!("1234567{:02}", i), i);
    }
    let addressbook = AddressBookDerive {
        person: person_map,
        phonemap,
    };
    group.bench_function("addressbook", |b| {
        b.iter(|| {
            sproto::to_bytes(
                black_box(&ab_sproto),
                "AddressBook",
                black_box(&addressbook),
            )
            .unwrap()
        })
    });

    group.finish();
}

// ============================================================================
// Derive Decode Benchmarks
// ============================================================================

fn bench_derive_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("derive_decode");

    let person_sproto = create_person_schema();
    let person = PersonDerive {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };
    let person_bytes = sproto::to_bytes(&person_sproto, "Person", &person).unwrap();
    group.throughput(Throughput::Bytes(person_bytes.len() as u64));
    group.bench_function("person", |b| {
        b.iter(|| {
            sproto::from_bytes::<PersonDerive>(
                black_box(&person_sproto),
                "Person",
                black_box(&person_bytes),
            )
            .unwrap()
        })
    });

    let user_sproto = create_user_profile_schema();
    let user = UserProfileDerive {
        id: 12345,
        username: "alice_wonder".to_string(),
        email: "alice@example.com".to_string(),
        age: 28,
        verified: true,
        score: 98.5,
    };
    let user_bytes = sproto::to_bytes(&user_sproto, "UserProfile", &user).unwrap();
    group.throughput(Throughput::Bytes(user_bytes.len() as u64));
    group.bench_function("user_profile", |b| {
        b.iter(|| {
            sproto::from_bytes::<UserProfileDerive>(
                black_box(&user_sproto),
                "UserProfile",
                black_box(&user_bytes),
            )
            .unwrap()
        })
    });

    let ab_sproto = create_addressbook_schema();
    let mut person_map = HashMap::new();
    for i in 0..10i64 {
        person_map.insert(
            10000 + i,
            MapPersonDerive {
                name: format!("User{}", i),
                id: 10000 + i,
            },
        );
    }
    let mut phonemap = HashMap::new();
    for i in 0..10i64 {
        phonemap.insert(format!("1234567{:02}", i), i);
    }
    let addressbook = AddressBookDerive {
        person: person_map,
        phonemap,
    };
    let ab_bytes = sproto::to_bytes(&ab_sproto, "AddressBook", &addressbook).unwrap();
    group.throughput(Throughput::Bytes(ab_bytes.len() as u64));
    group.bench_function("addressbook", |b| {
        b.iter(|| {
            sproto::from_bytes::<AddressBookDerive>(
                black_box(&ab_sproto),
                "AddressBook",
                black_box(&ab_bytes),
            )
            .unwrap()
        })
    });

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_encode,
    bench_decode,
    bench_derive_encode,
    bench_derive_decode,
    bench_pack,
    bench_unpack,
);

criterion_main!(benches);
