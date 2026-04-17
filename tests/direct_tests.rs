//! Direct API tests using low-level StructEncoder/StructDecoder.
//!
//! These tests cover:
//! - Cross-validation with C/Lua binary fixtures
//! - Cross-validation with Go inline byte vectors
//! - Full type coverage (all scalar and array types)
//! - Edge cases and error handling

use sproto::binary_schema;
use sproto::codec::decoder::{DecodedField, StructDecoder};
use sproto::codec::encoder::StructEncoder;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

fn load_sproto() -> sproto::Sproto {
    binary_schema::load_binary(&testdata("schema.bin")).unwrap()
}

fn hexdump(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Helper: encode using StructEncoder with a closure.
fn direct_encode(
    sproto: &sproto::Sproto,
    type_name: &str,
    f: impl FnOnce(&mut StructEncoder) -> Result<(), sproto::error::EncodeError>,
) -> Vec<u8> {
    let st = sproto.get_type(type_name).unwrap();
    let mut buf = Vec::new();
    let mut enc = StructEncoder::new(sproto, st, &mut buf);
    f(&mut enc).unwrap();
    enc.finish();
    buf
}

/// Helper: collect all decoded fields as (tag, value) for assertions.
fn decode_fields<'a>(dec: &mut StructDecoder<'a>) -> Vec<DecodedField<'a>> {
    let mut fields = Vec::new();
    while let Some(f) = dec.next_field().unwrap() {
        fields.push(f);
    }
    fields
}

// =============================================================================
// Binary fixture cross-validation: Encode with StructEncoder, compare with .bin
// =============================================================================

#[test]
fn test_direct_encode_simple_struct() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_string(0, "Alice")?;
        enc.set_integer(1, 13)?;
        enc.set_bool(2, false)?;
        Ok(())
    });
    let expected = testdata("simple_struct_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode simple_struct mismatch"
    );
}

#[test]
fn test_direct_encode_all_scalars() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_string(0, "Alice")?;
        enc.set_integer(1, 30)?;
        enc.set_bool(2, true)?;
        enc.set_double(3, 0.01171875)?;
        enc.set_bytes(4, &[0x28, 0x29, 0x30, 0x31])?;
        enc.set_integer(5, 182)?; // fpn: integer(2), pre-scaled
        Ok(())
    });
    let expected = testdata("all_scalars_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode all_scalars mismatch"
    );
}

#[test]
fn test_direct_encode_nested_struct() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_string(0, "Alice")?;
        enc.encode_nested(7, |phone| {
            phone.set_string(0, "123456789")?;
            phone.set_integer(1, 1)?;
            Ok(())
        })?;
        Ok(())
    });
    let expected = testdata("nested_struct_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode nested_struct mismatch"
    );
}

#[test]
fn test_direct_encode_struct_array() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_string(0, "Bob")?;
        enc.set_integer(1, 40)?;
        enc.encode_struct_array(9, |arr| {
            arr.encode_element(|e| {
                e.set_string(0, "Alice")?;
                e.set_integer(1, 13)?;
                Ok(())
            })?;
            arr.encode_element(|e| {
                e.set_string(0, "Carol")?;
                e.set_integer(1, 5)?;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    });
    let expected = testdata("struct_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode struct_array mismatch"
    );
}

#[test]
fn test_direct_encode_int_array() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_integer_array(11, &[1, 2, 3, 4, 5])?;
        Ok(())
    });
    let expected = testdata("int_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode int_array mismatch"
    );
}

#[test]
fn test_direct_encode_big_int_array() {
    let sproto = load_sproto();
    let base: i64 = 1 << 32;
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_integer_array(11, &[base + 1, base + 2, base + 3])?;
        Ok(())
    });
    let expected = testdata("big_int_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode big_int_array mismatch"
    );
}

#[test]
fn test_direct_encode_bool_array() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_bool_array(12, &[false, true, false])?;
        Ok(())
    });
    let expected = testdata("bool_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode bool_array mismatch"
    );
}

#[test]
fn test_direct_encode_number() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_integer(1, 100000)?;
        enc.set_integer(6, -10000000000)?;
        Ok(())
    });
    let expected = testdata("number_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode number mismatch"
    );
}

#[test]
fn test_direct_encode_double() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_double(3, 0.01171875)?;
        enc.set_double_array(13, &[0.01171875, 23.0, 4.0])?;
        Ok(())
    });
    let expected = testdata("double_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode double mismatch"
    );
}

#[test]
fn test_direct_encode_string_array() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_string_array(10, &["hello", "world", "\u{4f60}\u{597d}"])?;
        Ok(())
    });
    let expected = testdata("string_array_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode string_array mismatch"
    );
}

#[test]
fn test_direct_encode_fixed_point() {
    let sproto = load_sproto();
    // fpn is integer(2): pre-scaled value 182
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_integer(5, 182)?;
        Ok(())
    });
    let expected = testdata("fixed_point_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode fixed_point mismatch"
    );
}

#[test]
fn test_direct_encode_full() {
    let sproto = load_sproto();
    let encoded = direct_encode(&sproto, "Person", |enc| {
        enc.set_string(0, "Alice")?;
        enc.set_integer(1, 30)?;
        enc.set_bool(2, true)?;
        enc.set_double(3, 0.01171875)?;
        enc.set_bytes(4, &[0xDE, 0xAD, 0xBE, 0xEF])?;
        enc.set_integer(5, 182)?;
        enc.set_integer(6, 10000)?;
        enc.encode_nested(7, |phone| {
            phone.set_string(0, "123456789")?;
            phone.set_integer(1, 1)?;
            Ok(())
        })?;
        enc.encode_struct_array(8, |phones| {
            phones.encode_element(|p| {
                p.set_string(0, "123456789")?;
                p.set_integer(1, 1)?;
                Ok(())
            })?;
            phones.encode_element(|p| {
                p.set_string(0, "87654321")?;
                p.set_integer(1, 2)?;
                Ok(())
            })?;
            Ok(())
        })?;
        enc.encode_struct_array(9, |children| {
            children.encode_element(|c| {
                c.set_string(0, "Bob")?;
                c.set_integer(1, 5)?;
                Ok(())
            })?;
            Ok(())
        })?;
        enc.set_string_array(10, &["hello", "world", "\u{4f60}\u{597d}"])?;
        enc.set_integer_array(11, &[1, 2, 3, 4, 5])?;
        enc.set_bool_array(12, &[false, true, false])?;
        enc.set_double_array(13, &[0.01171875, 23.0, 4.0])?;
        Ok(())
    });
    let expected = testdata("full_encoded.bin");
    assert_eq!(
        hexdump(&encoded),
        hexdump(&expected),
        "direct encode full mismatch"
    );
}

// =============================================================================
// Binary fixture cross-validation: Decode C-generated .bin with StructDecoder
// =============================================================================

#[test]
fn test_direct_decode_simple_struct() {
    let sproto = load_sproto();
    let data = testdata("simple_struct_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    let name_field = fields.iter().find(|f| f.tag() == 0).unwrap();
    assert_eq!(name_field.as_string().unwrap(), "Alice");

    let age_field = fields.iter().find(|f| f.tag() == 1).unwrap();
    assert_eq!(age_field.as_integer().unwrap(), 13);

    let active_field = fields.iter().find(|f| f.tag() == 2).unwrap();
    assert!(!active_field.as_bool().unwrap());
}

#[test]
fn test_direct_decode_all_scalars() {
    let sproto = load_sproto();
    let data = testdata("all_scalars_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    assert_eq!(
        fields
            .iter()
            .find(|f| f.tag() == 0)
            .unwrap()
            .as_string()
            .unwrap(),
        "Alice"
    );
    assert_eq!(
        fields
            .iter()
            .find(|f| f.tag() == 1)
            .unwrap()
            .as_integer()
            .unwrap(),
        30
    );
    assert!(fields
        .iter()
        .find(|f| f.tag() == 2)
        .unwrap()
        .as_bool()
        .unwrap());
    assert_eq!(
        fields
            .iter()
            .find(|f| f.tag() == 3)
            .unwrap()
            .as_double()
            .unwrap(),
        0.01171875
    );
    assert_eq!(
        fields.iter().find(|f| f.tag() == 4).unwrap().as_bytes(),
        &[0x28, 0x29, 0x30, 0x31]
    );
    assert_eq!(
        fields
            .iter()
            .find(|f| f.tag() == 5)
            .unwrap()
            .as_integer()
            .unwrap(),
        182
    );
}

#[test]
fn test_direct_decode_nested_struct() {
    let sproto = load_sproto();
    let data = testdata("nested_struct_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();

    let mut name = None;
    let mut phone_number = None;
    let mut phone_type = None;

    while let Some(f) = dec.next_field().unwrap() {
        match f.tag() {
            0 => name = Some(f.as_string().unwrap().to_owned()),
            7 => {
                let mut sub = f.as_struct().unwrap();
                while let Some(sf) = sub.next_field().unwrap() {
                    match sf.tag() {
                        0 => phone_number = Some(sf.as_string().unwrap().to_owned()),
                        1 => phone_type = Some(sf.as_integer().unwrap()),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    assert_eq!(name.as_deref(), Some("Alice"));
    assert_eq!(phone_number.as_deref(), Some("123456789"));
    assert_eq!(phone_type, Some(1));
}

#[test]
fn test_direct_decode_struct_array() {
    let sproto = load_sproto();
    let data = testdata("struct_array_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();

    let mut name = None;
    let mut age = None;
    let mut children_names = Vec::new();
    let mut children_ages = Vec::new();

    while let Some(f) = dec.next_field().unwrap() {
        match f.tag() {
            0 => name = Some(f.as_string().unwrap().to_owned()),
            1 => age = Some(f.as_integer().unwrap()),
            9 => {
                for elem in f.as_struct_iter().unwrap() {
                    let mut sub = elem.unwrap();
                    while let Some(sf) = sub.next_field().unwrap() {
                        match sf.tag() {
                            0 => children_names.push(sf.as_string().unwrap().to_owned()),
                            1 => children_ages.push(sf.as_integer().unwrap()),
                            _ => {}
                        }
                    }
                }
            }
            _ => {}
        }
    }

    assert_eq!(name.as_deref(), Some("Bob"));
    assert_eq!(age, Some(40));
    assert_eq!(children_names, vec!["Alice", "Carol"]);
    assert_eq!(children_ages, vec![13, 5]);
}

#[test]
fn test_direct_decode_int_array() {
    let sproto = load_sproto();
    let data = testdata("int_array_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    let numbers_field = fields.iter().find(|f| f.tag() == 11).unwrap();
    assert_eq!(
        numbers_field.as_integer_array().unwrap(),
        vec![1, 2, 3, 4, 5]
    );
}

#[test]
fn test_direct_decode_big_int_array() {
    let sproto = load_sproto();
    let data = testdata("big_int_array_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    let base: i64 = 1 << 32;
    let numbers_field = fields.iter().find(|f| f.tag() == 11).unwrap();
    assert_eq!(
        numbers_field.as_integer_array().unwrap(),
        vec![base + 1, base + 2, base + 3]
    );
}

#[test]
fn test_direct_decode_bool_array() {
    let sproto = load_sproto();
    let data = testdata("bool_array_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    let flags_field = fields.iter().find(|f| f.tag() == 12).unwrap();
    assert_eq!(flags_field.as_bool_array(), vec![false, true, false]);
}

#[test]
fn test_direct_decode_number() {
    let sproto = load_sproto();
    let data = testdata("number_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    let age_field = fields.iter().find(|f| f.tag() == 1).unwrap();
    assert_eq!(age_field.as_integer().unwrap(), 100000);

    let id_field = fields.iter().find(|f| f.tag() == 6).unwrap();
    assert_eq!(id_field.as_integer().unwrap(), -10000000000);
}

#[test]
fn test_direct_decode_double() {
    let sproto = load_sproto();
    let data = testdata("double_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    let score_field = fields.iter().find(|f| f.tag() == 3).unwrap();
    assert_eq!(score_field.as_double().unwrap(), 0.01171875);

    let values_field = fields.iter().find(|f| f.tag() == 13).unwrap();
    assert_eq!(
        values_field.as_double_array().unwrap(),
        vec![0.01171875, 23.0, 4.0]
    );
}

#[test]
fn test_direct_decode_string_array() {
    let sproto = load_sproto();
    let data = testdata("string_array_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    let tags_field = fields.iter().find(|f| f.tag() == 10).unwrap();
    assert_eq!(
        tags_field.as_string_array().unwrap(),
        vec!["hello", "world", "\u{4f60}\u{597d}"]
    );
}

#[test]
fn test_direct_decode_fixed_point() {
    let sproto = load_sproto();
    let data = testdata("fixed_point_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();
    let fields = decode_fields(&mut dec);

    let fpn_field = fields.iter().find(|f| f.tag() == 5).unwrap();
    assert_eq!(fpn_field.as_integer().unwrap(), 182);
}

#[test]
fn test_direct_decode_full() {
    let sproto = load_sproto();
    let data = testdata("full_encoded.bin");
    let st = sproto.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&sproto, st, &data).unwrap();

    let mut name = None;
    let mut age = None;
    let mut active = None;
    let mut score = None;
    let mut photo = None;
    let mut fpn = None;
    let mut id = None;
    let mut phone_number = None;
    let mut phone_type = None;
    let mut phones = Vec::new();
    let mut children_names = Vec::new();
    let mut tags = Vec::new();
    let mut numbers = Vec::new();
    let mut flags = Vec::new();
    let mut values = Vec::new();

    while let Some(f) = dec.next_field().unwrap() {
        match f.tag() {
            0 => name = Some(f.as_string().unwrap().to_owned()),
            1 => age = Some(f.as_integer().unwrap()),
            2 => active = Some(f.as_bool().unwrap()),
            3 => score = Some(f.as_double().unwrap()),
            4 => photo = Some(f.as_bytes().to_owned()),
            5 => fpn = Some(f.as_integer().unwrap()),
            6 => id = Some(f.as_integer().unwrap()),
            7 => {
                let mut sub = f.as_struct().unwrap();
                while let Some(sf) = sub.next_field().unwrap() {
                    match sf.tag() {
                        0 => phone_number = Some(sf.as_string().unwrap().to_owned()),
                        1 => phone_type = Some(sf.as_integer().unwrap()),
                        _ => {}
                    }
                }
            }
            8 => {
                for elem in f.as_struct_iter().unwrap() {
                    let mut sub = elem.unwrap();
                    let mut pn = None;
                    let mut pt = None;
                    while let Some(sf) = sub.next_field().unwrap() {
                        match sf.tag() {
                            0 => pn = Some(sf.as_string().unwrap().to_owned()),
                            1 => pt = Some(sf.as_integer().unwrap()),
                            _ => {}
                        }
                    }
                    phones.push((pn.unwrap(), pt.unwrap()));
                }
            }
            9 => {
                for elem in f.as_struct_iter().unwrap() {
                    let mut sub = elem.unwrap();
                    while let Some(sf) = sub.next_field().unwrap() {
                        if sf.tag() == 0 {
                            children_names.push(sf.as_string().unwrap().to_owned());
                        }
                    }
                }
            }
            10 => {
                tags = f
                    .as_string_array()
                    .unwrap()
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            }
            11 => numbers = f.as_integer_array().unwrap(),
            12 => flags = f.as_bool_array(),
            13 => values = f.as_double_array().unwrap(),
            _ => {}
        }
    }

    assert_eq!(name.as_deref(), Some("Alice"));
    assert_eq!(age, Some(30));
    assert_eq!(active, Some(true));
    assert_eq!(score, Some(0.01171875));
    assert_eq!(photo.as_deref(), Some(&[0xDE, 0xAD, 0xBE, 0xEF][..]));
    assert_eq!(fpn, Some(182));
    assert_eq!(id, Some(10000));
    assert_eq!(phone_number.as_deref(), Some("123456789"));
    assert_eq!(phone_type, Some(1));
    assert_eq!(
        phones,
        vec![("123456789".to_owned(), 1), ("87654321".to_owned(), 2)]
    );
    assert_eq!(children_names, vec!["Bob"]);
    assert_eq!(tags, vec!["hello", "world", "\u{4f60}\u{597d}"]);
    assert_eq!(numbers, vec![1, 2, 3, 4, 5]);
    assert_eq!(flags, vec![false, true, false]);
    assert_eq!(values, vec![0.01171875, 23.0, 4.0]);
}

// =============================================================================
// Go reference inline byte tests
// =============================================================================

fn go_data_schema() -> sproto::Sproto {
    use sproto::types::{Field, FieldType};
    let mut s = sproto::Sproto::new();
    s.add_type(
        "Data",
        vec![
            Field::array("numbers", 0, FieldType::Integer),
            Field::array("bools", 1, FieldType::Boolean),
            Field::new("number", 2, FieldType::Integer),
            Field::new("bignumber", 3, FieldType::Integer),
            Field::new("double", 4, FieldType::Double),
            Field::array("doubles", 5, FieldType::Double),
            Field::array("strings", 7, FieldType::String),
            Field::new("bytes", 8, FieldType::Binary),
        ],
    );
    s
}

fn go_addressbook_schema() -> sproto::Sproto {
    use sproto::types::{Field, FieldType};
    let mut s = sproto::Sproto::new();
    let phone_idx = s.add_type(
        "PhoneNumber",
        vec![
            Field::new("number", 0, FieldType::String),
            Field::new("type", 1, FieldType::Integer),
        ],
    );
    let person_idx = s.add_type(
        "Person",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("id", 1, FieldType::Integer),
            Field::new("email", 2, FieldType::String),
            Field::array("phone", 3, FieldType::Struct(phone_idx)),
        ],
    );
    s.add_type(
        "AddressBook",
        vec![Field::array("person", 0, FieldType::Struct(person_idx))],
    );
    s
}

/// Bytes: Data{bytes:[0x28,0x29,0x30,0x31]}
const GO_BYTES_FIELD: &[u8] = &[
    0x02, 0x00, 0x0f, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x28, 0x29, 0x30, 0x31,
];

/// StringArray: Data{strings:["Bob","Alice","Carol"]}
const GO_STRING_ARRAY: &[u8] = &[
    0x02, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x19, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x42, 0x6F,
    0x62, 0x05, 0x00, 0x00, 0x00, 0x41, 0x6C, 0x69, 0x63, 0x65, 0x05, 0x00, 0x00, 0x00, 0x43, 0x61,
    0x72, 0x6F, 0x6C,
];

/// EmptyIntSlice: Data{numbers:[]}
const GO_EMPTY_INT_SLICE: &[u8] = &[0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// EmptyDoubleSlice: Data{doubles:[]}
const GO_EMPTY_DOUBLE_SLICE: &[u8] = &[0x02, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// AddressBook encoded (from Go abData)
const GO_ADDRESSBOOK: &[u8] = &[
    1, 0, 0, 0, 122, 0, 0, 0, 68, 0, 0, 0, 4, 0, 0, 0, 34, 78, 1, 0, 0, 0, 5, 0, 0, 0, 65, 108,
    105, 99, 101, 45, 0, 0, 0, 19, 0, 0, 0, 2, 0, 0, 0, 4, 0, 9, 0, 0, 0, 49, 50, 51, 52, 53, 54,
    55, 56, 57, 18, 0, 0, 0, 2, 0, 0, 0, 6, 0, 8, 0, 0, 0, 56, 55, 54, 53, 52, 51, 50, 49, 46, 0,
    0, 0, 4, 0, 0, 0, 66, 156, 1, 0, 0, 0, 3, 0, 0, 0, 66, 111, 98, 25, 0, 0, 0, 21, 0, 0, 0, 2, 0,
    0, 0, 8, 0, 11, 0, 0, 0, 48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 48,
];

#[test]
fn test_direct_encode_go_bytes_field() {
    let schema = go_data_schema();
    let encoded = direct_encode(&schema, "Data", |enc| {
        enc.set_bytes(8, &[0x28, 0x29, 0x30, 0x31])?;
        Ok(())
    });
    assert_eq!(hexdump(&encoded), hexdump(GO_BYTES_FIELD));
}

#[test]
fn test_direct_encode_go_string_array() {
    let schema = go_data_schema();
    let encoded = direct_encode(&schema, "Data", |enc| {
        enc.set_string_array(7, &["Bob", "Alice", "Carol"])?;
        Ok(())
    });
    assert_eq!(hexdump(&encoded), hexdump(GO_STRING_ARRAY));
}

#[test]
fn test_direct_encode_go_empty_int_slice() {
    let schema = go_data_schema();
    let encoded = direct_encode(&schema, "Data", |enc| {
        enc.set_integer_array(0, &[])?;
        Ok(())
    });
    assert_eq!(hexdump(&encoded), hexdump(GO_EMPTY_INT_SLICE));
}

#[test]
fn test_direct_encode_go_empty_double_slice() {
    let schema = go_data_schema();
    let encoded = direct_encode(&schema, "Data", |enc| {
        enc.set_double_array(5, &[])?;
        Ok(())
    });
    assert_eq!(hexdump(&encoded), hexdump(GO_EMPTY_DOUBLE_SLICE));
}

#[test]
fn test_direct_encode_go_addressbook() {
    let schema = go_addressbook_schema();
    let encoded = direct_encode(&schema, "AddressBook", |enc| {
        enc.encode_struct_array(0, |arr| {
            // Alice
            arr.encode_element(|e| {
                e.set_string(0, "Alice")?;
                e.set_integer(1, 10000)?;
                e.encode_struct_array(3, |phones| {
                    phones.encode_element(|p| {
                        p.set_string(0, "123456789")?;
                        p.set_integer(1, 1)?;
                        Ok(())
                    })?;
                    phones.encode_element(|p| {
                        p.set_string(0, "87654321")?;
                        p.set_integer(1, 2)?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            // Bob
            arr.encode_element(|e| {
                e.set_string(0, "Bob")?;
                e.set_integer(1, 20000)?;
                e.encode_struct_array(3, |phones| {
                    phones.encode_element(|p| {
                        p.set_string(0, "01234567890")?;
                        p.set_integer(1, 3)?;
                        Ok(())
                    })?;
                    Ok(())
                })?;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    });
    assert_eq!(hexdump(&encoded), hexdump(GO_ADDRESSBOOK));
}

#[test]
fn test_direct_decode_go_bytes_field() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let mut dec = StructDecoder::new(&schema, st, GO_BYTES_FIELD).unwrap();
    let fields = decode_fields(&mut dec);

    let bytes_field = fields.iter().find(|f| f.tag() == 8).unwrap();
    assert_eq!(bytes_field.as_bytes(), &[0x28, 0x29, 0x30, 0x31]);
}

#[test]
fn test_direct_decode_go_string_array() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let mut dec = StructDecoder::new(&schema, st, GO_STRING_ARRAY).unwrap();
    let fields = decode_fields(&mut dec);

    let strings_field = fields.iter().find(|f| f.tag() == 7).unwrap();
    assert_eq!(
        strings_field.as_string_array().unwrap(),
        vec!["Bob", "Alice", "Carol"]
    );
}

#[test]
fn test_direct_decode_go_empty_int_slice() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let mut dec = StructDecoder::new(&schema, st, GO_EMPTY_INT_SLICE).unwrap();
    let fields = decode_fields(&mut dec);

    let numbers_field = fields.iter().find(|f| f.tag() == 0).unwrap();
    assert_eq!(numbers_field.as_integer_array().unwrap(), Vec::<i64>::new());
}

#[test]
fn test_direct_decode_go_empty_double_slice() {
    let schema = go_data_schema();
    let st = schema.get_type("Data").unwrap();
    let mut dec = StructDecoder::new(&schema, st, GO_EMPTY_DOUBLE_SLICE).unwrap();
    let fields = decode_fields(&mut dec);

    let doubles_field = fields.iter().find(|f| f.tag() == 5).unwrap();
    assert_eq!(doubles_field.as_double_array().unwrap(), Vec::<f64>::new());
}

#[test]
fn test_direct_decode_go_addressbook() {
    let schema = go_addressbook_schema();
    let st = schema.get_type("AddressBook").unwrap();
    let mut dec = StructDecoder::new(&schema, st, GO_ADDRESSBOOK).unwrap();

    let f = dec.next_field().unwrap().unwrap();
    assert_eq!(f.tag(), 0); // person array

    let mut persons = Vec::new();
    for elem in f.as_struct_iter().unwrap() {
        let mut sub = elem.unwrap();
        let mut name = None;
        let mut id = None;
        let mut phones = Vec::new();
        while let Some(sf) = sub.next_field().unwrap() {
            match sf.tag() {
                0 => name = Some(sf.as_string().unwrap().to_owned()),
                1 => id = Some(sf.as_integer().unwrap()),
                3 => {
                    for phone_elem in sf.as_struct_iter().unwrap() {
                        let mut phone = phone_elem.unwrap();
                        let mut number = None;
                        let mut ptype = None;
                        while let Some(pf) = phone.next_field().unwrap() {
                            match pf.tag() {
                                0 => number = Some(pf.as_string().unwrap().to_owned()),
                                1 => ptype = Some(pf.as_integer().unwrap()),
                                _ => {}
                            }
                        }
                        phones.push((number.unwrap(), ptype.unwrap()));
                    }
                }
                _ => {}
            }
        }
        persons.push((name.unwrap(), id.unwrap(), phones));
    }

    assert_eq!(persons.len(), 2);
    assert_eq!(persons[0].0, "Alice");
    assert_eq!(persons[0].1, 10000);
    assert_eq!(
        persons[0].2,
        vec![("123456789".to_owned(), 1), ("87654321".to_owned(), 2)]
    );
    assert_eq!(persons[1].0, "Bob");
    assert_eq!(persons[1].1, 20000);
    assert_eq!(persons[1].2, vec![("01234567890".to_owned(), 3)]);
}

// =============================================================================
// Full roundtrip encode->decode with StructEncoder/StructDecoder
// =============================================================================

fn roundtrip_schema() -> sproto::Sproto {
    use sproto::types::{Field, FieldType};
    let mut s = sproto::Sproto::new();
    let person_idx = s.add_type(
        "Person",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("age", 1, FieldType::Integer),
            Field::new("active", 2, FieldType::Boolean),
            Field::new("score", 3, FieldType::Double),
            Field::new("data", 4, FieldType::Binary),
        ],
    );
    s.add_type(
        "Data",
        vec![
            Field::array("numbers", 0, FieldType::Integer),
            Field::array("names", 1, FieldType::String),
            Field::array("flags", 2, FieldType::Boolean),
            Field::array("values", 3, FieldType::Double),
        ],
    );
    s.add_type(
        "Team",
        vec![
            Field::new("name", 0, FieldType::String),
            Field::new("leader", 1, FieldType::Struct(person_idx)),
            Field::array("members", 2, FieldType::Struct(person_idx)),
        ],
    );
    s
}

#[test]
fn test_direct_roundtrip_all_scalar_types() {
    let schema = roundtrip_schema();
    let binary_data = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let encoded = direct_encode(&schema, "Person", |enc| {
        enc.set_string(0, "Alice")?;
        enc.set_integer(1, 30)?;
        enc.set_bool(2, true)?;
        enc.set_double(3, 98.5)?;
        enc.set_bytes(4, &binary_data)?;
        Ok(())
    });

    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    let fields = decode_fields(&mut dec);

    assert_eq!(fields.len(), 5);
    assert_eq!(fields[0].as_string().unwrap(), "Alice");
    assert_eq!(fields[1].as_integer().unwrap(), 30);
    assert!(fields[2].as_bool().unwrap());
    assert!((fields[3].as_double().unwrap() - 98.5).abs() < 1e-10);
    assert_eq!(fields[4].as_bytes(), &[0xDE, 0xAD, 0xBE, 0xEF]);
}

#[test]
fn test_direct_roundtrip_all_array_types() {
    let schema = roundtrip_schema();
    let encoded = direct_encode(&schema, "Data", |enc| {
        enc.set_integer_array(0, &[10, -20, 30, i64::MAX, i64::MIN])?;
        enc.set_string_array(1, &["hello", "世界", "🌍"])?;
        enc.set_bool_array(2, &[true, false, true, false, true])?;
        enc.set_double_array(3, &[1.1, -2.2, 0.0, f64::MAX])?;
        Ok(())
    });

    let st = schema.get_type("Data").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    let fields = decode_fields(&mut dec);

    assert_eq!(fields.len(), 4);
    assert_eq!(
        fields[0].as_integer_array().unwrap(),
        vec![10, -20, 30, i64::MAX, i64::MIN]
    );
    assert_eq!(
        fields[1].as_string_array().unwrap(),
        vec!["hello", "世界", "🌍"]
    );
    assert_eq!(
        fields[2].as_bool_array(),
        vec![true, false, true, false, true]
    );
    let doubles = fields[3].as_double_array().unwrap();
    assert!((doubles[0] - 1.1).abs() < 1e-10);
    assert!((doubles[1] - (-2.2)).abs() < 1e-10);
    assert_eq!(doubles[2], 0.0);
    assert_eq!(doubles[3], f64::MAX);
}

#[test]
fn test_direct_roundtrip_nested_struct() {
    let schema = roundtrip_schema();
    let encoded = direct_encode(&schema, "Team", |enc| {
        enc.set_string(0, "Engineering")?;
        enc.encode_nested(1, |leader| {
            leader.set_string(0, "Alice")?;
            leader.set_integer(1, 35)?;
            leader.set_bool(2, true)?;
            leader.set_double(3, 99.9)?;
            Ok(())
        })?;
        enc.encode_struct_array(2, |members| {
            members.encode_element(|m| {
                m.set_string(0, "Bob")?;
                m.set_integer(1, 28)?;
                m.set_bool(2, true)?;
                Ok(())
            })?;
            members.encode_element(|m| {
                m.set_string(0, "Carol")?;
                m.set_integer(1, 32)?;
                m.set_bool(2, false)?;
                Ok(())
            })?;
            Ok(())
        })?;
        Ok(())
    });

    let st = schema.get_type("Team").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();

    // field 0: name
    let f0 = dec.next_field().unwrap().unwrap();
    assert_eq!(f0.as_string().unwrap(), "Engineering");

    // field 1: leader (nested struct)
    let f1 = dec.next_field().unwrap().unwrap();
    let mut leader_dec = f1.as_struct().unwrap();
    let leader_fields = decode_fields(&mut leader_dec);
    assert_eq!(leader_fields[0].as_string().unwrap(), "Alice");
    assert_eq!(leader_fields[1].as_integer().unwrap(), 35);
    assert!(leader_fields[2].as_bool().unwrap());
    assert!((leader_fields[3].as_double().unwrap() - 99.9).abs() < 1e-10);

    // field 2: members (struct array)
    let f2 = dec.next_field().unwrap().unwrap();
    let mut member_names = Vec::new();
    let mut member_ages = Vec::new();
    for elem in f2.as_struct_iter().unwrap() {
        let mut sub = elem.unwrap();
        while let Some(sf) = sub.next_field().unwrap() {
            match sf.tag() {
                0 => member_names.push(sf.as_string().unwrap().to_owned()),
                1 => member_ages.push(sf.as_integer().unwrap()),
                _ => {}
            }
        }
    }
    assert_eq!(member_names, vec!["Bob", "Carol"]);
    assert_eq!(member_ages, vec![28, 32]);

    // No more fields
    assert!(dec.next_field().unwrap().is_none());
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_direct_roundtrip_empty_string() {
    let schema = roundtrip_schema();
    let encoded = direct_encode(&schema, "Person", |enc| {
        enc.set_string(0, "")?;
        Ok(())
    });
    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    let f = dec.next_field().unwrap().unwrap();
    assert_eq!(f.as_string().unwrap(), "");
}

#[test]
fn test_direct_roundtrip_unicode() {
    let schema = roundtrip_schema();
    let text = "你好世界 🎉 こんにちは";
    let encoded = direct_encode(&schema, "Person", |enc| {
        enc.set_string(0, text)?;
        Ok(())
    });
    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    let f = dec.next_field().unwrap().unwrap();
    assert_eq!(f.as_string().unwrap(), text);
}

#[test]
fn test_direct_roundtrip_zero_integer() {
    let schema = roundtrip_schema();
    let encoded = direct_encode(&schema, "Person", |enc| {
        enc.set_integer(1, 0)?;
        Ok(())
    });
    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    let f = dec.next_field().unwrap().unwrap();
    assert_eq!(f.as_integer().unwrap(), 0);
}

#[test]
fn test_direct_roundtrip_boundary_integers() {
    let schema = roundtrip_schema();
    let values = vec![
        0x7FFE_i64,            // just below inline threshold
        0x7FFF,                // at inline threshold (still inline)
        -1,                    // negative (data section, 4 bytes)
        i32::MAX as i64,       // max i32
        i32::MIN as i64,       // min i32
        (i32::MAX as i64) + 1, // exceeds i32 → 8 bytes
        i64::MAX,
        i64::MIN,
    ];
    for &v in &values {
        let encoded = direct_encode(&schema, "Person", |enc| {
            enc.set_integer(1, v)?;
            Ok(())
        });
        let st = schema.get_type("Person").unwrap();
        let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
        let f = dec.next_field().unwrap().unwrap();
        assert_eq!(
            f.as_integer().unwrap(),
            v,
            "integer roundtrip failed for {}",
            v
        );
    }
}

#[test]
fn test_direct_roundtrip_special_doubles() {
    let schema = roundtrip_schema();
    let values = vec![
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
        1e-300,
        1e300,
    ];
    for &v in &values {
        let encoded = direct_encode(&schema, "Person", |enc| {
            enc.set_double(3, v)?;
            Ok(())
        });
        let st = schema.get_type("Person").unwrap();
        let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
        let f = dec.next_field().unwrap().unwrap();
        assert_eq!(
            f.as_double().unwrap().to_bits(),
            v.to_bits(),
            "double roundtrip failed for {}",
            v
        );
    }
}

#[test]
fn test_direct_roundtrip_empty_arrays() {
    let schema = roundtrip_schema();
    let encoded = direct_encode(&schema, "Data", |enc| {
        enc.set_integer_array(0, &[])?;
        enc.set_string_array(1, &[] as &[&str])?;
        enc.set_bool_array(2, &[])?;
        enc.set_double_array(3, &[])?;
        Ok(())
    });
    let st = schema.get_type("Data").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    let fields = decode_fields(&mut dec);

    assert_eq!(fields.len(), 4);
    assert!(fields[0].as_integer_array().unwrap().is_empty());
    assert!(fields[1].as_string_array().unwrap().is_empty());
    assert!(fields[2].as_bool_array().is_empty());
    assert!(fields[3].as_double_array().unwrap().is_empty());
}

#[test]
fn test_direct_roundtrip_empty_binary() {
    let schema = roundtrip_schema();
    let encoded = direct_encode(&schema, "Person", |enc| {
        enc.set_bytes(4, &[])?;
        Ok(())
    });
    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    let f = dec.next_field().unwrap().unwrap();
    assert_eq!(f.as_bytes(), &[] as &[u8]);
}

#[test]
fn test_direct_roundtrip_skip_nil_fields() {
    let schema = roundtrip_schema();
    // Only set tag 1 (age) and tag 3 (score), skip 0 and 2
    let encoded = direct_encode(&schema, "Person", |enc| {
        enc.set_integer(1, 42)?;
        enc.set_double(3, 3.15)?;
        Ok(())
    });
    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    let fields = decode_fields(&mut dec);

    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].tag(), 1);
    assert_eq!(fields[0].as_integer().unwrap(), 42);
    assert_eq!(fields[1].tag(), 3);
    assert!((fields[1].as_double().unwrap() - 3.15).abs() < 1e-10);
}

#[test]
fn test_direct_encode_empty_struct() {
    let schema = roundtrip_schema();
    let encoded = direct_encode(&schema, "Person", |_enc| Ok(()));
    let st = schema.get_type("Person").unwrap();
    let mut dec = StructDecoder::new(&schema, st, &encoded).unwrap();
    assert!(dec.next_field().unwrap().is_none());
}

// =============================================================================
// Error handling
// =============================================================================

#[test]
fn test_direct_decode_truncated_data() {
    let schema = roundtrip_schema();
    let st = schema.get_type("Person").unwrap();
    // Only 1 byte — not even enough for the header
    let result = StructDecoder::new(&schema, st, &[0x01]);
    assert!(result.is_err());
}

#[test]
fn test_direct_decode_truncated_header() {
    let schema = roundtrip_schema();
    let st = schema.get_type("Person").unwrap();
    // Header says 5 fields but only 2 bytes of data
    let result = StructDecoder::new(&schema, st, &[0x05, 0x00, 0x00]);
    assert!(result.is_err());
}

#[test]
fn test_direct_encode_unknown_tag() {
    let schema = roundtrip_schema();
    let st = schema.get_type("Person").unwrap();
    let mut buf = Vec::new();
    let mut enc = StructEncoder::new(&schema, st, &mut buf);
    // Tag 99 doesn't exist in Person
    let result = enc.set_integer(99, 42);
    assert!(result.is_err());
}

// =============================================================================
// Encode->decode roundtrip matching binary fixtures
// =============================================================================

#[test]
fn test_direct_roundtrip_all_fixtures() {
    let sproto = load_sproto();

    let fixtures = [
        "simple_struct_encoded.bin",
        "all_scalars_encoded.bin",
        "nested_struct_encoded.bin",
        "struct_array_encoded.bin",
        "int_array_encoded.bin",
        "big_int_array_encoded.bin",
        "bool_array_encoded.bin",
        "number_encoded.bin",
        "double_encoded.bin",
        "string_array_encoded.bin",
        "fixed_point_encoded.bin",
        "full_encoded.bin",
    ];

    for file in &fixtures {
        let original = testdata(file);
        let st = sproto.get_type("Person").unwrap();

        // Re-encode from decoded fields
        let mut re_encoded = Vec::new();
        {
            let mut enc = StructEncoder::new(&sproto, st, &mut re_encoded);
            let mut dec = StructDecoder::new(&sproto, st, &original).unwrap();
            while let Some(f) = dec.next_field().unwrap() {
                let field = f.field();
                if field.is_array {
                    match &field.field_type {
                        sproto::types::FieldType::Struct(_) => {
                            enc.encode_struct_array(field.tag, |arr| {
                                for elem in f.as_struct_iter().unwrap() {
                                    arr.encode_element(|sub_enc| {
                                        let mut sub_dec = elem.unwrap();
                                        while let Some(sf) = sub_dec.next_field().unwrap() {
                                            reencode_scalar(sub_enc, &sf)?;
                                        }
                                        Ok(())
                                    })?;
                                }
                                Ok(())
                            })
                            .unwrap();
                        }
                        _ => {
                            reencode_scalar_array(&mut enc, &f).unwrap();
                        }
                    }
                } else {
                    match &field.field_type {
                        sproto::types::FieldType::Struct(_) => {
                            enc.encode_nested(field.tag, |sub_enc| {
                                let mut sub_dec = f.as_struct().unwrap();
                                while let Some(sf) = sub_dec.next_field().unwrap() {
                                    reencode_scalar(sub_enc, &sf)?;
                                }
                                Ok(())
                            })
                            .unwrap();
                        }
                        _ => {
                            reencode_scalar(&mut enc, &f).unwrap();
                        }
                    }
                }
            }
            enc.finish();
        }
        assert_eq!(
            hexdump(&re_encoded),
            hexdump(&original),
            "decode->re-encode roundtrip failed for {}",
            file
        );
    }
}

/// Re-encode a single scalar field from a DecodedField into a StructEncoder.
fn reencode_scalar(
    enc: &mut StructEncoder,
    f: &DecodedField,
) -> Result<(), sproto::error::EncodeError> {
    let field = f.field();
    match &field.field_type {
        sproto::types::FieldType::Integer => {
            enc.set_integer(
                field.tag,
                f.as_integer()
                    .map_err(|e| sproto::error::EncodeError::Other(e.to_string()))?,
            )?;
        }
        sproto::types::FieldType::Boolean => {
            enc.set_bool(
                field.tag,
                f.as_bool()
                    .map_err(|e| sproto::error::EncodeError::Other(e.to_string()))?,
            )?;
        }
        sproto::types::FieldType::Double => {
            enc.set_double(
                field.tag,
                f.as_double()
                    .map_err(|e| sproto::error::EncodeError::Other(e.to_string()))?,
            )?;
        }
        sproto::types::FieldType::String => {
            enc.set_string(
                field.tag,
                f.as_string()
                    .map_err(|e| sproto::error::EncodeError::Other(e.to_string()))?,
            )?;
        }
        sproto::types::FieldType::Binary => {
            enc.set_bytes(field.tag, f.as_bytes())?;
        }
        sproto::types::FieldType::Struct(_) => {
            unreachable!("reencode_scalar should not be called for struct fields");
        }
    }
    Ok(())
}

/// Re-encode a scalar array field from a DecodedField into a StructEncoder.
fn reencode_scalar_array(
    enc: &mut StructEncoder,
    f: &DecodedField,
) -> Result<(), sproto::error::EncodeError> {
    let field = f.field();
    match &field.field_type {
        sproto::types::FieldType::Integer => {
            let arr = f
                .as_integer_array()
                .map_err(|e| sproto::error::EncodeError::Other(e.to_string()))?;
            enc.set_integer_array(field.tag, &arr)?;
        }
        sproto::types::FieldType::Boolean => {
            let arr = f.as_bool_array();
            enc.set_bool_array(field.tag, &arr)?;
        }
        sproto::types::FieldType::Double => {
            let arr = f
                .as_double_array()
                .map_err(|e| sproto::error::EncodeError::Other(e.to_string()))?;
            enc.set_double_array(field.tag, &arr)?;
        }
        sproto::types::FieldType::String => {
            let arr = f
                .as_string_array()
                .map_err(|e| sproto::error::EncodeError::Other(e.to_string()))?;
            enc.set_string_array(field.tag, &arr)?;
        }
        sproto::types::FieldType::Binary => {
            let arr = f
                .as_bytes_array()
                .map_err(|e| sproto::error::EncodeError::Other(e.to_string()))?;
            enc.set_bytes_array(field.tag, &arr)?;
        }
        sproto::types::FieldType::Struct(_) => {
            unreachable!("reencode_scalar_array should not be called for struct arrays");
        }
    }
    Ok(())
}
