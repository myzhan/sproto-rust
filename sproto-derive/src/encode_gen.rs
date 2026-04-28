use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};

use crate::classify::{classify_type, ClassifiedField, FieldKind};
use crate::parse::parse_field_attrs;

struct FieldInfo {
    ident: syn::Ident,
    tag: u16,
    classified: ClassifiedField,
}

pub fn generate(input: &DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(named) => &named.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "SprotoEncode only supports named structs",
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

    let mut field_infos: Vec<FieldInfo> = Vec::new();
    for field in fields.iter() {
        let attrs = parse_field_attrs(&field.attrs)?;
        let classified = classify_type(&field.ty, attrs.decimal);
        field_infos.push(FieldInfo {
            ident: field.ident.clone().unwrap(),
            tag: attrs.tag,
            classified,
        });
    }

    // Sort by tag for optimal in-order encoding
    field_infos.sort_by_key(|f| f.tag);

    let encode_stmts: Vec<TokenStream> = field_infos.iter().map(gen_encode_field).collect();

    let expanded = quote! {
        impl #impl_generics ::sproto::SprotoEncode for #struct_name #ty_generics #where_clause {
            fn sproto_encode(&self, enc: &mut ::sproto::codec::StructEncoder) -> ::std::result::Result<(), ::sproto::error::EncodeError> {
                #(#encode_stmts)*
                Ok(())
            }
        }
    };

    Ok(expanded)
}

fn gen_encode_field(field: &FieldInfo) -> TokenStream {
    let ident = &field.ident;
    let tag = field.tag;

    if field.classified.is_optional {
        gen_encode_optional(ident, tag, &field.classified)
    } else {
        gen_encode_required(ident, tag, &field.classified)
    }
}

fn gen_encode_optional(ident: &syn::Ident, tag: u16, classified: &ClassifiedField) -> TokenStream {
    let inner_encode = gen_encode_value_from_ref(tag, classified);
    if classified.is_boxed {
        quote! {
            if let Some(ref __boxed) = self.#ident {
                let __v = &**__boxed;
                #inner_encode
            }
        }
    } else {
        quote! {
            if let Some(ref __v) = self.#ident {
                #inner_encode
            }
        }
    }
}

fn gen_encode_required(ident: &syn::Ident, tag: u16, classified: &ClassifiedField) -> TokenStream {
    match &classified.kind {
        FieldKind::Integer => quote! {
            enc.set_integer(#tag, self.#ident as i64)?;
        },
        FieldKind::Bool => quote! {
            enc.set_bool(#tag, self.#ident)?;
        },
        FieldKind::Double => quote! {
            enc.set_double(#tag, self.#ident as f64)?;
        },
        FieldKind::Decimal(precision) => {
            let scale = 10_f64.powi(*precision as i32) as u64;
            quote! {
                enc.set_integer(#tag, (self.#ident * #scale as f64).round() as i64)?;
            }
        }
        FieldKind::StringField => quote! {
            enc.set_string(#tag, &self.#ident)?;
        },
        FieldKind::Binary => quote! {
            enc.set_bytes(#tag, &self.#ident)?;
        },
        FieldKind::IntegerArray => quote! {
            if !self.#ident.is_empty() {
                let __arr: Vec<i64> = self.#ident.iter().map(|v| *v as i64).collect();
                enc.set_integer_array(#tag, &__arr)?;
            }
        },
        FieldKind::BoolArray => quote! {
            if !self.#ident.is_empty() {
                enc.set_bool_array(#tag, &self.#ident)?;
            }
        },
        FieldKind::DoubleArray => quote! {
            if !self.#ident.is_empty() {
                enc.set_double_array(#tag, &self.#ident)?;
            }
        },
        FieldKind::StringArray => quote! {
            if !self.#ident.is_empty() {
                enc.set_string_array(#tag, &self.#ident)?;
            }
        },
        FieldKind::BytesArray => quote! {
            if !self.#ident.is_empty() {
                enc.set_bytes_array(#tag, &self.#ident)?;
            }
        },
        FieldKind::StructArray(_) => quote! {
            if !self.#ident.is_empty() {
                enc.encode_struct_array(#tag, |arr| {
                    for __item in &self.#ident {
                        arr.encode_element(|sub| __item.sproto_encode(sub))?;
                    }
                    Ok(())
                })?;
            }
        },
        FieldKind::NestedStruct(_) => {
            if classified.is_boxed {
                quote! {
                    enc.encode_nested(#tag, |sub| (*self.#ident).sproto_encode(sub))?;
                }
            } else {
                quote! {
                    enc.encode_nested(#tag, |sub| self.#ident.sproto_encode(sub))?;
                }
            }
        }
    }
}

/// Generate encode for a value accessed through `__v` reference (used in Option unwrap).
fn gen_encode_value_from_ref(tag: u16, classified: &ClassifiedField) -> TokenStream {
    match &classified.kind {
        FieldKind::Integer => quote! {
            enc.set_integer(#tag, *__v as i64)?;
        },
        FieldKind::Bool => quote! {
            enc.set_bool(#tag, *__v)?;
        },
        FieldKind::Double => quote! {
            enc.set_double(#tag, *__v as f64)?;
        },
        FieldKind::Decimal(precision) => {
            let scale = 10_f64.powi(*precision as i32) as u64;
            quote! {
                enc.set_integer(#tag, (*__v * #scale as f64).round() as i64)?;
            }
        }
        FieldKind::StringField => quote! {
            enc.set_string(#tag, __v)?;
        },
        FieldKind::Binary => quote! {
            enc.set_bytes(#tag, __v)?;
        },
        FieldKind::IntegerArray => quote! {
            if !__v.is_empty() {
                let __arr: Vec<i64> = __v.iter().map(|x| *x as i64).collect();
                enc.set_integer_array(#tag, &__arr)?;
            }
        },
        FieldKind::BoolArray => quote! {
            if !__v.is_empty() {
                enc.set_bool_array(#tag, __v)?;
            }
        },
        FieldKind::DoubleArray => quote! {
            if !__v.is_empty() {
                enc.set_double_array(#tag, __v)?;
            }
        },
        FieldKind::StringArray => quote! {
            if !__v.is_empty() {
                enc.set_string_array(#tag, __v)?;
            }
        },
        FieldKind::BytesArray => quote! {
            if !__v.is_empty() {
                enc.set_bytes_array(#tag, __v)?;
            }
        },
        FieldKind::StructArray(_) => quote! {
            if !__v.is_empty() {
                enc.encode_struct_array(#tag, |arr| {
                    for __item in __v.iter() {
                        arr.encode_element(|sub| __item.sproto_encode(sub))?;
                    }
                    Ok(())
                })?;
            }
        },
        FieldKind::NestedStruct(_) => quote! {
            enc.encode_nested(#tag, |sub| __v.sproto_encode(sub))?;
        },
    }
}
