//! Cross-validation: pack/unpack with C-generated fixture files.

use sproto::pack;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
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

pack_test!(
    test_pack_simple_struct,
    test_unpack_simple_struct,
    "simple_struct_encoded.bin",
    "simple_struct_packed.bin"
);
pack_test!(
    test_pack_struct_array,
    test_unpack_struct_array,
    "struct_array_encoded.bin",
    "struct_array_packed.bin"
);
pack_test!(
    test_pack_number_array,
    test_unpack_number_array,
    "number_array_encoded.bin",
    "number_array_packed.bin"
);
pack_test!(
    test_pack_big_number_array,
    test_unpack_big_number_array,
    "big_number_array_encoded.bin",
    "big_number_array_packed.bin"
);
pack_test!(
    test_pack_bool_array,
    test_unpack_bool_array,
    "bool_array_encoded.bin",
    "bool_array_packed.bin"
);
pack_test!(
    test_pack_number,
    test_unpack_number,
    "number_encoded.bin",
    "number_packed.bin"
);
pack_test!(
    test_pack_double,
    test_unpack_double,
    "double_encoded.bin",
    "double_packed.bin"
);
pack_test!(
    test_pack_fixed_point,
    test_unpack_fixed_point,
    "fixed_point_encoded.bin",
    "fixed_point_packed.bin"
);

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
