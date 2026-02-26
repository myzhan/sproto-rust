//! Attribute parsing for sproto derive macros.

use proc_macro2::Span;
use syn::{Attribute, Expr, ExprLit, Ident, Lit, Result};

/// Parsed field attributes from #[sproto(...)]
#[derive(Default)]
pub struct FieldAttrs {
    /// The tag number for this field (required).
    pub tag: Option<u16>,
    /// Whether to skip this field during serialization.
    pub skip: bool,
    /// Whether to use Default::default() for missing fields.
    pub use_default: bool,
}

impl FieldAttrs {
    /// Parse attributes from a field.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut result = FieldAttrs::default();

        for attr in attrs {
            if attr.path().is_ident("sproto") {
                result.parse_sproto_attr(attr)?;
            }
        }

        Ok(result)
    }

    fn parse_sproto_attr(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("tag") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(ExprLit {
                    lit: Lit::Int(lit), ..
                }) = value
                {
                    let tag: u16 = lit.base10_parse()?;
                    self.tag = Some(tag);
                } else {
                    return Err(syn::Error::new_spanned(value, "expected integer literal"));
                }
            } else if meta.path.is_ident("skip") {
                self.skip = true;
            } else if meta.path.is_ident("default") {
                self.use_default = true;
            } else {
                return Err(syn::Error::new_spanned(
                    meta.path,
                    "unknown sproto attribute",
                ));
            }
            Ok(())
        })
    }
}

/// Parsed struct-level attributes.
#[derive(Default)]
pub struct StructAttrs {
    /// Custom type name (defaults to struct name).
    pub name: Option<String>,
}

impl StructAttrs {
    /// Parse attributes from a struct.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self> {
        let mut result = StructAttrs::default();

        for attr in attrs {
            if attr.path().is_ident("sproto") {
                result.parse_sproto_attr(attr)?;
            }
        }

        Ok(result)
    }

    fn parse_sproto_attr(&mut self, attr: &Attribute) -> Result<()> {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                let value: Expr = meta.value()?.parse()?;
                if let Expr::Lit(ExprLit {
                    lit: Lit::Str(lit), ..
                }) = value
                {
                    self.name = Some(lit.value());
                } else {
                    return Err(syn::Error::new_spanned(value, "expected string literal"));
                }
            } else {
                return Err(syn::Error::new_spanned(
                    meta.path,
                    "unknown sproto attribute",
                ));
            }
            Ok(())
        })
    }
}

/// Field information collected from the struct definition.
#[derive(Clone)]
pub struct FieldInfo {
    pub ident: Ident,
    pub tag: u16,
    pub is_optional: bool,
    pub is_vec: bool,
    pub skip: bool,
    pub use_default: bool,
    pub span: Span,
}

/// Validate that all non-skipped fields have tags and tags are unique.
pub fn validate_fields(fields: &[FieldInfo]) -> Result<()> {
    use std::collections::HashSet;

    let mut seen_tags = HashSet::new();

    for field in fields {
        if field.skip {
            continue;
        }

        if !seen_tags.insert(field.tag) {
            return Err(syn::Error::new(
                field.span,
                format!("duplicate tag {} in struct", field.tag),
            ));
        }
    }

    Ok(())
}
