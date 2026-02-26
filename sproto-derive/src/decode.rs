//! Code generation for SprotoDecode derive macro.
//!
//! This module generates optimized inline decoding code that directly reads
//! from bytes without constructing intermediate SprotoValue or Schema objects.

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
    let mut field_data: Vec<(FieldInfo, FieldTypeInfo, &Type)> = Vec::new();
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
                FieldTypeInfo::Integer,
                &field.ty,
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
        let field_type_info = rust_type_to_field_type_info(inner_type.unwrap_or(&field.ty));

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
            field_type_info,
            &field.ty,
        ));
    }

    // Validate tags
    let field_infos: Vec<_> = field_data.iter().map(|(info, _, _)| info.clone()).collect();
    validate_fields(&field_infos)?;

    // Sort active fields by tag
    let mut sorted_fields: Vec<_> = field_data.iter().filter(|(f, _, _)| !f.skip).collect();
    sorted_fields.sort_by_key(|(f, _, _)| f.tag);

    // Generate field variable declarations
    let field_declarations: Vec<_> = field_data
        .iter()
        .map(|(field, _, ty)| {
            let ident = &field.ident;
            if field.skip {
                quote! { let mut #ident: #ty = Default::default(); }
            } else if field.is_optional {
                quote! { let mut #ident: #ty = None; }
            } else if field.use_default {
                quote! { let mut #ident: #ty = Default::default(); }
            } else {
                quote! { let mut #ident: Option<#ty> = None; }
            }
        })
        .collect();

    // Generate match arms for each tag
    let match_arms: Vec<_> = sorted_fields
        .iter()
        .map(|(field, field_type, _ty)| {
            let tag = field.tag;
            let ident = &field.ident;

            let decode_logic = if field.is_vec {
                generate_array_decode(ident, field.is_optional, field.use_default, *field_type)
            } else {
                generate_scalar_decode(ident, field.is_optional, field.use_default, *field_type)
            };

            quote! {
                #tag => {
                    #decode_logic
                }
            }
        })
        .collect();

    // Generate final field extraction
    let field_extractions: Vec<_> = field_data
        .iter()
        .map(|(field, _, _)| {
            let ident = &field.ident;
            let name_str = ident.to_string();
            if field.skip || field.is_optional || field.use_default {
                quote! { #ident }
            } else {
                quote! {
                    #ident: #ident.ok_or_else(|| ::sproto::error::DecodeError::InvalidData(
                        format!("missing required field '{}'", #name_str)
                    ))?
                }
            }
        })
        .collect();

    Ok(quote! {
        impl ::sproto::SprotoDecode for #name {
            fn sproto_decode(data: &[u8]) -> ::std::result::Result<Self, ::sproto::error::DecodeError> {
                // Constants for wire format
                const SIZEOF_HEADER: usize = 2;
                const SIZEOF_FIELD: usize = 2;
                const SIZEOF_LENGTH: usize = 4;

                #[inline]
                fn read_u16_le(buf: &[u8]) -> u16 {
                    u16::from_le_bytes([buf[0], buf[1]])
                }

                #[inline]
                fn read_u32_le(buf: &[u8]) -> u32 {
                    u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]])
                }

                #[inline]
                fn read_u64_le(buf: &[u8]) -> u64 {
                    u64::from_le_bytes([buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], buf[6], buf[7]])
                }

                #[inline]
                fn expand64(v: u32) -> u64 {
                    let value = v as u64;
                    if value & 0x80000000 != 0 {
                        value | (!0u64 << 32)
                    } else {
                        value
                    }
                }

                let size = data.len();
                if size < SIZEOF_HEADER {
                    return Err(::sproto::error::DecodeError::Truncated {
                        need: SIZEOF_HEADER,
                        have: size,
                    });
                }

                let fn_count = read_u16_le(&data[0..]) as usize;
                let field_part_end = SIZEOF_HEADER + fn_count * SIZEOF_FIELD;
                if size < field_part_end {
                    return Err(::sproto::error::DecodeError::Truncated {
                        need: field_part_end,
                        have: size,
                    });
                }

                let field_part = &data[SIZEOF_HEADER..field_part_end];
                let mut data_offset = field_part_end;
                let mut __sproto_tag: i32 = -1;

                // Declare field variables
                #(#field_declarations)*

                // Parse fields
                for __sproto_i in 0..fn_count {
                    let __sproto_wire_value = read_u16_le(&field_part[__sproto_i * SIZEOF_FIELD..]) as i32;
                    __sproto_tag += 1;

                    if __sproto_wire_value & 1 != 0 {
                        // Odd value: skip tag
                        __sproto_tag += __sproto_wire_value / 2;
                        continue;
                    }

                    let __sproto_decoded_value = __sproto_wire_value / 2 - 1;
                    let __sproto_data_start = data_offset;

                    // If __sproto_decoded_value < 0, the data is in the data part
                    if __sproto_decoded_value < 0 {
                        if data_offset + SIZEOF_LENGTH > size {
                            return Err(::sproto::error::DecodeError::Truncated {
                                need: data_offset + SIZEOF_LENGTH,
                                have: size,
                            });
                        }
                        let dsz = read_u32_le(&data[data_offset..]) as usize;
                        if data_offset + SIZEOF_LENGTH + dsz > size {
                            return Err(::sproto::error::DecodeError::Truncated {
                                need: data_offset + SIZEOF_LENGTH + dsz,
                                have: size,
                            });
                        }
                        data_offset += SIZEOF_LENGTH + dsz;
                    }

                    let field_data_slice = &data[__sproto_data_start..data_offset];
                    let inline_val = if __sproto_decoded_value >= 0 { Some(__sproto_decoded_value as u64) } else { None };

                    match __sproto_tag as u16 {
                        #(#match_arms)*
                        _ => {} // Unknown tag, skip for forward compatibility
                    }
                }

                Ok(Self {
                    #(#field_extractions),*
                })
            }
        }
    })
}

/// Field type info for code generation
#[derive(Clone, Copy, Debug)]
enum FieldTypeInfo {
    Integer,
    Boolean,
    Double,
    String,
    Binary,
}

/// Analyze a type to determine if it's Option<T> or Vec<T>, returning inner type.
fn analyze_type(ty: &Type) -> (bool, bool, Option<&Type>) {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            if ident == "Option" || ident == "Vec" {
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

/// Convert Rust type to FieldTypeInfo
fn rust_type_to_field_type_info(ty: &Type) -> FieldTypeInfo {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            return match ident.as_str() {
                "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize" => {
                    FieldTypeInfo::Integer
                }
                "bool" => FieldTypeInfo::Boolean,
                "f32" | "f64" => FieldTypeInfo::Double,
                "String" | "str" => FieldTypeInfo::String,
                "Vec" => {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner)) = args.args.first() {
                            if let Type::Path(inner_path) = inner {
                                if let Some(inner_seg) = inner_path.path.segments.last() {
                                    if inner_seg.ident == "u8" {
                                        return FieldTypeInfo::Binary;
                                    }
                                }
                            }
                            return rust_type_to_field_type_info(inner);
                        }
                    }
                    FieldTypeInfo::Integer
                }
                _ => FieldTypeInfo::String,
            };
        }
    }
    FieldTypeInfo::String
}

/// Generate decoding code for a scalar (non-array) field
fn generate_scalar_decode(
    ident: &syn::Ident,
    is_optional: bool,
    use_default: bool,
    field_type: FieldTypeInfo,
) -> TokenStream {
    let decode_value = match field_type {
        FieldTypeInfo::Integer => quote! {
            if let Some(v) = inline_val {
                decoded = Some(v as i64);
            } else {
                let sz = read_u32_le(&field_data_slice[0..]) as usize;
                let content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
                if sz == 4 {
                    decoded = Some(expand64(read_u32_le(content)) as i64);
                } else if sz == 8 {
                    let low = read_u32_le(content) as u64;
                    let hi = read_u32_le(&content[4..]) as u64;
                    decoded = Some((low | (hi << 32)) as i64);
                }
            }
        },
        FieldTypeInfo::Boolean => quote! {
            if let Some(v) = inline_val {
                decoded = Some(v != 0);
            }
        },
        FieldTypeInfo::Double => quote! {
            if inline_val.is_none() {
                let sz = read_u32_le(&field_data_slice[0..]) as usize;
                let content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
                if sz == 8 {
                    let low = read_u32_le(content) as u64;
                    let hi = read_u32_le(&content[4..]) as u64;
                    let bits = low | (hi << 32);
                    decoded = Some(f64::from_bits(bits));
                }
            }
        },
        FieldTypeInfo::String => quote! {
            if inline_val.is_none() {
                let sz = read_u32_le(&field_data_slice[0..]) as usize;
                let content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
                decoded = Some(String::from_utf8(content.to_vec()).map_err(|e| {
                    ::sproto::error::DecodeError::InvalidData(format!("invalid UTF-8: {}", e))
                })?);
            }
        },
        FieldTypeInfo::Binary => quote! {
            if inline_val.is_none() {
                let sz = read_u32_le(&field_data_slice[0..]) as usize;
                let content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
                decoded = Some(content.to_vec());
            }
        },
    };

    let type_hint = match field_type {
        FieldTypeInfo::Integer => quote! { Option<i64> },
        FieldTypeInfo::Boolean => quote! { Option<bool> },
        FieldTypeInfo::Double => quote! { Option<f64> },
        FieldTypeInfo::String => quote! { Option<String> },
        FieldTypeInfo::Binary => quote! { Option<Vec<u8>> },
    };

    if is_optional {
        quote! {
            let mut decoded: #type_hint = None;
            #decode_value
            #ident = decoded;
        }
    } else if use_default {
        quote! {
            let mut decoded: #type_hint = None;
            #decode_value
            if let Some(v) = decoded {
                #ident = v;
            }
        }
    } else {
        quote! {
            let mut decoded: #type_hint = None;
            #decode_value
            #ident = decoded;
        }
    }
}

/// Generate decoding code for an array field
fn generate_array_decode(
    ident: &syn::Ident,
    is_optional: bool,
    use_default: bool,
    field_type: FieldTypeInfo,
) -> TokenStream {
    let decode_array = match field_type {
        FieldTypeInfo::Integer => quote! {
            let sz = read_u32_le(&field_data_slice[0..]) as usize;
            if sz == 0 {
                decoded = Some(Vec::<i64>::new());
            } else {
                let content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
                let int_len = content[0] as usize;
                let values_data = &content[1..];
                
                if int_len == 4 || int_len == 8 {
                    let count = values_data.len() / int_len;
                    let mut arr: Vec<i64> = Vec::with_capacity(count);
                    for i in 0..count {
                        let offset = i * int_len;
                        if int_len == 4 {
                            arr.push(expand64(read_u32_le(&values_data[offset..])) as i64);
                        } else {
                            let low = read_u32_le(&values_data[offset..]) as u64;
                            let hi = read_u32_le(&values_data[offset + 4..]) as u64;
                            arr.push((low | (hi << 32)) as i64);
                        }
                    }
                    decoded = Some(arr);
                }
            }
        },
        FieldTypeInfo::Boolean => quote! {
            let sz = read_u32_le(&field_data_slice[0..]) as usize;
            let content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
            let arr: Vec<bool> = content.iter().map(|&b| b != 0).collect();
            decoded = Some(arr);
        },
        FieldTypeInfo::Double => quote! {
            let sz = read_u32_le(&field_data_slice[0..]) as usize;
            if sz == 0 {
                decoded = Some(Vec::<f64>::new());
            } else {
                let content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
                let int_len = content[0] as usize;
                let values_data = &content[1..];
                
                if int_len == 8 {
                    let count = values_data.len() / int_len;
                    let mut arr: Vec<f64> = Vec::with_capacity(count);
                    for i in 0..count {
                        let offset = i * 8;
                        let low = read_u32_le(&values_data[offset..]) as u64;
                        let hi = read_u32_le(&values_data[offset + 4..]) as u64;
                        arr.push(f64::from_bits(low | (hi << 32)));
                    }
                    decoded = Some(arr);
                }
            }
        },
        FieldTypeInfo::String => quote! {
            let sz = read_u32_le(&field_data_slice[0..]) as usize;
            let mut content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
            let mut arr: Vec<String> = Vec::new();
            while !content.is_empty() {
                let elem_sz = read_u32_le(content) as usize;
                let elem_data = &content[SIZEOF_LENGTH..SIZEOF_LENGTH + elem_sz];
                arr.push(String::from_utf8(elem_data.to_vec()).map_err(|e| {
                    ::sproto::error::DecodeError::InvalidData(format!("invalid UTF-8: {}", e))
                })?);
                content = &content[SIZEOF_LENGTH + elem_sz..];
            }
            decoded = Some(arr);
        },
        FieldTypeInfo::Binary => quote! {
            let sz = read_u32_le(&field_data_slice[0..]) as usize;
            let mut content = &field_data_slice[SIZEOF_LENGTH..SIZEOF_LENGTH + sz];
            let mut arr: Vec<Vec<u8>> = Vec::new();
            while !content.is_empty() {
                let elem_sz = read_u32_le(content) as usize;
                let elem_data = &content[SIZEOF_LENGTH..SIZEOF_LENGTH + elem_sz];
                arr.push(elem_data.to_vec());
                content = &content[SIZEOF_LENGTH + elem_sz..];
            }
            decoded = Some(arr);
        },
    };

    let type_hint = match field_type {
        FieldTypeInfo::Integer => quote! { Option<Vec<i64>> },
        FieldTypeInfo::Boolean => quote! { Option<Vec<bool>> },
        FieldTypeInfo::Double => quote! { Option<Vec<f64>> },
        FieldTypeInfo::String => quote! { Option<Vec<String>> },
        FieldTypeInfo::Binary => quote! { Option<Vec<Vec<u8>>> },
    };

    if is_optional {
        quote! {
            let mut decoded: #type_hint = None;
            #decode_array
            #ident = decoded;
        }
    } else if use_default {
        quote! {
            let mut decoded: #type_hint = None;
            #decode_array
            if let Some(v) = decoded {
                #ident = v;
            }
        }
    } else {
        quote! {
            let mut decoded: #type_hint = None;
            #decode_array
            #ident = decoded;
        }
    }
}
