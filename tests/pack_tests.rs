//! Cross-validation: pack/unpack with C-generated fixture files.

use sproto::pack;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

fn hexdump(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

// For each example: Rust pack(encoded) == C packed, Rust unpack(packed) == C encoded
macro_rules! pack_test {
    ($name_pack:ident, $name_unpack:ident, $encoded:expr, $packed:expr) => {
        #[test]
        fn $name_pack() {
            let encoded = testdata($encoded);
            let expected_packed = testdata($packed);

            let packed = pack::pack(&encoded);
            assert_eq!(
                hexdump(&packed),
                hexdump(&expected_packed),
                "pack mismatch for {}",
                $encoded,
            );
        }

        #[test]
        fn $name_unpack() {
            let packed = testdata($packed);
            let expected_encoded = testdata($encoded);

            let unpacked = pack::unpack(&packed).unwrap();
            // Unpacked may have trailing zeros from 8-byte padding
            assert_eq!(
                &unpacked[..expected_encoded.len()],
                &expected_encoded[..],
                "unpack mismatch for {}",
                $packed,
            );
        }
    };
}

pack_test!(test_pack_simple_struct, test_unpack_simple_struct, "example1_encoded.bin", "example1_packed.bin");
pack_test!(test_pack_struct_array, test_unpack_struct_array, "example2_encoded.bin", "example2_packed.bin");
pack_test!(test_pack_number_array, test_unpack_number_array, "example3_encoded.bin", "example3_packed.bin");
pack_test!(test_pack_big_number_array, test_unpack_big_number_array, "example4_encoded.bin", "example4_packed.bin");
pack_test!(test_pack_bool_array, test_unpack_bool_array, "example5_encoded.bin", "example5_packed.bin");
pack_test!(test_pack_number, test_unpack_number, "example6_encoded.bin", "example6_packed.bin");
pack_test!(test_pack_double, test_unpack_double, "example7_encoded.bin", "example7_packed.bin");
pack_test!(test_pack_fixed_point, test_unpack_fixed_point, "example8_encoded.bin", "example8_packed.bin");

// Pack/unpack round-trip on addressbook
#[test]
fn test_pack_unpack_addressbook() {
    let encoded = testdata("addressbook_encoded.bin");
    let expected_packed = testdata("addressbook_packed.bin");

    let packed = pack::pack(&encoded);
    assert_eq!(
        hexdump(&packed),
        hexdump(&expected_packed),
        "addressbook pack mismatch"
    );

    let unpacked = pack::unpack(&packed).unwrap();
    assert_eq!(
        &unpacked[..encoded.len()],
        &encoded[..],
        "addressbook unpack mismatch"
    );
}
