//! Fuzz target: decode arbitrary bytes with the direct `StructDecoder` API.
//!
//! Goal: ensure malformed input never panics and that all typed accessors are
//! exercised on real wire bytes.

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use sproto::codec::decoder::{DecodedField, StructDecoder};
use sproto::types::FieldType;

fn consume_field(field: &DecodedField) {
    let _ = field.as_bytes();

    match &field.field().field_type {
        FieldType::Integer => {
            let _ = field.as_integer();
        }
        FieldType::Boolean => {
            let _ = field.as_bool();
        }
        FieldType::Double => {
            let _ = field.as_double();
        }
        FieldType::String => {
            let _ = field.as_string();
        }
        FieldType::Binary => {
            let _ = field.as_bytes();
        }
        FieldType::Struct(_) => {
            if let Ok(mut sub) = field.as_struct() {
                while let Ok(Some(f)) = sub.next_field() {
                    consume_field(&f);
                }
            }
        }
    }

    if field.field().is_array {
        match &field.field().field_type {
            FieldType::Integer => {
                let _ = field.as_integer_array();
            }
            FieldType::Boolean => {
                let _ = field.as_bool_array();
            }
            FieldType::Double => {
                let _ = field.as_double_array();
            }
            FieldType::String => {
                let _ = field.as_string_array();
            }
            FieldType::Binary => {
                let _ = field.as_bytes_array();
            }
            FieldType::Struct(_) => {
                if let Ok(iter) = field.as_struct_iter() {
                    for mut sub in iter.flatten() {
                        while let Ok(Some(f)) = sub.next_field() {
                            consume_field(&f);
                        }
                    }
                }
            }
        }
    }
}

fuzz_target!(|data: &[u8]| {
    let schema = common::build_fuzz_schema();
    let st = schema.get_type("FuzzPerson").unwrap();
    if let Ok(mut dec) = StructDecoder::new(&schema, st, data) {
        while let Ok(Some(f)) = dec.next_field() {
            consume_field(&f);
        }
    }
});
