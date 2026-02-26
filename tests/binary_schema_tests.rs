//! Cross-validation: load C-generated binary schemas and verify structure.

use sproto::binary_schema;

fn testdata(name: &str) -> Vec<u8> {
    let path = format!(
        "{}/testdata/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    std::fs::read(&path).unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e))
}

#[test]
fn test_load_person_data_schema() {
    let data = testdata("person_data_schema.bin");
    let sproto = binary_schema::load_binary(&data).unwrap();

    // Should have Data and Person types
    let person = sproto.get_type("Person").expect("Person type missing");
    assert_eq!(person.fields.len(), 4);
    assert_eq!(person.fields[0].name, "name");
    assert_eq!(person.fields[0].tag, 0);
    assert_eq!(person.fields[1].name, "age");
    assert_eq!(person.fields[1].tag, 1);
    assert_eq!(person.fields[2].name, "marital");
    assert_eq!(person.fields[2].tag, 2);
    assert_eq!(person.fields[3].name, "children");
    assert_eq!(person.fields[3].tag, 3);
    assert!(person.fields[3].is_array);

    let data_type = sproto.get_type("Data").expect("Data type missing");
    assert_eq!(data_type.fields.len(), 7);
    assert_eq!(data_type.fields[0].name, "numbers");
    assert!(data_type.fields[0].is_array);
    assert_eq!(data_type.fields[6].name, "fpn");
    assert!(data_type.fields[6].decimal_precision > 0);
}

#[test]
fn test_load_addressbook_schema() {
    let data = testdata("addressbook_schema.bin");
    let sproto = binary_schema::load_binary(&data).unwrap();

    let ab = sproto.get_type("AddressBook").expect("AddressBook type missing");
    assert_eq!(ab.fields.len(), 2);

    // person field should be array with key
    let person_field = &ab.fields[0];
    assert_eq!(person_field.name, "person");
    assert!(person_field.is_array);
    assert!(person_field.key_tag >= 0); // has a key (id field tag)

    // others field should be plain array
    let others_field = &ab.fields[1];
    assert_eq!(others_field.name, "others");
    assert!(others_field.is_array);

    // Person type should exist
    assert!(sproto.get_type("Person").is_some());
    // Nested PhoneNumber type
    assert!(sproto.get_type("Person.PhoneNumber").is_some());
}

#[test]
fn test_load_rpc_schema() {
    let data = testdata("rpc_schema.bin");
    let sproto = binary_schema::load_binary(&data).unwrap();

    // Check package type
    let pkg = sproto.get_type("package").expect("package type missing");
    assert_eq!(pkg.fields.len(), 3); // type, session, ud

    // Check protocols
    let foobar = sproto.get_protocol("foobar").expect("foobar protocol missing");
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

    let bh = sproto.get_protocol("blackhole").expect("blackhole protocol missing");
    assert_eq!(bh.tag, 4);
    assert!(bh.request.is_none());
    assert!(bh.response.is_none());
}
