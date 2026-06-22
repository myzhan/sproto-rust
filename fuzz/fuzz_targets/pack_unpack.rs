//! Fuzz target: pack/unpack round-trip.
//!
//! Packing always produces valid input for unpacking, so this target checks that
//! no panic occurs and that the unpacked bytes match the original up to the
//! 8-byte zero-padding boundary.

#![no_main]

use libfuzzer_sys::fuzz_target;
use sproto::pack;

fuzz_target!(|data: &[u8]| {
    let packed = pack::pack(data);
    let unpacked = pack::unpack(&packed).expect("pack output must always be unpackable");
    assert!(
        unpacked.len() >= data.len(),
        "unpack output must be at least as long as input"
    );
    assert_eq!(
        &unpacked[..data.len()],
        data,
        "unpack must reproduce original bytes"
    );
});
