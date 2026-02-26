//! Benchmarks for sproto encode/decode/pack/unpack operations.
//!
//! Run with: cargo bench
//! Or: make benchmark

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use serde::{Deserialize, Serialize};
use sproto::codec;
use sproto::pack;
use sproto::types::{Field, FieldType, Sproto, SprotoType};
use sproto::value::SprotoValue;
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
    let person_type = SprotoType {
        name: "Person".to_string(),
        fields: vec![
            Field {
                name: "name".to_string(),
                tag: 0,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "age".to_string(),
                tag: 1,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "active".to_string(),
                tag: 2,
                field_type: FieldType::Boolean,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 3,
    };

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
    let user_type = SprotoType {
        name: "UserProfile".to_string(),
        fields: vec![
            Field {
                name: "id".to_string(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "username".to_string(),
                tag: 1,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "email".to_string(),
                tag: 2,
                field_type: FieldType::String,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "age".to_string(),
                tag: 3,
                field_type: FieldType::Integer,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "verified".to_string(),
                tag: 4,
                field_type: FieldType::Boolean,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "score".to_string(),
                tag: 5,
                field_type: FieldType::Double,
                is_array: false,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 6,
    };

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
    let data_type = SprotoType {
        name: "DataSet".to_string(),
        fields: vec![
            Field {
                name: "numbers".to_string(),
                tag: 0,
                field_type: FieldType::Integer,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
            Field {
                name: "values".to_string(),
                tag: 1,
                field_type: FieldType::Double,
                is_array: true,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            },
        ],
        base_tag: 0,
        maxn: 2,
    };

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
// Encode Benchmarks
// ============================================================================

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode");

    // Simple Person struct
    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person_value = SprotoValue::from_fields(vec![
        ("name", "Alice".into()),
        ("age", 30i64.into()),
        ("active", true.into()),
    ]);

    group.throughput(Throughput::Elements(1));
    group.bench_function("person/value_api", |b| {
        b.iter(|| {
            codec::encode(
                black_box(&person_sproto),
                black_box(person_type),
                black_box(&person_value),
            )
            .unwrap()
        })
    });

    // UserProfile struct (more fields)
    let user_sproto = create_user_profile_schema();
    let user_type = user_sproto.get_type("UserProfile").unwrap();
    let user_value = SprotoValue::from_fields(vec![
        ("id", 12345i64.into()),
        ("username", "alice_wonder".into()),
        ("email", "alice@example.com".into()),
        ("age", 28i64.into()),
        ("verified", true.into()),
        ("score", 98.5f64.into()),
    ]);

    group.bench_function("user_profile/value_api", |b| {
        b.iter(|| {
            codec::encode(
                black_box(&user_sproto),
                black_box(user_type),
                black_box(&user_value),
            )
            .unwrap()
        })
    });

    // DataSet with arrays (100 elements)
    let data_sproto = create_dataset_schema();
    let data_type = data_sproto.get_type("DataSet").unwrap();
    let numbers: Vec<SprotoValue> = (0..100).map(|i| SprotoValue::Integer(i)).collect();
    let values: Vec<SprotoValue> = (0..100).map(|i| SprotoValue::Double(i as f64 * 0.1)).collect();
    let data_value = SprotoValue::from_fields(vec![
        ("numbers", SprotoValue::Array(numbers)),
        ("values", SprotoValue::Array(values)),
    ]);

    group.bench_function("dataset_100/value_api", |b| {
        b.iter(|| {
            codec::encode(
                black_box(&data_sproto),
                black_box(data_type),
                black_box(&data_value),
            )
            .unwrap()
        })
    });

    group.finish();
}

fn bench_encode_serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_serde");

    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person = Person {
        name: "Alice".to_string(),
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
        username: "alice_wonder".to_string(),
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

    group.finish();
}

fn bench_encode_derive(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_derive");

    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        active: true,
    };

    group.throughput(Throughput::Elements(1));
    group.bench_function("person/derive", |b| {
        b.iter(|| black_box(&person).sproto_encode().unwrap())
    });

    let user = UserProfile {
        id: 12345,
        username: "alice_wonder".to_string(),
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
// Decode Benchmarks
// ============================================================================

fn bench_decode(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode");

    // Person
    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person_value = SprotoValue::from_fields(vec![
        ("name", "Alice".into()),
        ("age", 30i64.into()),
        ("active", true.into()),
    ]);
    let person_bytes = codec::encode(&person_sproto, person_type, &person_value).unwrap();

    group.throughput(Throughput::Bytes(person_bytes.len() as u64));
    group.bench_function("person/value_api", |b| {
        b.iter(|| {
            codec::decode(
                black_box(&person_sproto),
                black_box(person_type),
                black_box(&person_bytes),
            )
            .unwrap()
        })
    });

    // UserProfile
    let user_sproto = create_user_profile_schema();
    let user_type = user_sproto.get_type("UserProfile").unwrap();
    let user_value = SprotoValue::from_fields(vec![
        ("id", 12345i64.into()),
        ("username", "alice_wonder".into()),
        ("email", "alice@example.com".into()),
        ("age", 28i64.into()),
        ("verified", true.into()),
        ("score", 98.5f64.into()),
    ]);
    let user_bytes = codec::encode(&user_sproto, user_type, &user_value).unwrap();

    group.throughput(Throughput::Bytes(user_bytes.len() as u64));
    group.bench_function("user_profile/value_api", |b| {
        b.iter(|| {
            codec::decode(
                black_box(&user_sproto),
                black_box(user_type),
                black_box(&user_bytes),
            )
            .unwrap()
        })
    });

    // DataSet with arrays
    let data_sproto = create_dataset_schema();
    let data_type = data_sproto.get_type("DataSet").unwrap();
    let numbers: Vec<SprotoValue> = (0..100).map(|i| SprotoValue::Integer(i)).collect();
    let values: Vec<SprotoValue> = (0..100).map(|i| SprotoValue::Double(i as f64 * 0.1)).collect();
    let data_value = SprotoValue::from_fields(vec![
        ("numbers", SprotoValue::Array(numbers)),
        ("values", SprotoValue::Array(values)),
    ]);
    let data_bytes = codec::encode(&data_sproto, data_type, &data_value).unwrap();

    group.throughput(Throughput::Bytes(data_bytes.len() as u64));
    group.bench_function("dataset_100/value_api", |b| {
        b.iter(|| {
            codec::decode(
                black_box(&data_sproto),
                black_box(data_type),
                black_box(&data_bytes),
            )
            .unwrap()
        })
    });

    group.finish();
}

fn bench_decode_serde(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_serde");

    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person = Person {
        name: "Alice".to_string(),
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
        username: "alice_wonder".to_string(),
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

fn bench_decode_derive(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_derive");

    let person = Person {
        name: "Alice".to_string(),
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
        username: "alice_wonder".to_string(),
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

    // Small data (Person encoded)
    let person_sproto = create_person_schema();
    let person_type = person_sproto.get_type("Person").unwrap();
    let person_value = SprotoValue::from_fields(vec![
        ("name", "Alice".into()),
        ("age", 30i64.into()),
        ("active", true.into()),
    ]);
    let small_data = codec::encode(&person_sproto, person_type, &person_value).unwrap();

    group.throughput(Throughput::Bytes(small_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("small", small_data.len()),
        &small_data,
        |b, data| {
            b.iter(|| pack::pack(black_box(data)))
        },
    );

    // Medium data (UserProfile encoded)
    let user_sproto = create_user_profile_schema();
    let user_type = user_sproto.get_type("UserProfile").unwrap();
    let user_value = SprotoValue::from_fields(vec![
        ("id", 12345i64.into()),
        ("username", "alice_wonder".into()),
        ("email", "alice@example.com".into()),
        ("age", 28i64.into()),
        ("verified", true.into()),
        ("score", 98.5f64.into()),
    ]);
    let medium_data = codec::encode(&user_sproto, user_type, &user_value).unwrap();

    group.throughput(Throughput::Bytes(medium_data.len() as u64));
    group.bench_with_input(
        BenchmarkId::new("medium", medium_data.len()),
        &medium_data,
        |b, data| {
            b.iter(|| pack::pack(black_box(data)))
        },
    );

    // Large data (DataSet with 1000 elements)
    let data_sproto = create_dataset_schema();
    let data_type = data_sproto.get_type("DataSet").unwrap();
    let numbers: Vec<SprotoValue> = (0..1000).map(|i| SprotoValue::Integer(i)).collect();
    let values: Vec<SprotoValue> = (0..1000).map(|i| SprotoValue::Double(i as f64 * 0.1)).collect();
    let data_value = SprotoValue::from_fields(vec![
        ("numbers", SprotoValue::Array(numbers)),
        ("values", SprotoValue::Array(values)),
    ]);
    let large_data = codec::encode(&data_sproto, data_type, &data_value).unwrap();

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
    let person_value = SprotoValue::from_fields(vec![
        ("name", "Alice".into()),
        ("age", 30i64.into()),
        ("active", true.into()),
    ]);
    let small_encoded = codec::encode(&person_sproto, person_type, &person_value).unwrap();
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
    let user_value = SprotoValue::from_fields(vec![
        ("id", 12345i64.into()),
        ("username", "alice_wonder".into()),
        ("email", "alice@example.com".into()),
        ("age", 28i64.into()),
        ("verified", true.into()),
        ("score", 98.5f64.into()),
    ]);
    let medium_encoded = codec::encode(&user_sproto, user_type, &user_value).unwrap();
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
    let data_sproto = create_dataset_schema();
    let data_type = data_sproto.get_type("DataSet").unwrap();
    let numbers: Vec<SprotoValue> = (0..1000).map(|i| SprotoValue::Integer(i)).collect();
    let values: Vec<SprotoValue> = (0..1000).map(|i| SprotoValue::Double(i as f64 * 0.1)).collect();
    let data_value = SprotoValue::from_fields(vec![
        ("numbers", SprotoValue::Array(numbers)),
        ("values", SprotoValue::Array(values)),
    ]);
    let large_encoded = codec::encode(&data_sproto, data_type, &data_value).unwrap();
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
    bench_encode,
    bench_encode_serde,
    bench_encode_derive,
    bench_decode,
    bench_decode_serde,
    bench_decode_derive,
    bench_pack,
    bench_unpack,
);

criterion_main!(benches);
