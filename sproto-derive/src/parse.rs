use syn::{Attribute, Lit};

/// Parsed `#[sproto(...)]` attributes for a field.
pub struct FieldAttrs {
    pub tag: u16,
    pub decimal: Option<u32>,
}

pub fn parse_field_attrs(attrs: &[Attribute]) -> syn::Result<FieldAttrs> {
    let mut tag: Option<u16> = None;
    let mut decimal: Option<u32> = None;

    for attr in attrs {
        if !attr.path().is_ident("sproto") {
            continue;
        }
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
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
                let value = meta.value()?;
                let lit: Lit = value.parse()?;
                match lit {
                    Lit::Int(lit_int) => {
                        decimal = Some(lit_int.base10_parse::<u32>()?);
                    }
                    _ => {
                        return Err(meta.error("expected integer literal for `decimal`"));
                    }
                }
                Ok(())
            } else {
                Err(meta.error("unknown sproto attribute"))
            }
        })?;
    }

    let tag = tag.ok_or_else(|| {
        syn::Error::new_spanned(
            &attrs[0],
            "missing `tag` in #[sproto(tag = N)]",
        )
    })?;

    Ok(FieldAttrs { tag, decimal })
}
