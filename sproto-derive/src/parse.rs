use syn::{Attribute, Lit};

/// Parsed `#[sproto(...)]` attributes for a field.
pub struct FieldAttrs {
    pub tag: u16,
    pub decimal: Option<u32>,
    pub key: Option<u16>,
    pub value: Option<u16>,
    pub key_field: Option<String>,
}

pub fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut tag: Option<u16> = None;
    let mut decimal: Option<u32> = None;
    let mut key: Option<u16> = None;
    let mut value: Option<u16> = None;
    let mut key_field: Option<String> = None;

    for attr in attrs {
        if !attr.path().is_ident("sproto") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let v = meta.value()?;
                let lit: Lit = v.parse()?;
                match lit {
                    Lit::Int(lit_int) => {
                        tag = Some(lit_int.base10_parse::<u16>()?);
                    }
                    _ => {
                        return Err(meta.error("expected integer literal for `tag`"));
                    }
                }
                Ok(())
            } else if meta.path.is_ident("decimal") {
                let v = meta.value()?;
                let lit: Lit = v.parse()?;
                match lit {
                    Lit::Int(lit_int) => {
                        decimal = Some(lit_int.base10_parse::<u32>()?);
                    }
                    _ => {
                        return Err(meta.error("expected integer literal for `decimal`"));
                    }
                }
                Ok(())
            } else if meta.path.is_ident("key") {
                let v = meta.value()?;
                let lit: Lit = v.parse()?;
                match lit {
                    Lit::Int(lit_int) => {
                        key = Some(lit_int.base10_parse::<u16>()?);
                    }
                    _ => {
                        return Err(meta.error("expected integer literal for `key`"));
                    }
                }
                Ok(())
            } else if meta.path.is_ident("value") {
                let v = meta.value()?;
                let lit: Lit = v.parse()?;
                match lit {
                    Lit::Int(lit_int) => {
                        value = Some(lit_int.base10_parse::<u16>()?);
                    }
                    _ => {
                        return Err(meta.error("expected integer literal for `value`"));
                    }
                }
                Ok(())
            } else if meta.path.is_ident("key_field") {
                let v = meta.value()?;
                let lit: Lit = v.parse()?;
                match lit {
                    Lit::Str(lit_str) => {
                        key_field = Some(lit_str.value());
                    }
                    _ => {
                        return Err(meta.error("expected string literal for `key_field`"));
                    }
                }
                Ok(())
            } else {
                Err(meta.error("unknown sproto attribute"))
            }
        })?;
    }

    let tag = tag
        .ok_or_else(|| syn::Error::new_spanned(&attrs[0], "missing `tag` in #[sproto(tag = N)]"))?;

    Ok(FieldAttrs {
        tag,
        decimal,
        key,
        value,
        key_field,
    })
}
