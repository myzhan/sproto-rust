//! Fuzz target: derive API encode/decode round-trip.
//!
//! Generates arbitrary `FuzzPerson` values, encodes them with `SprotoEncode`,
//! decodes with `SprotoDecode`, and verifies the decoded value matches the
//! original.

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;
use sproto::{from_bytes, to_bytes};

fuzz_target!(|person: common::FuzzPerson| {
    let schema = common::build_fuzz_schema();
    let bytes = to_bytes(&schema, "FuzzPerson", &person).unwrap();
    let decoded: common::FuzzPerson =
        from_bytes(&schema, "FuzzPerson", &bytes).unwrap();
    common::assert_person_eq(&person, &decoded);
});
