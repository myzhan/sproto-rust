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

pack_test!(test_pack_ex1, test_unpack_ex1, "example1_encoded.bin", "example1_packed.bin");
pack_test!(test_pack_ex2, test_unpack_ex2, "example2_encoded.bin", "example2_packed.bin");
pack_test!(test_pack_ex3, test_unpack_ex3, "example3_encoded.bin", "example3_packed.bin");
pack_test!(test_pack_ex4, test_unpack_ex4, "example4_encoded.bin", "example4_packed.bin");
pack_test!(test_pack_ex5, test_unpack_ex5, "example5_encoded.bin", "example5_packed.bin");
pack_test!(test_pack_ex6, test_unpack_ex6, "example6_encoded.bin", "example6_packed.bin");
pack_test!(test_pack_ex7, test_unpack_ex7, "example7_encoded.bin", "example7_packed.bin");
pack_test!(test_pack_ex8, test_unpack_ex8, "example8_encoded.bin", "example8_packed.bin");

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
