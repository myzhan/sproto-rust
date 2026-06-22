use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Type};

use crate::classify::{classify_scalar_kind, classify_type, ClassifiedField, FieldKind};
use crate::parse::parse_field_attrs;

struct FieldInfo {
    ident: syn::Ident,
    tag: u16,
    classified: ClassifiedField,
    ty: Type,
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
                    "SprotoDecode only supports named structs",
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

    let mut field_infos: Vec<FieldInfo> = Vec::new();
    for field in fields.iter() {
        let attrs = parse_field_attrs(&field.attrs)?;
        let classified = classify_type(
            &field.ty,
            attrs.decimal,
            attrs.key,
            attrs.value,
            attrs.key_field,
        );
        field_infos.push(FieldInfo {
            ident: field.ident.clone().unwrap(),
            tag: attrs.tag,
            classified,
            ty: field.ty.clone(),
        });
    }

    let var_decls: Vec<TokenStream> = field_infos.iter().map(gen_var_decl).collect();
    let match_arms: Vec<TokenStream> = field_infos.iter().map(gen_match_arm).collect();
    let struct_fields: Vec<TokenStream> = field_infos
        .iter()
        .map(|f| {
            let ident = &f.ident;
            quote! { #ident }
        })
        .collect();

    let expanded = quote! {
        impl #impl_generics ::sproto::SprotoDecode for #struct_name #ty_generics #where_clause {
            fn sproto_decode(dec: &mut ::sproto::codec::StructDecoder) -> ::std::result::Result<Self, ::sproto::error::DecodeError> {
                #(#var_decls)*

                while let Some(__field) = dec.next_field()? {
                    match __field.tag() {
                        #(#match_arms)*
                        _ => {}
                    }
                }

                Ok(Self {
                    #(#struct_fields,)*
                })
            }
        }
    };

    Ok(expanded)
}

fn gen_var_decl(field: &FieldInfo) -> TokenStream {
    let ident = &field.ident;
    let ty = &field.ty;

    if field.classified.is_optional {
        quote! { let mut #ident: #ty = None; }
    } else {
        match &field.classified.kind {
            FieldKind::StringField => quote! { let mut #ident = String::new(); },
            FieldKind::Binary => quote! { let mut #ident: Vec<u8> = Vec::new(); },
            FieldKind::IntegerArray => quote! { let mut #ident: #ty = Vec::new(); },
            FieldKind::BoolArray => quote! { let mut #ident: Vec<bool> = Vec::new(); },
            FieldKind::DoubleArray => quote! { let mut #ident: Vec<f64> = Vec::new(); },
            FieldKind::StringArray => quote! { let mut #ident: Vec<String> = Vec::new(); },
            FieldKind::BytesArray => quote! { let mut #ident: Vec<Vec<u8>> = Vec::new(); },
            FieldKind::StructArray(_) => quote! { let mut #ident: #ty = Vec::new(); },
            FieldKind::Integer => quote! { let mut #ident: #ty = Default::default(); },
            FieldKind::Bool => quote! { let mut #ident: bool = false; },
            FieldKind::Double => quote! { let mut #ident: f64 = 0.0; },
            FieldKind::Decimal(_) => quote! { let mut #ident: f64 = 0.0; },
            FieldKind::NestedStruct(_) => {
                quote! { let mut #ident: #ty = Default::default(); }
            }
            FieldKind::IndexedMap { .. } | FieldKind::AnonymousMap { .. } => {
                quote! { let mut #ident: #ty = std::collections::HashMap::new(); }
            }
        }
    }
}

fn gen_match_arm(field: &FieldInfo) -> TokenStream {
    let tag = field.tag;
    let decode_expr = gen_decode_expr(field);

    quote! {
        #tag => { #decode_expr }
    }
}

fn gen_decode_expr(field: &FieldInfo) -> TokenStream {
    if field.classified.is_optional {
        gen_decode_optional(&field.ident, &field.classified)
    } else {
        gen_decode_required(&field.ident, &field.classified)
    }
}

fn gen_decode_optional(ident: &syn::Ident, classified: &ClassifiedField) -> TokenStream {
    let value_expr = gen_decode_value(classified);
    if classified.is_boxed {
        quote! { #ident = Some(Box::new(#value_expr)); }
    } else {
        quote! { #ident = Some(#value_expr); }
    }
}

fn gen_decode_required(ident: &syn::Ident, classified: &ClassifiedField) -> TokenStream {
    let value_expr = gen_decode_value(classified);
    if classified.is_boxed {
        quote! { #ident = Box::new(#value_expr); }
    } else {
        quote! { #ident = #value_expr; }
    }
}

fn gen_decode_value(classified: &ClassifiedField) -> TokenStream {
    match &classified.kind {
        FieldKind::Integer => quote! { __field.as_integer()? },
        FieldKind::Bool => quote! { __field.as_bool()? },
        FieldKind::Double => quote! { __field.as_double()? },
        FieldKind::Decimal(precision) => {
            let scale = 10_f64.powi(*precision as i32) as u64;
            quote! { (__field.as_integer()? as f64 / #scale as f64) }
        }
        FieldKind::StringField => quote! { __field.as_string()?.to_owned() },
        FieldKind::Binary => quote! { __field.as_bytes().to_vec() },
        FieldKind::IntegerArray => quote! { __field.as_integer_array()? },
        FieldKind::BoolArray => quote! { __field.as_bool_array() },
        FieldKind::DoubleArray => quote! { __field.as_double_array()? },
        FieldKind::StringArray => quote! {
            __field.as_string_array()?.iter().map(|s| s.to_string()).collect()
        },
        FieldKind::BytesArray => quote! {
            __field.as_bytes_array()?.iter().map(|b| b.to_vec()).collect()
        },
        FieldKind::StructArray(elem_ty) => quote! {
            {
                let mut __items = Vec::new();
                for __elem in __field.as_struct_iter()? {
                    let mut __sub = __elem?;
                    __items.push(<#elem_ty as ::sproto::SprotoDecode>::sproto_decode(&mut __sub)?);
                }
                __items
            }
        },
        FieldKind::NestedStruct(inner_ty) => quote! {
            {
                let mut __sub = __field.as_struct()?;
                <#inner_ty as ::sproto::SprotoDecode>::sproto_decode(&mut __sub)?
            }
        },
        FieldKind::IndexedMap { val_ty, .. } => {
            let key_field_name = classified
                .key_field
                .as_ref()
                .expect("IndexedMap must have key_field");
            let key_ident = syn::Ident::new(key_field_name, proc_macro2::Span::call_site());
            quote! {
                {
                    let mut __map = std::collections::HashMap::new();
                    for __elem in __field.as_struct_iter()? {
                        let mut __sub = __elem?;
                        let __item = <#val_ty as ::sproto::SprotoDecode>::sproto_decode(&mut __sub)?;
                        let __key = __item.#key_ident.clone();
                        __map.insert(__key, __item);
                    }
                    __map
                }
            }
        }
        FieldKind::AnonymousMap { key_ty, val_ty } => {
            let key_tag = classified.key_tag.expect("AnonymousMap must have key_tag");
            let value_tag = classified
                .value_tag
                .expect("AnonymousMap must have value_tag");
            let key_decode = gen_scalar_decode_expr(key_ty, quote! { __sf });
            let val_decode = gen_scalar_decode_expr(val_ty, quote! { __sf });

            quote! {
                {
                    let mut __map = std::collections::HashMap::new();
                    for __elem in __field.as_struct_iter()? {
                        let mut __sub = __elem?;
                        let mut __key_opt = None;
                        let mut __val_opt = None;
                        while let Some(__sf) = __sub.next_field()? {
                            if __sf.tag() == #key_tag {
                                __key_opt = Some(#key_decode);
                            } else if __sf.tag() == #value_tag {
                                __val_opt = Some(#val_decode);
                            }
                        }
                        if let (Some(__k), Some(__v)) = (__key_opt, __val_opt) {
                            __map.insert(__k, __v);
                        }
                    }
                    __map
                }
            }
        }
    }
}

fn gen_scalar_decode_expr(ty: &syn::Type, field_var: TokenStream) -> TokenStream {
    let kind = classify_scalar_kind(ty);
    match kind {
        FieldKind::Integer => quote! { #field_var.as_integer()? },
        FieldKind::Bool => quote! { #field_var.as_bool()? },
        FieldKind::Double => quote! { #field_var.as_double()? },
        FieldKind::StringField => quote! { #field_var.as_string()?.to_owned() },
        FieldKind::Binary => quote! { #field_var.as_bytes().to_vec() },
        _ => quote! { #field_var.as_string()?.to_owned() },
    }
}
