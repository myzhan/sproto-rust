//! Code generation for SprotoEncode derive macro.
//!
//! This module generates optimized inline encoding code that directly writes
//! to bytes without constructing intermediate SprotoValue or Schema objects.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Fields, GenericArgument, PathArguments, Result, Type};

use crate::attr::{validate_fields, FieldAttrs, FieldInfo, StructAttrs};

/// Generate the SprotoEncode implementation for a struct.
pub fn derive_encode(input: &DeriveInput) -> Result<TokenStream> {
    let name = &input.ident;
    let _struct_attrs = StructAttrs::from_attrs(&input.attrs)?;

    let fields = match &input.data {
        syn::Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "SprotoEncode only supports structs with named fields",
                ))
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                input,
                "SprotoEncode only supports structs",
            ))
        }
    };

    // Parse field attributes and collect field info with types
    let mut field_data: Vec<(FieldInfo, FieldTypeInfo)> = Vec::new();
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
                    use_default: false,
                    span: field.ident.as_ref().unwrap().span(),
                },
                FieldTypeInfo::Integer,
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
        ));
    }

    // Validate tags
    let field_infos: Vec<_> = field_data.iter().map(|(info, _)| info.clone()).collect();
    validate_fields(&field_infos)?;

    // Sort fields by tag for wire format ordering
    let mut sorted_fields: Vec<_> = field_data.iter().filter(|(f, _)| !f.skip).collect();
    sorted_fields.sort_by_key(|(f, _)| f.tag);

    // Calculate maxn - must include skip markers for non-contiguous tags
    let maxn = if sorted_fields.is_empty() {
        0usize
    } else {
        let num_fields = sorted_fields.len();
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

    // Generate encoding code for each field
    let field_encode_blocks = generate_field_encode_blocks(&sorted_fields);

    Ok(quote! {
        impl ::sproto::SprotoEncode for #name {
            fn sproto_encode(&self) -> ::std::result::Result<::std::vec::Vec<u8>, ::sproto::error::EncodeError> {
                // Constants for wire format
                const SIZEOF_HEADER: usize = 2;
                const SIZEOF_FIELD: usize = 2;
                const SIZEOF_LENGTH: usize = 4;

                #[inline]
                fn write_u16_le(buf: &mut [u8], val: u16) {
                    let bytes = val.to_le_bytes();
                    buf[0] = bytes[0];
                    buf[1] = bytes[1];
                }

                #[inline]
                fn write_u32_le(buf: &mut [u8], val: u32) {
                    let bytes = val.to_le_bytes();
                    buf[..4].copy_from_slice(&bytes);
                }

                #[inline]
                fn write_u64_le(buf: &mut [u8], val: u64) {
                    let bytes = val.to_le_bytes();
                    buf[..8].copy_from_slice(&bytes);
                }

                // Pre-allocate header buffer
                let header_sz = SIZEOF_HEADER + #maxn * SIZEOF_FIELD;
                let mut header = vec![0u8; header_sz];
                let mut data_part: Vec<u8> = Vec::new();
                let mut index = 0usize;
                let mut last_tag: i32 = -1;

                #(#field_encode_blocks)*

                // Write field count
                write_u16_le(&mut header[0..], index as u16);

                // Compact header and combine with data
                let used_header = SIZEOF_HEADER + index * SIZEOF_FIELD;
                header.truncate(used_header);
                header.extend_from_slice(&data_part);

                Ok(header)
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
/// Also handles Option<Vec<T>> by setting both is_optional and is_vec.
fn analyze_type(ty: &Type) -> (bool, bool, Option<&Type>) {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            let ident = segment.ident.to_string();
            if ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        // Check if inner is Vec<T> -> Option<Vec<T>>
                        let (_, inner_is_vec, inner_inner_type) = analyze_type(inner);
                        if inner_is_vec {
                            // Option<Vec<T>> -> is_optional=true, is_vec=true, inner_type=T
                            return (true, true, inner_inner_type);
                        }
                        return (true, false, Some(inner));
                    }
                }
                return (true, false, None);
            } else if ident == "Vec" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return (false, true, Some(inner));
                    }
                }
                return (false, true, None);
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

/// Generate encoding blocks for each field
fn generate_field_encode_blocks(sorted_fields: &[&(FieldInfo, FieldTypeInfo)]) -> Vec<TokenStream> {
    sorted_fields
        .iter()
        .map(|(field, field_type)| {
            let ident = &field.ident;
            let tag = field.tag as i32;

            let encode_logic = if field.is_vec {
                generate_array_encode(ident, field.is_optional, *field_type)
            } else {
                generate_scalar_encode(ident, field.is_optional, *field_type)
            };

            quote! {
                // Field: #ident, tag: #tag
                {
                    #encode_logic

                    if has_value {
                        // Handle tag gap
                        let tag_gap = #tag - last_tag - 1;
                        if tag_gap > 0 {
                            let skip = ((tag_gap - 1) * 2 + 1) as u16;
                            let offset = SIZEOF_HEADER + SIZEOF_FIELD * index;
                            write_u16_le(&mut header[offset..], skip);
                            index += 1;
                        }

                        // Write field descriptor
                        let offset = SIZEOF_HEADER + SIZEOF_FIELD * index;
                        write_u16_le(&mut header[offset..], inline_value);
                        index += 1;
                        last_tag = #tag;
                    }
                }
            }
        })
        .collect()
}

/// Generate encoding code for a scalar (non-array) field
fn generate_scalar_encode(ident: &syn::Ident, is_optional: bool, field_type: FieldTypeInfo) -> TokenStream {
    let encode_value = match field_type {
        FieldTypeInfo::Integer => quote! {
            let int_val = *val as i64;
            let uint_val = int_val as u64;
            let u32_val = uint_val as u32;
            
            // Try inline for small positive values
            if uint_val == u32_val as u64 && u32_val < 0x7fff {
                inline_value = ((u32_val + 1) * 2) as u16;
            } else {
                // Check if fits in 32 bits
                let i32_check = int_val as i32;
                if i32_check as i64 == int_val {
                    let mut buf = vec![0u8; SIZEOF_LENGTH + 4];
                    write_u32_le(&mut buf[0..], 4);
                    write_u32_le(&mut buf[SIZEOF_LENGTH..], int_val as u32);
                    data_part.extend_from_slice(&buf);
                } else {
                    let mut buf = vec![0u8; SIZEOF_LENGTH + 8];
                    write_u32_le(&mut buf[0..], 8);
                    write_u64_le(&mut buf[SIZEOF_LENGTH..], uint_val);
                    data_part.extend_from_slice(&buf);
                }
            }
            has_value = true;
        },
        FieldTypeInfo::Boolean => quote! {
            let int_val = if *val { 1u32 } else { 0u32 };
            inline_value = ((int_val + 1) * 2) as u16;
            has_value = true;
        },
        FieldTypeInfo::Double => quote! {
            let bits = (*val).to_bits();
            let mut buf = vec![0u8; SIZEOF_LENGTH + 8];
            write_u32_le(&mut buf[0..], 8);
            write_u64_le(&mut buf[SIZEOF_LENGTH..], bits);
            data_part.extend_from_slice(&buf);
            has_value = true;
        },
        FieldTypeInfo::String => quote! {
            let s = val.as_bytes();
            let mut buf = vec![0u8; SIZEOF_LENGTH + s.len()];
            write_u32_le(&mut buf[0..], s.len() as u32);
            buf[SIZEOF_LENGTH..].copy_from_slice(s);
            data_part.extend_from_slice(&buf);
            has_value = true;
        },
        FieldTypeInfo::Binary => quote! {
            let b = val.as_slice();
            let mut buf = vec![0u8; SIZEOF_LENGTH + b.len()];
            write_u32_le(&mut buf[0..], b.len() as u32);
            buf[SIZEOF_LENGTH..].copy_from_slice(b);
            data_part.extend_from_slice(&buf);
            has_value = true;
        },
    };

    if is_optional {
        quote! {
            let mut has_value = false;
            let mut inline_value: u16 = 0;
            if let Some(ref val) = self.#ident {
                #encode_value
            }
        }
    } else {
        quote! {
            let mut has_value = false;
            let mut inline_value: u16 = 0;
            {
                let val = &self.#ident;
                #encode_value
            }
        }
    }
}

/// Generate encoding code for an array field
fn generate_array_encode(ident: &syn::Ident, is_optional: bool, field_type: FieldTypeInfo) -> TokenStream {
    let encode_array = match field_type {
        FieldTypeInfo::Integer => quote! {
            if arr.is_empty() {
                // Empty array
                let mut buf = vec![0u8; SIZEOF_LENGTH];
                write_u32_le(&mut buf[0..], 0);
                data_part.extend_from_slice(&buf);
            } else {
                // Check if we need 64-bit
                let mut need_64bit = false;
                for &v in arr.iter() {
                    let ival = v as i64;
                    if (ival as i32) as i64 != ival {
                        need_64bit = true;
                        break;
                    }
                }
                
                let int_size = if need_64bit { 8usize } else { 4usize };
                let data_len = 1 + arr.len() * int_size;
                let mut buf = vec![0u8; SIZEOF_LENGTH + data_len];
                write_u32_le(&mut buf[0..], data_len as u32);
                buf[SIZEOF_LENGTH] = int_size as u8;
                
                let mut offset = SIZEOF_LENGTH + 1;
                for &v in arr.iter() {
                    if need_64bit {
                        write_u64_le(&mut buf[offset..], v as i64 as u64);
                        offset += 8;
                    } else {
                        write_u32_le(&mut buf[offset..], v as i64 as u32);
                        offset += 4;
                    }
                }
                data_part.extend_from_slice(&buf);
            }
            has_value = true;
        },
        FieldTypeInfo::Boolean => quote! {
            let data_len = arr.len();
            let mut buf = vec![0u8; SIZEOF_LENGTH + data_len];
            write_u32_le(&mut buf[0..], data_len as u32);
            for (i, &v) in arr.iter().enumerate() {
                buf[SIZEOF_LENGTH + i] = if v { 1 } else { 0 };
            }
            data_part.extend_from_slice(&buf);
            has_value = true;
        },
        FieldTypeInfo::Double => quote! {
            if arr.is_empty() {
                let mut buf = vec![0u8; SIZEOF_LENGTH];
                write_u32_le(&mut buf[0..], 0);
                data_part.extend_from_slice(&buf);
            } else {
                let int_size = 8usize;
                let data_len = 1 + arr.len() * int_size;
                let mut buf = vec![0u8; SIZEOF_LENGTH + data_len];
                write_u32_le(&mut buf[0..], data_len as u32);
                buf[SIZEOF_LENGTH] = int_size as u8;
                
                let mut offset = SIZEOF_LENGTH + 1;
                for &v in arr.iter() {
                    write_u64_le(&mut buf[offset..], v.to_bits());
                    offset += 8;
                }
                data_part.extend_from_slice(&buf);
            }
            has_value = true;
        },
        FieldTypeInfo::String => quote! {
            let mut inner = Vec::new();
            for s in arr.iter() {
                let bytes = s.as_bytes();
                let mut elem_buf = vec![0u8; SIZEOF_LENGTH + bytes.len()];
                write_u32_le(&mut elem_buf[0..], bytes.len() as u32);
                elem_buf[SIZEOF_LENGTH..].copy_from_slice(bytes);
                inner.extend_from_slice(&elem_buf);
            }
            let mut buf = vec![0u8; SIZEOF_LENGTH + inner.len()];
            write_u32_le(&mut buf[0..], inner.len() as u32);
            buf[SIZEOF_LENGTH..].copy_from_slice(&inner);
            data_part.extend_from_slice(&buf);
            has_value = true;
        },
        FieldTypeInfo::Binary => quote! {
            let mut inner = Vec::new();
            for b in arr.iter() {
                let mut elem_buf = vec![0u8; SIZEOF_LENGTH + b.len()];
                write_u32_le(&mut elem_buf[0..], b.len() as u32);
                elem_buf[SIZEOF_LENGTH..].copy_from_slice(b);
                inner.extend_from_slice(&elem_buf);
            }
            let mut buf = vec![0u8; SIZEOF_LENGTH + inner.len()];
            write_u32_le(&mut buf[0..], inner.len() as u32);
            buf[SIZEOF_LENGTH..].copy_from_slice(&inner);
            data_part.extend_from_slice(&buf);
            has_value = true;
        },
    };

    if is_optional {
        quote! {
            let mut has_value = false;
            let mut inline_value: u16 = 0;
            if let Some(ref arr) = self.#ident {
                #encode_array
            }
        }
    } else {
        quote! {
            let mut has_value = false;
            let mut inline_value: u16 = 0;
            {
                let arr = &self.#ident;
                #encode_array
            }
        }
    }
}
