//! Cross-validation: load C-generated binary schemas and verify structure.

use sproto::binary_schema;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/testdata/{}", env!("CARGO_MANIFEST_DIR"), name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

#[test]
fn test_load_schema() {
    let data = testdata("schema.bin");
    let sproto = binary_schema::load_binary(&data).unwrap();

    // PhoneNumber type
    let phone = sproto
        .get_type("PhoneNumber")
        .expect("PhoneNumber type missing");
    assert_eq!(phone.fields.len(), 2);
    assert_eq!(&*phone.fields[0].name, "number");
    assert_eq!(phone.fields[0].tag, 0);
    assert_eq!(&*phone.fields[1].name, "type");
    assert_eq!(phone.fields[1].tag, 1);

    // Person type — 14 fields covering all sproto type mappings
    let person = sproto.get_type("Person").expect("Person type missing");
    assert_eq!(person.fields.len(), 14);

    let expected_fields = [
        ("name", 0, false),
        ("age", 1, false),
        ("active", 2, false),
        ("score", 3, false),
        ("photo", 4, false),
        ("fpn", 5, false),
        ("id", 6, false),
        ("phone", 7, false),
        ("phones", 8, true),
        ("children", 9, true),
        ("tags", 10, true),
        ("numbers", 11, true),
        ("flags", 12, true),
        ("values", 13, true),
    ];

    for (i, &(name, tag, is_array)) in expected_fields.iter().enumerate() {
        assert_eq!(&*person.fields[i].name, name, "field {} name mismatch", i);
        assert_eq!(person.fields[i].tag, tag, "field {} tag mismatch", i);
        assert_eq!(
            person.fields[i].is_array, is_array,
            "field {} is_array mismatch",
            i
        );
    }

    // fpn has decimal precision
    assert!(person.fields[5].decimal_precision > 0);
}

#[test]
fn test_load_rpc_schema() {
    let data = testdata("rpc_schema.bin");
    let sproto = binary_schema::load_binary(&data).unwrap();

    // Check package type
    let pkg = sproto.get_type("package").expect("package type missing");
    assert_eq!(pkg.fields.len(), 3); // type, session, ud

    // Check protocols
    let foobar = sproto
        .get_protocol("foobar")
        .expect("foobar protocol missing");
    assert_eq!(foobar.tag, 1);
    assert!(foobar.request.is_some());
    assert!(foobar.response.is_some());

    let foo = sproto.get_protocol("foo").expect("foo protocol missing");
    assert_eq!(foo.tag, 2);
    assert!(foo.request.is_none());
    assert!(foo.response.is_some());

    let bar = sproto.get_protocol("bar").expect("bar protocol missing");
    assert_eq!(bar.tag, 3);
    assert!(bar.confirm);

    let bh = sproto
        .get_protocol("blackhole")
        .expect("blackhole protocol missing");
    assert_eq!(bh.tag, 4);
    assert!(bh.request.is_none());
    assert!(bh.response.is_none());
}
