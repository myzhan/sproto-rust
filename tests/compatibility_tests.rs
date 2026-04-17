//! Tests for backward/forward compatibility with missing fields.
//!
//! These tests verify that decode handles missing fields correctly:
//! - Option<T> fields: default to None
//! - #[sproto(default)] fields: use Default::default()
//! - Required fields: return error
//! - Unknown fields in binary: silently ignored (forward compatibility)

use serde::{Deserialize, Serialize};
use sproto::{parser, SprotoDecode, SprotoEncode};

// =============================================================================
// Test Schemas - V1 (old) and V2 (new with additional fields)
// =============================================================================

fn schema_v1() -> sproto::Sproto {
    parser::parse(
        r#"
        .Person {
            name 0 : string
            age 1 : integer
        }
    "#,
    )
    .unwrap()
}

fn schema_v2() -> sproto::Sproto {
    parser::parse(
        r#"
        .Person {
            name 0 : string
            age 1 : integer
            email 2 : string
            score 3 : double
        }
    "#,
    )
    .unwrap()
}

// =============================================================================
// Serde API: Cross-version Compatibility Tests
// =============================================================================

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PersonV1Serde {
    name: String,
    age: i64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PersonV2WithOptional {
    name: String,
    age: i64,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    score: Option<f64>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct PersonV2Full {
    name: String,
    age: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Default)]
struct PersonV2WithDefaults {
    name: String,
    age: i64,
    #[serde(default)]
    email: String,
    #[serde(default)]
    score: f64,
}

#[test]
fn test_serde_v1_encoded_decoded_as_v2_optional() {
    // Encode V1 data
    let schema = schema_v1();
    let person_type = schema.get_type("Person").unwrap();

    let person_v1 = PersonV1Serde {
        name: "Carol".into(),
        age: 28,
    };
    let bytes = sproto::serde::to_bytes(&schema, person_type, &person_v1).unwrap();

    // Decode as V2 with optional fields
    let schema_v2 = schema_v2();
    let person_type_v2 = schema_v2.get_type("Person").unwrap();

    let decoded: PersonV2WithOptional =
        sproto::serde::from_bytes(&schema_v2, person_type_v2, &bytes).unwrap();

    assert_eq!(decoded.name, "Carol");
    assert_eq!(decoded.age, 28);
    assert_eq!(decoded.email, None);
    assert_eq!(decoded.score, None);
}

#[test]
fn test_serde_v1_encoded_decoded_as_v2_defaults() {
    // Encode V1 data
    let schema = schema_v1();
    let person_type = schema.get_type("Person").unwrap();

    let person_v1 = PersonV1Serde {
        name: "Dave".into(),
        age: 35,
    };
    let bytes = sproto::serde::to_bytes(&schema, person_type, &person_v1).unwrap();

    // Decode as V2 with default fields
    let schema_v2 = schema_v2();
    let person_type_v2 = schema_v2.get_type("Person").unwrap();

    let decoded: PersonV2WithDefaults =
        sproto::serde::from_bytes(&schema_v2, person_type_v2, &bytes).unwrap();

    assert_eq!(decoded.name, "Dave");
    assert_eq!(decoded.age, 35);
    assert_eq!(decoded.email, ""); // Default for String
    assert_eq!(decoded.score, 0.0); // Default for f64
}

#[test]
fn test_serde_v2_encoded_decoded_as_v1() {
    // Encode V2 data (with extra fields)
    let schema_v2_inst = schema_v2();
    let person_type_v2 = schema_v2_inst.get_type("Person").unwrap();

    let person_v2 = PersonV2Full {
        name: "Bob".into(),
        age: 25,
        email: Some("bob@example.com".into()),
        score: Some(95.5),
    };
    let bytes = sproto::serde::to_bytes(&schema_v2_inst, person_type_v2, &person_v2).unwrap();

    // Decode as V1 — extra fields silently ignored
    let schema_v1_inst = schema_v1();
    let person_type_v1 = schema_v1_inst.get_type("Person").unwrap();

    let decoded: PersonV1Serde =
        sproto::serde::from_bytes(&schema_v1_inst, person_type_v1, &bytes).unwrap();

    assert_eq!(decoded.name, "Bob");
    assert_eq!(decoded.age, 25);
}

#[test]
fn test_serde_roundtrip_with_optional_none() {
    let schema = schema_v2();
    let person_type = schema.get_type("Person").unwrap();

    let person = PersonV2WithOptional {
        name: "Eve".into(),
        age: 22,
        email: None,
        score: Some(88.5),
    };

    let bytes = sproto::serde::to_bytes(&schema, person_type, &person).unwrap();
    let decoded: PersonV2WithOptional =
        sproto::serde::from_bytes(&schema, person_type, &bytes).unwrap();

    assert_eq!(decoded.name, "Eve");
    assert_eq!(decoded.age, 22);
    assert_eq!(decoded.email, None);
    assert_eq!(decoded.score, Some(88.5));
}

// =============================================================================
// Derive API: Missing Field Tests
// =============================================================================

mod derive_tests {
    use super::*;

    // V1 struct - basic fields
    #[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
    struct PersonV1 {
        #[sproto(tag = 0)]
        name: String,
        #[sproto(tag = 1)]
        age: i64,
    }

    // V2 struct with optional fields
    #[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
    struct PersonV2Optional {
        #[sproto(tag = 0)]
        name: String,
        #[sproto(tag = 1)]
        age: i64,
        #[sproto(tag = 2)]
        email: Option<String>,
        #[sproto(tag = 3)]
        score: Option<f64>,
    }

    // V2 struct with default fields
    #[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
    struct PersonV2Default {
        #[sproto(tag = 0)]
        name: String,
        #[sproto(tag = 1)]
        age: i64,
        #[sproto(tag = 2, default)]
        email: String,
        #[sproto(tag = 3, default)]
        score: f64,
    }

    // V2 struct with required fields (should fail on missing)
    #[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
    struct PersonV2Required {
        #[sproto(tag = 0)]
        name: String,
        #[sproto(tag = 1)]
        age: i64,
        #[sproto(tag = 2)]
        email: String, // Required!
    }

    #[test]
    fn test_derive_decode_with_optional_fields() {
        // Encode V1
        let v1 = PersonV1 {
            name: "Frank".into(),
            age: 40,
        };
        let bytes = v1.sproto_encode().unwrap();

        // Decode as V2 with optional
        let v2: PersonV2Optional = PersonV2Optional::sproto_decode(&bytes).unwrap();

        assert_eq!(v2.name, "Frank");
        assert_eq!(v2.age, 40);
        assert_eq!(v2.email, None);
        assert_eq!(v2.score, None);
    }

    #[test]
    fn test_derive_decode_with_default_fields() {
        // Encode V1
        let v1 = PersonV1 {
            name: "Grace".into(),
            age: 45,
        };
        let bytes = v1.sproto_encode().unwrap();

        // Decode as V2 with defaults
        let v2: PersonV2Default = PersonV2Default::sproto_decode(&bytes).unwrap();

        assert_eq!(v2.name, "Grace");
        assert_eq!(v2.age, 45);
        assert_eq!(v2.email, ""); // Default
        assert_eq!(v2.score, 0.0); // Default
    }

    #[test]
    fn test_derive_decode_required_field_missing_error() {
        // Encode V1
        let v1 = PersonV1 {
            name: "Henry".into(),
            age: 50,
        };
        let bytes = v1.sproto_encode().unwrap();

        // Decode as V2 with required email field - should fail
        let result = PersonV2Required::sproto_decode(&bytes);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            format!("{:?}", err).contains("missing required field"),
            "Error should mention missing field: {:?}",
            err
        );
    }

    #[test]
    fn test_derive_forward_compatibility_extra_fields_ignored() {
        // Encode V2 with all fields
        let v2 = PersonV2Optional {
            name: "Ivy".into(),
            age: 33,
            email: Some("ivy@example.com".into()),
            score: Some(92.0),
        };
        let bytes = v2.sproto_encode().unwrap();

        // Decode as V1 - extra fields should be ignored
        let v1: PersonV1 = PersonV1::sproto_decode(&bytes).unwrap();

        assert_eq!(v1.name, "Ivy");
        assert_eq!(v1.age, 33);
        // email and score are silently ignored
    }

    #[test]
    fn test_derive_roundtrip_partial_optional() {
        // Only some optional fields set
        let original = PersonV2Optional {
            name: "Jack".into(),
            age: 28,
            email: Some("jack@test.com".into()),
            score: None, // Not set
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded: PersonV2Optional = PersonV2Optional::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_all_optional_none() {
        #[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
        struct AllOptional {
            #[sproto(tag = 0)]
            a: Option<i64>,
            #[sproto(tag = 1)]
            b: Option<String>,
            #[sproto(tag = 2)]
            c: Option<bool>,
        }

        let original = AllOptional {
            a: None,
            b: None,
            c: None,
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded: AllOptional = AllOptional::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_derive_empty_binary_all_defaults() {
        #[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
        struct AllDefaults {
            #[sproto(tag = 0, default)]
            count: i64,
            #[sproto(tag = 1, default)]
            name: String,
            #[sproto(tag = 2, default)]
            active: bool,
        }

        // Empty sproto message (just header with 0 fields)
        let empty_bytes = vec![0u8, 0]; // field count = 0

        let decoded: AllDefaults = AllDefaults::sproto_decode(&empty_bytes).unwrap();

        assert_eq!(decoded.count, 0);
        assert_eq!(decoded.name, "");
        assert!(!decoded.active);
    }
}

// =============================================================================
// Array Field Missing Tests
// =============================================================================

mod array_tests {
    use super::*;

    #[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
    struct WithArrays {
        #[sproto(tag = 0)]
        name: String,
        #[sproto(tag = 1)]
        tags: Option<Vec<String>>,
        #[sproto(tag = 2, default)]
        scores: Vec<i64>,
    }

    #[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
    struct SimpleStruct {
        #[sproto(tag = 0)]
        name: String,
    }

    #[test]
    fn test_derive_missing_optional_array() {
        let simple = SimpleStruct {
            name: "Test".into(),
        };
        let bytes = simple.sproto_encode().unwrap();

        let decoded: WithArrays = WithArrays::sproto_decode(&bytes).unwrap();

        assert_eq!(decoded.name, "Test");
        assert_eq!(decoded.tags, None);
        assert_eq!(decoded.scores, Vec::<i64>::new()); // Default empty vec
    }

    #[test]
    fn test_derive_roundtrip_with_arrays() {
        let original = WithArrays {
            name: "ArrayTest".into(),
            tags: Some(vec!["tag1".into(), "tag2".into()]),
            scores: vec![100, 200, 300],
        };

        let bytes = original.sproto_encode().unwrap();
        let decoded: WithArrays = WithArrays::sproto_decode(&bytes).unwrap();

        assert_eq!(original, decoded);
    }
}
