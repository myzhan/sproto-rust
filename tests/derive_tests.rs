//! Tests for sproto derive macros.

use sproto::{SprotoDecode, SprotoEncode};

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct Person {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    age: i64,
    #[sproto(tag = 2)]
    active: bool,
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct Data {
    #[sproto(tag = 0)]
    numbers: Vec<i64>,
    #[sproto(tag = 1)]
    value: f64,
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct OptionalFields {
    #[sproto(tag = 0)]
    required: String,
    #[sproto(tag = 1)]
    optional: Option<i64>,
}

#[test]
fn test_derive_encode_decode_primitives() {
    let person = Person {
        name: "Alice".into(),
        age: 30,
        active: true,
    };

    let bytes = person.sproto_encode().unwrap();
    assert!(!bytes.is_empty());

    let decoded = Person::sproto_decode(&bytes).unwrap();
    assert_eq!(person, decoded);
}

#[test]
fn test_derive_encode_decode_arrays() {
    let data = Data {
        numbers: vec![1, 2, 3, 4, 5],
        value: 3.15,
    };

    let bytes = data.sproto_encode().unwrap();
    let decoded = Data::sproto_decode(&bytes).unwrap();
    assert_eq!(data, decoded);
}

#[test]
fn test_derive_optional_some() {
    let obj = OptionalFields {
        required: "test".into(),
        optional: Some(42),
    };

    let bytes = obj.sproto_encode().unwrap();
    let decoded = OptionalFields::sproto_decode(&bytes).unwrap();
    assert_eq!(obj, decoded);
}

#[test]
fn test_derive_optional_none() {
    let obj = OptionalFields {
        required: "test".into(),
        optional: None,
    };

    let bytes = obj.sproto_encode().unwrap();
    let decoded = OptionalFields::sproto_decode(&bytes).unwrap();
    assert_eq!(obj, decoded);
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct NonContiguousTags {
    #[sproto(tag = 0)]
    first: i64,
    #[sproto(tag = 5)]
    second: i64,
    #[sproto(tag = 10)]
    third: i64,
}

#[test]
fn test_derive_non_contiguous_tags() {
    let obj = NonContiguousTags {
        first: 1,
        second: 2,
        third: 3,
    };

    let bytes = obj.sproto_encode().unwrap();
    let decoded = NonContiguousTags::sproto_decode(&bytes).unwrap();
    assert_eq!(obj, decoded);
}

// ============================================================================
// Nested struct and Vec<Struct> tests
// ============================================================================

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct PhoneNumber {
    #[sproto(tag = 0)]
    number: String,
    #[sproto(tag = 1)]
    phone_type: i64,
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct Contact {
    #[sproto(tag = 0)]
    name: String,
    #[sproto(tag = 1)]
    id: i64,
    #[sproto(tag = 2)]
    phones: Vec<PhoneNumber>,
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct AddressBook {
    #[sproto(tag = 0)]
    contacts: Vec<Contact>,
}

#[test]
fn test_derive_nested_struct() {
    let contact = Contact {
        name: "Alice".into(),
        id: 10000,
        phones: vec![
            PhoneNumber {
                number: "123456789".into(),
                phone_type: 1,
            },
            PhoneNumber {
                number: "987654321".into(),
                phone_type: 2,
            },
        ],
    };

    let bytes = contact.sproto_encode().unwrap();
    let decoded = Contact::sproto_decode(&bytes).unwrap();
    assert_eq!(contact, decoded);
}

#[test]
fn test_derive_nested_struct_array() {
    let book = AddressBook {
        contacts: vec![
            Contact {
                name: "Alice".into(),
                id: 10000,
                phones: vec![
                    PhoneNumber {
                        number: "111".into(),
                        phone_type: 1,
                    },
                    PhoneNumber {
                        number: "222".into(),
                        phone_type: 2,
                    },
                ],
            },
            Contact {
                name: "Bob".into(),
                id: 20000,
                phones: vec![PhoneNumber {
                    number: "333".into(),
                    phone_type: 3,
                }],
            },
        ],
    };

    let bytes = book.sproto_encode().unwrap();
    let decoded = AddressBook::sproto_decode(&bytes).unwrap();
    assert_eq!(book, decoded);
}

#[test]
fn test_derive_nested_empty_array() {
    let contact = Contact {
        name: "Empty".into(),
        id: 1,
        phones: vec![],
    };

    let bytes = contact.sproto_encode().unwrap();
    let decoded = Contact::sproto_decode(&bytes).unwrap();
    assert_eq!(contact, decoded);
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct OptionalNested {
    #[sproto(tag = 0)]
    label: String,
    #[sproto(tag = 1)]
    inner: Option<PhoneNumber>,
}

#[test]
fn test_derive_optional_struct_some() {
    let obj = OptionalNested {
        label: "test".into(),
        inner: Some(PhoneNumber {
            number: "555".into(),
            phone_type: 1,
        }),
    };

    let bytes = obj.sproto_encode().unwrap();
    let decoded = OptionalNested::sproto_decode(&bytes).unwrap();
    assert_eq!(obj, decoded);
}

#[test]
fn test_derive_optional_struct_none() {
    let obj = OptionalNested {
        label: "test".into(),
        inner: None,
    };

    let bytes = obj.sproto_encode().unwrap();
    let decoded = OptionalNested::sproto_decode(&bytes).unwrap();
    assert_eq!(obj, decoded);
}

#[derive(Debug, PartialEq, SprotoEncode, SprotoDecode)]
struct TreeNode {
    #[sproto(tag = 0)]
    value: i64,
    #[sproto(tag = 1)]
    children: Vec<TreeNode>,
}

#[test]
fn test_derive_recursive_struct() {
    let tree = TreeNode {
        value: 1,
        children: vec![
            TreeNode {
                value: 2,
                children: vec![
                    TreeNode {
                        value: 4,
                        children: vec![],
                    },
                    TreeNode {
                        value: 5,
                        children: vec![],
                    },
                ],
            },
            TreeNode {
                value: 3,
                children: vec![],
            },
        ],
    };

    let bytes = tree.sproto_encode().unwrap();
    let decoded = TreeNode::sproto_decode(&bytes).unwrap();
    assert_eq!(tree, decoded);
}
