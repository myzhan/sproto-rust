use std::collections::HashMap;

use crate::error::ParseError;
use crate::types::*;
use super::ast::*;

/// Built-in type name to ID mapping.
fn _builtin_type_id(name: &str) -> Option<u8> {
    match name {
        "integer" => Some(0),
        "boolean" => Some(1),
        "string" => Some(2),
        "binary" => Some(2), // binary is a sub-type of string
        "double" => Some(3),
        _ => None,
    }
}

fn _is_builtin(name: &str) -> bool {
    _builtin_type_id(name).is_some()
}

/// Build a `Sproto` from parsed AST items.
pub fn build_schema(items: Vec<AstItem>) -> Result<Sproto, ParseError> {
    // Phase 1: Collect all type and protocol definitions, flattening nested types.
    let mut raw_types: HashMap<String, Vec<RawField>> = HashMap::new();
    let mut raw_protocols: HashMap<String, RawProtocol> = HashMap::new();

    for item in &items {
        match item {
            AstItem::Type(t) => {
                collect_type("", t, &mut raw_types)?;
            }
            AstItem::Protocol(p) => {
                if raw_protocols.contains_key(&p.name) {
                    return Err(ParseError::DuplicateType {
                        name: p.name.clone(),
                    });
                }
                // Handle inline struct definitions in protocols
                let request = match &p.request {
                    Some(AstProtoType::TypeName(n)) => Some(n.clone()),
                    Some(AstProtoType::InlineStruct(members)) => {
                        let inline_name = format!("{}.request", p.name);
                        let inline_type = AstType {
                            name: "request".into(),
                            members: members.clone(),
                            line: p.line,
                        };
                        collect_type(&p.name, &inline_type, &mut raw_types)?;
                        Some(inline_name)
                    }
                    Some(AstProtoType::Nil) | None => None,
                };
                let response = match &p.response {
                    Some(AstProtoType::TypeName(n)) => Some(n.clone()),
                    Some(AstProtoType::InlineStruct(members)) => {
                        let inline_name = format!("{}.response", p.name);
                        let inline_type = AstType {
                            name: "response".into(),
                            members: members.clone(),
                            line: p.line,
                        };
                        collect_type(&p.name, &inline_type, &mut raw_types)?;
                        Some(inline_name)
                    }
                    Some(AstProtoType::Nil) => None,
                    None => None,
                };
                let confirm = matches!(&p.response, Some(AstProtoType::Nil));

                raw_protocols.insert(
                    p.name.clone(),
                    RawProtocol {
                        name: p.name.clone(),
                        tag: p.tag as u16,
                        request,
                        response,
                        confirm,
                    },
                );
            }
        }
    }

    // Phase 2: Sort type names alphabetically (matching Lua parser behavior for stable output).
    let mut sorted_type_names: Vec<String> = raw_types.keys().cloned().collect();
    sorted_type_names.sort();

    // Build name -> index mapping
    let mut types_by_name: HashMap<String, usize> = HashMap::new();
    for (idx, name) in sorted_type_names.iter().enumerate() {
        types_by_name.insert(name.clone(), idx);
    }

    // Phase 3: Resolve type references and build SprotoType list.
    let mut types_list: Vec<SprotoType> = Vec::new();

    for type_name in &sorted_type_names {
        let raw_fields = &raw_types[type_name];
        let mut fields: Vec<Field> = Vec::new();

        for rf in raw_fields {
            let field_type = resolve_field_type(
                type_name,
                &rf.type_name,
                &types_by_name,
                &raw_types,
            )?;

            let mut decimal_precision: u32 = 0;
            let mut key_tag: i32 = -1;
            let mut is_map = false;

            if let Some(extra) = &rf.extra {
                if rf.type_name == "integer" && !rf.is_array {
                    // integer(N) -> decimal precision = 10^N
                    let n: u32 = extra.parse().map_err(|_| ParseError::Syntax {
                        line: rf.line,
                        message: format!("invalid decimal precision '{}'", extra),
                    })?;
                    decimal_precision = 10u32.pow(n);
                } else if rf.is_array {
                    // *Type(key) or *Type()
                    if extra.is_empty() {
                        // *Type() -> map mode with first field as key
                        is_map = true;
                        // key_tag will be resolved after we know the subtype's fields
                        if let FieldType::Struct(sub_idx) = &field_type {
                            let sub_fields = &raw_types[&sorted_type_names[*sub_idx]];
                            if sub_fields.len() != 2 {
                                return Err(ParseError::InvalidMapKey {
                                    type_name: type_name.clone(),
                                    field_name: rf.name.clone(),
                                });
                            }
                            // Find the field with the smallest tag
                            let min_field = sub_fields.iter().min_by_key(|f| f.tag).unwrap();
                            key_tag = min_field.tag as i32;
                        }
                    } else {
                        // *Type(key_name) -> find the key field's tag in the subtype
                        if let FieldType::Struct(sub_idx) = &field_type {
                            let sub_fields = &raw_types[&sorted_type_names[*sub_idx]];
                            let key_field = sub_fields.iter().find(|f| f.name == *extra);
                            match key_field {
                                Some(kf) => {
                                    key_tag = kf.tag as i32;
                                }
                                None => {
                                    return Err(ParseError::InvalidMapKey {
                                        type_name: type_name.clone(),
                                        field_name: rf.name.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            fields.push(Field {
                name: rf.name.clone(),
                tag: rf.tag as u16,
                field_type,
                is_array: rf.is_array,
                key_tag,
                is_map,
                decimal_precision,
            });
        }

        // Sort fields by tag
        fields.sort_by_key(|f| f.tag);

        // Compute base_tag and maxn (matching C logic)
        let (base_tag, maxn) = compute_base_tag_and_maxn(&fields);

        types_list.push(SprotoType {
            name: type_name.clone(),
            fields,
            base_tag,
            maxn,
        });
    }

    // Phase 4: Build protocols.
    let mut protocols: Vec<Protocol> = Vec::new();
    let mut protocols_by_name: HashMap<String, usize> = HashMap::new();
    let mut protocols_by_tag: HashMap<u16, usize> = HashMap::new();

    // Collect and sort by tag
    let mut proto_list: Vec<&RawProtocol> = raw_protocols.values().collect();
    proto_list.sort_by_key(|p| p.tag);

    // Check for duplicate tags
    for i in 1..proto_list.len() {
        if proto_list[i].tag == proto_list[i - 1].tag {
            return Err(ParseError::DuplicateProtocolTag {
                tag: proto_list[i].tag,
                name: proto_list[i].name.clone(),
            });
        }
    }

    for rp in proto_list {
        let request = rp
            .request
            .as_ref()
            .map(|n| {
                types_by_name.get(n).copied().ok_or_else(|| {
                    ParseError::UndefinedType {
                        type_name: n.clone(),
                        referenced_by: format!("protocol {}", rp.name),
                    }
                })
            })
            .transpose()?;

        let response = rp
            .response
            .as_ref()
            .map(|n| {
                types_by_name.get(n).copied().ok_or_else(|| {
                    ParseError::UndefinedType {
                        type_name: n.clone(),
                        referenced_by: format!("protocol {}", rp.name),
                    }
                })
            })
            .transpose()?;

        let idx = protocols.len();
        protocols_by_name.insert(rp.name.clone(), idx);
        protocols_by_tag.insert(rp.tag, idx);
        protocols.push(Protocol {
            name: rp.name.clone(),
            tag: rp.tag,
            request,
            response,
            confirm: rp.confirm,
        });
    }

    Ok(Sproto {
        types_list,
        types_by_name,
        protocols,
        protocols_by_name,
        protocols_by_tag,
    })
}

// Internal helper types

struct RawField {
    name: String,
    tag: u64,
    is_array: bool,
    type_name: String,
    extra: Option<String>,
    line: usize,
}

struct RawProtocol {
    name: String,
    tag: u16,
    request: Option<String>,
    response: Option<String>,
    confirm: bool,
}

/// Recursively collect type definitions, flattening nested types with dot-separated names.
fn collect_type(
    parent: &str,
    ast_type: &AstType,
    out: &mut HashMap<String, Vec<RawField>>,
) -> Result<(), ParseError> {
    let full_name = if parent.is_empty() {
        ast_type.name.clone()
    } else {
        format!("{}.{}", parent, ast_type.name)
    };

    if out.contains_key(&full_name) {
        return Err(ParseError::DuplicateType {
            name: full_name,
        });
    }

    let mut fields = Vec::new();
    let mut tag_set: HashMap<u64, String> = HashMap::new();
    let mut name_set: HashMap<String, u64> = HashMap::new();

    for member in &ast_type.members {
        match member {
            AstMember::Field(f) => {
                if let Some(_prev) = tag_set.get(&f.tag) {
                    return Err(ParseError::DuplicateTag {
                        type_name: full_name,
                        tag: f.tag as u16,
                    });
                }
                if name_set.contains_key(&f.name) {
                    return Err(ParseError::DuplicateField {
                        type_name: full_name,
                        field_name: f.name.clone(),
                    });
                }
                tag_set.insert(f.tag, f.name.clone());
                name_set.insert(f.name.clone(), f.tag);

                fields.push(RawField {
                    name: f.name.clone(),
                    tag: f.tag,
                    is_array: f.is_array,
                    type_name: f.type_name.clone(),
                    extra: f.extra.clone(),
                    line: f.line,
                });
            }
            AstMember::NestedType(nt) => {
                collect_type(&full_name, nt, out)?;
            }
        }
    }

    // Sort fields by tag
    fields.sort_by_key(|f| f.tag);

    out.insert(full_name, fields);
    Ok(())
}

/// Resolve a field's type name to a FieldType.
fn resolve_field_type(
    parent_type: &str,
    type_name: &str,
    types_by_name: &HashMap<String, usize>,
    _raw_types: &HashMap<String, Vec<RawField>>,
) -> Result<FieldType, ParseError> {
    match type_name {
        "integer" => Ok(FieldType::Integer),
        "boolean" => Ok(FieldType::Boolean),
        "string" => Ok(FieldType::String),
        "binary" => Ok(FieldType::Binary),
        "double" => Ok(FieldType::Double),
        _ => {
            // Try nested type first: parent.typename
            let full_name = format!("{}.{}", parent_type, type_name);
            if let Some(&idx) = types_by_name.get(&full_name) {
                return Ok(FieldType::Struct(idx));
            }
            // Try searching up the parent chain
            let mut prefix = parent_type.to_string();
            loop {
                let candidate = format!("{}.{}", prefix, type_name);
                if let Some(&idx) = types_by_name.get(&candidate) {
                    return Ok(FieldType::Struct(idx));
                }
                // Go up one level
                match prefix.rfind('.') {
                    Some(pos) => prefix = prefix[..pos].to_string(),
                    None => break,
                }
            }
            // Try as top-level type
            if let Some(&idx) = types_by_name.get(type_name) {
                return Ok(FieldType::Struct(idx));
            }
            Err(ParseError::UndefinedType {
                type_name: type_name.to_string(),
                referenced_by: parent_type.to_string(),
            })
        }
    }
}

/// Compute base_tag and maxn for a list of sorted fields.
/// Matches the C logic in import_type().
fn compute_base_tag_and_maxn(fields: &[Field]) -> (i32, usize) {
    if fields.is_empty() {
        return (-1, 0);
    }

    let n = fields.len();
    let mut maxn = n;
    let mut last: i32 = -1;

    for f in fields {
        let tag = f.tag as i32;
        if tag > last + 1 {
            maxn += 1;
        }
        last = tag;
    }

    let base = fields[0].tag as i32;
    let span = fields[n - 1].tag as i32 - base + 1;
    let base_tag = if span as usize != n { -1 } else { base };

    (base_tag, maxn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::grammar::parse_schema;

    #[test]
    fn test_build_simple_schema() {
        let items = parse_schema(
            ".Person { name 0 : string  age 1 : integer  marital 2 : boolean }",
        )
        .unwrap();
        let sproto = build_schema(items).unwrap();
        let t = sproto.get_type("Person").unwrap();
        assert_eq!(t.fields.len(), 3);
        assert_eq!(t.fields[0].name, "name");
        assert_eq!(t.fields[0].tag, 0);
        assert_eq!(t.fields[1].name, "age");
        assert_eq!(t.fields[2].name, "marital");
        assert_eq!(t.base_tag, 0);
    }

    #[test]
    fn test_build_nested_type() {
        let items = parse_schema(
            r#"
            .Person {
                name 0 : string
                .PhoneNumber {
                    number 0 : string
                    type 1 : integer
                }
                phone 1 : *PhoneNumber
            }
            "#,
        )
        .unwrap();
        let sproto = build_schema(items).unwrap();
        assert!(sproto.get_type("Person").is_some());
        assert!(sproto.get_type("Person.PhoneNumber").is_some());
        let person = sproto.get_type("Person").unwrap();
        let phone_field = person.find_field_by_name("phone").unwrap();
        assert!(phone_field.is_array);
        assert!(matches!(phone_field.field_type, FieldType::Struct(_)));
    }

    #[test]
    fn test_build_protocol() {
        let items = parse_schema(
            r#"
            .Request { what 0 : string }
            .Response { ok 0 : boolean }
            foobar 1 { request Request  response Response }
            "#,
        )
        .unwrap();
        let sproto = build_schema(items).unwrap();
        let proto = sproto.get_protocol("foobar").unwrap();
        assert_eq!(proto.tag, 1);
        assert!(proto.request.is_some());
        assert!(proto.response.is_some());
    }

    #[test]
    fn test_duplicate_tag_error() {
        let items = parse_schema(".T { a 0 : string  b 0 : integer }").unwrap();
        assert!(matches!(
            build_schema(items),
            Err(ParseError::DuplicateTag { .. })
        ));
    }

    #[test]
    fn test_decimal_precision() {
        let items = parse_schema(".Data { fpn 0 : integer(2) }").unwrap();
        let sproto = build_schema(items).unwrap();
        let t = sproto.get_type("Data").unwrap();
        assert_eq!(t.fields[0].decimal_precision, 100);
    }

    #[test]
    fn test_non_contiguous_tags() {
        let items = parse_schema(".T { a 0 : string  b 5 : integer }").unwrap();
        let sproto = build_schema(items).unwrap();
        let t = sproto.get_type("T").unwrap();
        assert_eq!(t.base_tag, -1); // Non-contiguous
        assert!(t.find_field_by_tag(0).is_some());
        assert!(t.find_field_by_tag(5).is_some());
        assert!(t.find_field_by_tag(3).is_none());
    }
}
