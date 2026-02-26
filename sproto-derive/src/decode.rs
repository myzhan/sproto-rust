//! Code generation for SprotoDecode derive macro.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, GenericArgument, PathArguments, Result, Type};

use crate::attr::{validate_fields, FieldAttrs, FieldInfo, StructAttrs};

/// Generate the SprotoDecode implementation for a struct.
pub fn derive_decode(input: &DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let _struct_attrs = StructAttrs::from_attrs(&input.attrs)?;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "SprotoDecode only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "SprotoDecode only supports structs",
            ))
        }
    };

    // Parse field attributes and collect field info with types
    let mut field_data: Vec<(FieldInfo, TokenStream)> = Vec::new();
    for field in fields {
        let ident = field.ident.clone().unwrap();
        let attrs = FieldAttrs::from_attrs(&field.attrs)?;

        if attrs.skip {
            field_data.push((
                FieldInfo {
                    ident,
                    tag: 0,
                    is_optional: false,
                    is_vec: false,
                    skip: true,
                    use_default: true,
                    span: field.ident.as_ref().unwrap().span(),
                },
                quote! { ::sproto::types::FieldType::Integer },
            ));
            continue;
        }

        let tag = attrs.tag.ok_or_else(|| {
            syn::Error::new_spanned(
                &field.ident,
                "field must have #[sproto(tag = N)] attribute",
            )
        })?;

        let (is_optional, is_vec, inner_type) = analyze_type(&field.ty);
        let field_type_tokens = rust_type_to_field_type(inner_type.unwrap_or(&field.ty));

        field_data.push((
            FieldInfo {
                ident,
                tag,
                is_optional,
                is_vec,
                skip: false,
                use_default: attrs.use_default,
                span: field.ident.as_ref().unwrap().span(),
            },
            field_type_tokens,
        ));
    }

    // Validate tags
    let field_infos: Vec<_> = field_data.iter().map(|(info, _)| info.clone()).collect();
    validate_fields(&field_infos)?;

    // Generate field extraction code - extract directly from SprotoValue
    let field_extractions = field_data.iter().map(|(field, _)| {
        let ident = &field.ident;
        let name_str = ident.to_string();

        if field.skip {
            quote! {
                let #ident = Default::default();
            }
        } else if field.is_optional {
            quote! {
                let #ident = fields.get(#name_str).map(|v| extract_field_value(v, #name_str)).transpose()?;
            }
        } else if field.use_default {
            quote! {
                let #ident = match fields.get(#name_str) {
                    Some(v) => extract_field_value(v, #name_str)?,
                    None => Default::default(),
                };
            }
        } else {
            quote! {
                let #ident = match fields.get(#name_str) {
                    Some(v) => extract_field_value(v, #name_str)?,
                    None => return Err(::sproto::error::DecodeError::InvalidData(
                        format!("missing required field '{}'", #name_str)
                    )),
                };
            }
        }
    });

    let field_names = field_data.iter().map(|(f, _)| &f.ident);

    // Generate schema field definitions sorted by tag
    let mut sorted_fields: Vec<_> = field_data.iter().filter(|(f, _)| !f.skip).collect();
    sorted_fields.sort_by_key(|(f, _)| f.tag);

    let schema_fields = sorted_fields.iter().map(|(field, field_type_tokens)| {
        let name_str = field.ident.to_string();
        let tag = field.tag;
        let is_array = field.is_vec;

        quote! {
            ::sproto::types::Field {
                name: #name_str.to_string(),
                tag: #tag,
                field_type: #field_type_tokens,
                is_array: #is_array,
                key_tag: -1,
                is_map: false,
                decimal_precision: 0,
            }
        }
    });

    let type_name = name.to_string();
    
    // Calculate maxn - must include skip markers for non-contiguous tags
    let maxn = if sorted_fields.is_empty() {
        0usize
    } else {
        let num_fields = sorted_fields.len();
        // Count gap regions
        let mut gap_count = 0usize;
        let mut prev_tag: i32 = -1;
        for (f, _) in &sorted_fields {
            if f.tag as i32 > prev_tag + 1 {
                gap_count += 1;
            }
            prev_tag = f.tag as i32;
        }
        num_fields + gap_count
    };

    // Calculate base_tag
    let base_tag_value = if sorted_fields.is_empty() {
        -1i32
    } else {
        let first_tag = sorted_fields.first().unwrap().0.tag as i32;
        let last_tag = sorted_fields.last().unwrap().0.tag as i32;
        let expected_count = (last_tag - first_tag + 1) as usize;
        if expected_count == sorted_fields.len() {
            first_tag
        } else {
            -1i32
        }
    };

    Ok(quote! {
        impl ::sproto::SprotoDecode for #name {
            fn sproto_decode(data: &[u8]) -> ::std::result::Result<Self, ::sproto::error::DecodeError> {
                use ::std::collections::HashMap;

                // Helper function to extract values from SprotoValue
                fn extract_field_value<T: ::std::convert::TryFrom<::sproto::SprotoValue>>(
                    value: &::sproto::SprotoValue,
                    _field_name: &str,
                ) -> ::std::result::Result<T, ::sproto::error::DecodeError>
                where
                    T::Error: ::std::fmt::Debug,
                {
                    T::try_from(value.clone()).map_err(|_| {
                        ::sproto::error::DecodeError::InvalidData(
                            format!("type conversion failed")
                        )
                    })
                }

                // Build inline schema with compile-time determined field types
                let schema_fields = vec![#(#schema_fields),*];
                let sproto_type = ::sproto::types::SprotoType {
                    name: #type_name.to_string(),
                    fields: schema_fields,
                    base_tag: #base_tag_value,
                    maxn: #maxn,
                };

                let sproto = ::sproto::Sproto {
                    types_list: vec![sproto_type],
                    types_by_name: {
                        let mut m = HashMap::new();
                        m.insert(#type_name.to_string(), 0);
                        m
                    },
                    protocols: vec![],
                    protocols_by_name: HashMap::new(),
                    protocols_by_tag: HashMap::new(),
                };

                // Decode bytes to SprotoValue
                let value = ::sproto::codec::decode(&sproto, &sproto.types_list[0], data)?;

                // Extract fields from SprotoValue
                let fields = match &value {
                    ::sproto::SprotoValue::Struct(map) => map,
                    _ => return Err(::sproto::error::DecodeError::InvalidData(
                        "expected struct".to_string()
                    )),
                };

                #(#field_extractions)*

                Ok(Self {
                    #(#field_names),*
                })
            }
        }
    })
}

/// Analyze a type to determine if it's Option<T> or Vec<T>, returning inner type.
fn analyze_type(ty: &Type) -> (bool, bool, Option<&Type>) {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            if ident == "Option" || ident == "Vec" {
                // Extract inner type
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return (ident == "Option", ident == "Vec", Some(inner));
                    }
                }
                return (ident == "Option", ident == "Vec", None);
            }
        }
    }
    (false, false, None)
}

/// Convert Rust type to sproto FieldType TokenStream.
fn rust_type_to_field_type(ty: &Type) -> TokenStream {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            return match ident.as_str() {
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" => {
                    quote! { ::sproto::types::FieldType::Integer }
                }
                "bool" => quote! { ::sproto::types::FieldType::Boolean },
                "f32" | "f64" => quote! { ::sproto::types::FieldType::Double },
                "String" | "str" => quote! { ::sproto::types::FieldType::String },
                "Vec" => {
                    // Check if it's Vec<u8> (binary)
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner)) = args.args.first() {
                            if let Type::Path(inner_path) = inner {
                                if let Some(inner_seg) = inner_path.path.segments.last() {
                                    if inner_seg.ident == "u8" {
                                        return quote! { ::sproto::types::FieldType::Binary };
                                    }
                                }
                            }
                            // Otherwise it's an array of the inner type
                            return rust_type_to_field_type(inner);
                        }
                    }
                    quote! { ::sproto::types::FieldType::Integer }
                }
                _ => quote! { ::sproto::types::FieldType::String },
            };
        }
    }
    quote! { ::sproto::types::FieldType::String }
}
