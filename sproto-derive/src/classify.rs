use syn::Type;

/// Classified wire type for a field.
#[derive(Clone)]
pub enum FieldKind {
    Integer,
    Bool,
    Double,
    Decimal(u32), // precision
    StringField,
    Binary,
    IntegerArray,
    BoolArray,
    DoubleArray,
    StringArray,
    BytesArray,
    StructArray(Type), // inner element type
    NestedStruct(Type),
    /// `*Type(key)` — indexed map: HashMap<K, Struct>, key is a field in the struct
    #[allow(dead_code)]
    IndexedMap {
        key_ty: Type,
        val_ty: Type,
    },
    /// `*Type()` — anonymous map: HashMap<K, V>, first field is key, second is value
    AnonymousMap {
        key_ty: Type,
        val_ty: Type,
    },
}

/// Wrapper tracking optional/vec status.
#[derive(Clone)]
pub struct ClassifiedField {
    pub kind: FieldKind,
    pub is_optional: bool,
    pub is_boxed: bool,
    pub key_tag: Option<u16>,
    pub value_tag: Option<u16>,
    pub key_field: Option<String>,
}

/// Classify a Rust type into a sproto wire category.
pub fn classify_type(
    ty: &Type,
    decimal: Option<u32>,
    key: Option<u16>,
    value: Option<u16>,
    key_field: Option<String>,
) -> ClassifiedField {
    if let Some(inner) = extract_option_inner(ty) {
        let mut result = classify_inner_type(inner, decimal, key, value);
        result.is_optional = true;
        result.key_field = key_field;
        return result;
    }
    let mut result = classify_inner_type(ty, decimal, key, value);
    result.key_field = key_field;
    result
}

fn classify_inner_type(
    ty: &Type,
    decimal: Option<u32>,
    key: Option<u16>,
    value: Option<u16>,
) -> ClassifiedField {
    // Check for Box<T>
    if let Some(inner) = extract_generic_inner(ty, "Box") {
        let mut result = classify_inner_type(inner, decimal, key, value);
        result.is_boxed = true;
        return result;
    }

    // Check for HashMap<K, V>
    if let Some((key_ty, val_ty)) = extract_hashmap_types(ty) {
        let kind = if value.is_some() {
            FieldKind::AnonymousMap {
                key_ty: key_ty.clone(),
                val_ty: val_ty.clone(),
            }
        } else {
            FieldKind::IndexedMap {
                key_ty: key_ty.clone(),
                val_ty: val_ty.clone(),
            }
        };
        return ClassifiedField {
            kind,
            is_optional: false,
            is_boxed: false,
            key_tag: key,
            value_tag: value,
            key_field: None,
        };
    }

    // Check for Vec<T>
    if let Some(inner) = extract_generic_inner(ty, "Vec") {
        let kind = classify_vec_element(inner);
        return ClassifiedField {
            kind,
            is_optional: false,
            is_boxed: false,
            key_tag: None,
            value_tag: None,
            key_field: None,
        };
    }

    // Scalar types
    let kind = if let Some(d) = decimal {
        FieldKind::Decimal(d)
    } else {
        classify_scalar(ty)
    };

    ClassifiedField {
        kind,
        is_optional: false,
        is_boxed: false,
        key_tag: None,
        value_tag: None,
        key_field: None,
    }
}

fn classify_scalar(ty: &Type) -> FieldKind {
    let name = type_name_str(ty);
    match name.as_str() {
        "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "u8" | "isize" | "usize" => {
            FieldKind::Integer
        }
        "bool" => FieldKind::Bool,
        "f64" | "f32" => FieldKind::Double,
        "String" => FieldKind::StringField,
        _ => FieldKind::NestedStruct(ty.clone()),
    }
}

fn classify_vec_element(inner: &Type) -> FieldKind {
    let name = type_name_str(inner);
    match name.as_str() {
        "u8" => FieldKind::Binary,
        "i64" | "i32" | "i16" | "i8" | "u64" | "u32" | "u16" | "usize" | "isize" => {
            FieldKind::IntegerArray
        }
        "bool" => FieldKind::BoolArray,
        "f64" | "f32" => FieldKind::DoubleArray,
        "String" => FieldKind::StringArray,
        _ => {
            // Check for Vec<Vec<u8>> (bytes array)
            if let Some(inner2) = extract_generic_inner(inner, "Vec") {
                let inner2_name = type_name_str(inner2);
                if inner2_name == "u8" {
                    return FieldKind::BytesArray;
                }
            }
            FieldKind::StructArray(inner.clone())
        }
    }
}

/// Classify a scalar type for use as a map key or value.
pub fn classify_scalar_kind(ty: &Type) -> FieldKind {
    classify_scalar(ty)
}

fn extract_option_inner(ty: &Type) -> Option<&Type> {
    extract_generic_inner(ty, "Option")
}

fn extract_hashmap_types(ty: &Type) -> Option<(&Type, &Type)> {
    if let Type::Path(type_path) = ty {
        let seg = type_path.path.segments.last()?;
        if seg.ident != "HashMap" {
            return None;
        }
        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
            let mut iter = args.args.iter();
            if let (
                Some(syn::GenericArgument::Type(key_ty)),
                Some(syn::GenericArgument::Type(val_ty)),
            ) = (iter.next(), iter.next())
            {
                return Some((key_ty, val_ty));
            }
        }
    }
    None
}

fn extract_generic_inner<'a>(ty: &'a Type, wrapper: &str) -> Option<&'a Type> {
    if let Type::Path(type_path) = ty {
        let seg = type_path.path.segments.last()?;
        if seg.ident != wrapper {
            return None;
        }
        if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
            if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                return Some(inner);
            }
        }
    }
    None
}

pub fn type_name_str(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    String::new()
}
