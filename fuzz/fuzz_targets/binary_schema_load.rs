//! Fuzz target: binary schema loader.
//!
//! Goal: ensure arbitrary bytes passed to `binary_schema::load_binary` never
//! panic; only `Result::Err` is acceptable.

#![no_main]

use libfuzzer_sys::fuzz_target;
use sproto::binary_schema;

fuzz_target!(|data: &[u8]| {
    let _ = binary_schema::load_binary(data);
});
