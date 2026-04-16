//! Code generation for SprotoEncode derive macro.
//!
//! This module generates optimized inline encoding code that directly writes
//! to a shared output buffer without intermediate allocations for nested structs.

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
            syn::Error::new_spanned(&field.ident, "field must have #[sproto(tag = N)] attribute")
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
            fn sproto_encode_to(&self, __sproto_out: &mut ::std::vec::Vec<u8>) -> ::std::result::Result<(), ::sproto::error::EncodeError> {
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

                let __sproto_start = __sproto_out.len();

                // Pre-allocate header space in the shared buffer
                let __sproto_header_sz = SIZEOF_HEADER + #maxn * SIZEOF_FIELD;
                __sproto_out.resize(__sproto_start + __sproto_header_sz, 0);
                let mut index = 0usize;
                let mut last_tag: i32 = -1;

                #(#field_encode_blocks)*

                // Write field count
                write_u16_le(&mut __sproto_out[__sproto_start..], index as u16);

                // Compact header if not all slots were used
                let __sproto_used_header = SIZEOF_HEADER + index * SIZEOF_FIELD;
                if __sproto_used_header < __sproto_header_sz {
                    let __sproto_data_start = __sproto_start + __sproto_header_sz;
                    let __sproto_total = __sproto_out.len();
                    let __sproto_data_len = __sproto_total - __sproto_data_start;
                    let __sproto_new_data_start = __sproto_start + __sproto_used_header;
                    __sproto_out.copy_within(__sproto_data_start..__sproto_total, __sproto_new_data_start);
                    __sproto_out.truncate(__sproto_new_data_start + __sproto_data_len);
                }

                Ok(())
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
    Struct,
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
                _ => FieldTypeInfo::Struct,
            };
        }
    }
    FieldTypeInfo::Struct
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
                            let offset = __sproto_start + SIZEOF_HEADER + SIZEOF_FIELD * index;
                            write_u16_le(&mut __sproto_out[offset..], skip);
                            index += 1;
                        }

                        // Write field descriptor
                        let offset = __sproto_start + SIZEOF_HEADER + SIZEOF_FIELD * index;
                        write_u16_le(&mut __sproto_out[offset..], inline_value);
                        index += 1;
                        last_tag = #tag;
                    }
                }
            }
        })
        .collect()
}

/// Generate encoding code for a scalar (non-array) field
fn generate_scalar_encode(
    ident: &syn::Ident,
    is_optional: bool,
    field_type: FieldTypeInfo,
) -> TokenStream {
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
                    __sproto_out.extend_from_slice(&4u32.to_le_bytes());
                    __sproto_out.extend_from_slice(&(int_val as u32).to_le_bytes());
                } else {
                    __sproto_out.extend_from_slice(&8u32.to_le_bytes());
                    __sproto_out.extend_from_slice(&uint_val.to_le_bytes());
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
            __sproto_out.extend_from_slice(&8u32.to_le_bytes());
            __sproto_out.extend_from_slice(&bits.to_le_bytes());
            has_value = true;
        },
        FieldTypeInfo::String => quote! {
            let s = val.as_bytes();
            __sproto_out.extend_from_slice(&(s.len() as u32).to_le_bytes());
            __sproto_out.extend_from_slice(s);
            has_value = true;
        },
        FieldTypeInfo::Binary => quote! {
            let b = val.as_slice();
            __sproto_out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            __sproto_out.extend_from_slice(b);
            has_value = true;
        },
        FieldTypeInfo::Struct => quote! {
            let __len_pos = __sproto_out.len();
            __sproto_out.extend_from_slice(&[0u8; SIZEOF_LENGTH]);
            ::sproto::SprotoEncode::sproto_encode_to(val, __sproto_out)?;
            let __encoded_len = (__sproto_out.len() - __len_pos - SIZEOF_LENGTH) as u32;
            write_u32_le(&mut __sproto_out[__len_pos..], __encoded_len);
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
fn generate_array_encode(
    ident: &syn::Ident,
    is_optional: bool,
    field_type: FieldTypeInfo,
) -> TokenStream {
    let encode_array = match field_type {
        FieldTypeInfo::Integer => quote! {
            if arr.is_empty() {
                __sproto_out.extend_from_slice(&0u32.to_le_bytes());
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
                __sproto_out.extend_from_slice(&(data_len as u32).to_le_bytes());
                __sproto_out.push(int_size as u8);

                for &v in arr.iter() {
                    if need_64bit {
                        __sproto_out.extend_from_slice(&(v as i64 as u64).to_le_bytes());
                    } else {
                        __sproto_out.extend_from_slice(&(v as i64 as u32).to_le_bytes());
                    }
                }
            }
            has_value = true;
        },
        FieldTypeInfo::Boolean => quote! {
            let data_len = arr.len();
            __sproto_out.extend_from_slice(&(data_len as u32).to_le_bytes());
            for &v in arr.iter() {
                __sproto_out.push(if v { 1 } else { 0 });
            }
            has_value = true;
        },
        FieldTypeInfo::Double => quote! {
            if arr.is_empty() {
                __sproto_out.extend_from_slice(&0u32.to_le_bytes());
            } else {
                let int_size = 8usize;
                let data_len = 1 + arr.len() * int_size;
                __sproto_out.extend_from_slice(&(data_len as u32).to_le_bytes());
                __sproto_out.push(int_size as u8);

                for &v in arr.iter() {
                    __sproto_out.extend_from_slice(&v.to_bits().to_le_bytes());
                }
            }
            has_value = true;
        },
        FieldTypeInfo::String => quote! {
            let __outer_pos = __sproto_out.len();
            __sproto_out.extend_from_slice(&[0u8; SIZEOF_LENGTH]);
            for s in arr.iter() {
                let bytes = s.as_bytes();
                __sproto_out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
                __sproto_out.extend_from_slice(bytes);
            }
            let __outer_len = (__sproto_out.len() - __outer_pos - SIZEOF_LENGTH) as u32;
            write_u32_le(&mut __sproto_out[__outer_pos..], __outer_len);
            has_value = true;
        },
        FieldTypeInfo::Binary => quote! {
            let __outer_pos = __sproto_out.len();
            __sproto_out.extend_from_slice(&[0u8; SIZEOF_LENGTH]);
            for b in arr.iter() {
                __sproto_out.extend_from_slice(&(b.len() as u32).to_le_bytes());
                __sproto_out.extend_from_slice(b);
            }
            let __outer_len = (__sproto_out.len() - __outer_pos - SIZEOF_LENGTH) as u32;
            write_u32_le(&mut __sproto_out[__outer_pos..], __outer_len);
            has_value = true;
        },
        FieldTypeInfo::Struct => quote! {
            let __outer_pos = __sproto_out.len();
            __sproto_out.extend_from_slice(&[0u8; SIZEOF_LENGTH]);
            for elem in arr.iter() {
                let __elem_pos = __sproto_out.len();
                __sproto_out.extend_from_slice(&[0u8; SIZEOF_LENGTH]);
                ::sproto::SprotoEncode::sproto_encode_to(elem, __sproto_out)?;
                let __elem_len = (__sproto_out.len() - __elem_pos - SIZEOF_LENGTH) as u32;
                write_u32_le(&mut __sproto_out[__elem_pos..], __elem_len);
            }
            let __outer_len = (__sproto_out.len() - __outer_pos - SIZEOF_LENGTH) as u32;
            write_u32_le(&mut __sproto_out[__outer_pos..], __outer_len);
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
