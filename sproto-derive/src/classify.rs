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
}

/// Wrapper tracking optional/vec status.
#[derive(Clone)]
pub struct ClassifiedField {
    pub kind: FieldKind,
    pub is_optional: bool,
    pub is_boxed: bool,
}

/// Classify a Rust type into a sproto wire category.
pub fn classify_type(ty: &Type, decimal: Option<u32>) -> ClassifiedField {
    // Check for Option<T>
    if let Some(inner) = extract_option_inner(ty) {
        let mut result = classify_inner_type(inner, decimal);
        result.is_optional = true;
        return result;
    }
    classify_inner_type(ty, decimal)
}

fn classify_inner_type(ty: &Type, decimal: Option<u32>) -> ClassifiedField {
    // Check for Box<T>
    if let Some(inner) = extract_generic_inner(ty, "Box") {
        let mut result = classify_inner_type(inner, decimal);
        result.is_boxed = true;
        return result;
    }

    // Check for Vec<T>
    if let Some(inner) = extract_generic_inner(ty, "Vec") {
        let kind = classify_vec_element(inner);
        return ClassifiedField {
            kind,
            is_optional: false,
            is_boxed: false,
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

fn extract_option_inner(ty: &Type) -> Option<&Type> {
    extract_generic_inner(ty, "Option")
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

fn type_name_str(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        if let Some(seg) = type_path.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    String::new()
}
