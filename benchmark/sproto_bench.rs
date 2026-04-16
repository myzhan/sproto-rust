//! Benchmarks for sproto encode/decode/pack/unpack operations.
//!
//! Run with: cargo bench
//! Or: make benchmark

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use serde::{Deserialize, Serialize};
use sproto::pack;
use sproto::types::{Field, FieldType, Sproto, SprotoType};
use sproto::{SprotoDecode, SprotoEncode};
use std::collections::HashMap;

// ============================================================================
// Test Data Structures
// ============================================================================

/// Simple struct for benchmarking
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    active: bool,
}

/// Struct with more fields
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SprotoEncode, SprotoDecode)]
struct UserProfile {
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

/// Struct with arrays
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, SprotoEncode, SprotoDecode)]
struct DataSet {
    #[sproto(tag = 0)]
    numbers: Vec<i64>,
    #[sproto(tag = 1)]
    values: Vec<f64>,
}

// ============================================================================
// Schema Creation Helpers
// ============================================================================

fn create_person_schema() -> Sproto {
    let person_type = SprotoType::new(
        "Person".to_string(),
        vec![
            Field {
                name: "name".into(),
                tag: 0,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "age".into(),
                tag: 1,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "active".into(),
                tag: 2,
                field_type: FieldType::Boolean,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        0,
        3,
    );

    let mut types_by_name = HashMap::new();
    types_by_name.insert("Person".to_string(), 0);

    Sproto {
        types_list: vec![person_type],
        types_by_name,
        protocols: vec![],
        protocols_by_name: HashMap::new(),
        protocols_by_tag: HashMap::new(),
    }
}

fn create_user_profile_schema() -> Sproto {
    let user_type = SprotoType::new(
        "UserProfile".to_string(),
        vec![
            Field {
                name: "id".into(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "username".into(),
                tag: 1,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "email".into(),
                tag: 2,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "age".into(),
                tag: 3,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "verified".into(),
                tag: 4,
                field_type: FieldType::Boolean,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "score".into(),
                tag: 5,
                field_type: FieldType::Double,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        0,
        6,
    );

    let mut types_by_name = HashMap::new();
    types_by_name.insert("UserProfile".to_string(), 0);

    Sproto {
        types_list: vec![user_type],
        types_by_name,
        protocols: vec![],
        protocols_by_name: HashMap::new(),
        protocols_by_tag: HashMap::new(),
    }
}

fn create_dataset_schema() -> Sproto {
    let data_type = SprotoType::new(
        "DataSet".to_string(),
        vec![
            Field {
                name: "numbers".into(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "values".into(),
                tag: 1,
                field_type: FieldType::Double,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        0,
        2,
    );

    let mut types_by_name = HashMap::new();
    types_by_name.insert("DataSet".to_string(), 0);

    Sproto {
        types_list: vec![data_type],
        types_by_name,
        protocols: vec![],
        protocols_by_name: HashMap::new(),
        protocols_by_tag: HashMap::new(),
    }
}

// ============================================================================
// Encode Benchmarks - Serde API
// ============================================================================

fn bench_encode_serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_serde");

    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
    };

    group.throughput(Throughput::Elements(1));
    group.bench_function("person/serde", |b| {
        b.iter(|| {
            sproto::serde::to_bytes(
                black_box(&person_sproto),
                black_box(person_type),
                black_box(&person),
            )
            .unwrap()
        })
    });

    let user_sproto = create_user_profile_schema();
    let user_type = user_sproto.get_type("UserProfile").unwrap();
    let user = UserProfile {
        id: 12345,
        username: "alice_wonder".into(),
        email: "alice@example.com".to_string(),
        age: 28,
        verified: true,
        score: 98.5,
    };

    group.bench_function("user_profile/serde", |b| {
        b.iter(|| {
            sproto::serde::to_bytes(
                black_box(&user_sproto),
                black_box(user_type),
                black_box(&user),
            )
            .unwrap()
        })
    });

    let data_sproto = create_dataset_schema();
    let data_type = data_sproto.get_type("DataSet").unwrap();
    let dataset = DataSet {
        numbers: (0..100).collect(),
        values: (0..100).map(|i| i as f64 * 0.1).collect(),
    };

    group.bench_function("dataset_100/serde", |b| {
        b.iter(|| {
            sproto::serde::to_bytes(
                black_box(&data_sproto),
                black_box(data_type),
                black_box(&dataset),
            )
            .unwrap()
        })
    });

    group.finish();
}

// ============================================================================
// Encode Benchmarks - Derive API
// ============================================================================

fn bench_encode_derive(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_derive");

    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
    };

    group.throughput(Throughput::Elements(1));
    group.bench_function("person/derive", |b| {
        b.iter(|| black_box(&person).sproto_encode().unwrap())
    });

    let user = UserProfile {
        id: 12345,
        username: "alice_wonder".into(),
        email: "alice@example.com".to_string(),
        age: 28,
        verified: true,
        score: 98.5,
    };

    group.bench_function("user_profile/derive", |b| {
        b.iter(|| black_box(&user).sproto_encode().unwrap())
    });

    let dataset = DataSet {
        numbers: (0..100).collect(),
        values: (0..100).map(|i| i as f64 * 0.1).collect(),
    };

    group.bench_function("dataset_100/derive", |b| {
        b.iter(|| black_box(&dataset).sproto_encode().unwrap())
    });

    group.finish();
}

// ============================================================================
// Decode Benchmarks - Serde API
// ============================================================================

fn bench_decode_serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_serde");

    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
    };
    let person_bytes = sproto::serde::to_bytes(&person_sproto, person_type, &person).unwrap();

    group.throughput(Throughput::Bytes(person_bytes.len() as u64));
    group.bench_function("person/serde", |b| {
        b.iter(|| {
            sproto::serde::from_bytes::<Person>(
                black_box(&person_sproto),
                black_box(person_type),
                black_box(&person_bytes),
            )
            .unwrap()
        })
    });

    let user_sproto = create_user_profile_schema();
    let user_type = user_sproto.get_type("UserProfile").unwrap();
    let user = UserProfile {
        id: 12345,
        username: "alice_wonder".into(),
        email: "alice@example.com".to_string(),
        age: 28,
        verified: true,
        score: 98.5,
    };
    let user_bytes = sproto::serde::to_bytes(&user_sproto, user_type, &user).unwrap();

    group.throughput(Throughput::Bytes(user_bytes.len() as u64));
    group.bench_function("user_profile/serde", |b| {
        b.iter(|| {
            sproto::serde::from_bytes::<UserProfile>(
                black_box(&user_sproto),
                black_box(user_type),
                black_box(&user_bytes),
            )
            .unwrap()
        })
    });

    group.finish();
}

// ============================================================================
// Decode Benchmarks - Derive API
// ============================================================================

fn bench_decode_derive(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_derive");

    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
    };
    let person_bytes = person.sproto_encode().unwrap();

    group.throughput(Throughput::Bytes(person_bytes.len() as u64));
    group.bench_function("person/derive", |b| {
        b.iter(|| Person::sproto_decode(black_box(&person_bytes)).unwrap())
    });

    let user = UserProfile {
        id: 12345,
        username: "alice_wonder".into(),
        email: "alice@example.com".to_string(),
        age: 28,
        verified: true,
        score: 98.5,
    };
    let user_bytes = user.sproto_encode().unwrap();

    group.throughput(Throughput::Bytes(user_bytes.len() as u64));
    group.bench_function("user_profile/derive", |b| {
        b.iter(|| UserProfile::sproto_decode(black_box(&user_bytes)).unwrap())
    });

    let dataset = DataSet {
        numbers: (0..100).collect(),
        values: (0..100).map(|i| i as f64 * 0.1).collect(),
    };
    let dataset_bytes = dataset.sproto_encode().unwrap();

    group.throughput(Throughput::Bytes(dataset_bytes.len() as u64));
    group.bench_function("dataset_100/derive", |b| {
        b.iter(|| DataSet::sproto_decode(black_box(&dataset_bytes)).unwrap())
    });

    group.finish();
}

// ============================================================================
// Pack/Unpack Benchmarks
// ============================================================================

fn bench_pack(c: &mut Criterion) {
    let mut group = c.benchmark_group("pack");

    // Small data (Person encoded via serde)
    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
    };
    let small_data = sproto::serde::to_bytes(&person_sproto, person_type, &person).unwrap();

    group.throughput(Throughput::Bytes(small_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("small", small_data.len()),
        &small_data,
        |b, data| {
            b.iter(|| pack::pack(black_box(data)))
        },
    );

    // Medium data (UserProfile encoded via serde)
    let user_sproto = create_user_profile_schema();
    let user_type = user_sproto.get_type("UserProfile").unwrap();
    let user = UserProfile {
        id: 12345,
        username: "alice_wonder".into(),
        email: "alice@example.com".to_string(),
        age: 28,
        verified: true,
        score: 98.5,
    };
    let medium_data = sproto::serde::to_bytes(&user_sproto, user_type, &user).unwrap();

    group.throughput(Throughput::Bytes(medium_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("medium", medium_data.len()),
        &medium_data,
        |b, data| {
            b.iter(|| pack::pack(black_box(data)))
        },
    );

    // Large data (DataSet with 1000 elements via derive)
    let dataset = DataSet {
        numbers: (0..1000).collect(),
        values: (0..1000).map(|i| i as f64 * 0.1).collect(),
    };
    let large_data = dataset.sproto_encode().unwrap();

    group.throughput(Throughput::Bytes(large_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("large", large_data.len()),
        &large_data,
        |b, data| {
            b.iter(|| pack::pack(black_box(data)))
        },
    );

    group.finish();
}

fn bench_unpack(c: &mut Criterion) {
    let mut group = c.benchmark_group("unpack");

    // Small data
    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
    };
    let small_encoded = sproto::serde::to_bytes(&person_sproto, person_type, &person).unwrap();
    let small_packed = pack::pack(&small_encoded);

    group.throughput(Throughput::Bytes(small_packed.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("small", small_packed.len()),
        &small_packed,
        |b, data| {
            b.iter(|| pack::unpack(black_box(data)).unwrap())
        },
    );

    // Medium data
    let user_sproto = create_user_profile_schema();
    let user_type = user_sproto.get_type("UserProfile").unwrap();
    let user = UserProfile {
        id: 12345,
        username: "alice_wonder".into(),
        email: "alice@example.com".to_string(),
        age: 28,
        verified: true,
        score: 98.5,
    };
    let medium_encoded = sproto::serde::to_bytes(&user_sproto, user_type, &user).unwrap();
    let medium_packed = pack::pack(&medium_encoded);

    group.throughput(Throughput::Bytes(medium_packed.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("medium", medium_packed.len()),
        &medium_packed,
        |b, data| {
            b.iter(|| pack::unpack(black_box(data)).unwrap())
        },
    );

    // Large data
    let dataset = DataSet {
        numbers: (0..1000).collect(),
        values: (0..1000).map(|i| i as f64 * 0.1).collect(),
    };
    let large_encoded = dataset.sproto_encode().unwrap();
    let large_packed = pack::pack(&large_encoded);

    group.throughput(Throughput::Bytes(large_packed.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("large", large_packed.len()),
        &large_packed,
        |b, data| {
            b.iter(|| pack::unpack(black_box(data)).unwrap())
        },
    );

    group.finish();
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    bench_encode_serde,
    bench_encode_derive,
    bench_decode_serde,
    bench_decode_derive,
    bench_pack,
    bench_unpack,
);

criterion_main!(benches);
