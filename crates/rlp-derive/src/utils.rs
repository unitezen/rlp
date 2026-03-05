use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse_quote, Attribute, DataStruct, Error, Field, GenericParam, Generics, Meta, Result, Type,
    TypePath,
};

pub(crate) const EMPTY_STRING_CODE: u8 = 0x80;

#[derive(Clone, Default)]
pub(crate) struct FieldAttrs {
    pub(crate) default: bool,
    pub(crate) skip: bool,
    pub(crate) with: Option<syn::Path>,
}

#[derive(Clone, Default)]
pub(crate) struct StructAttrs {
    pub(crate) pre_encode_with: Option<syn::Path>,
    pub(crate) post_encode_with: Option<syn::Path>,
    pub(crate) pre_decode_with: Option<syn::Path>,
    pub(crate) post_decode_with: Option<syn::Path>,
}

pub(crate) fn parse_struct<'a>(
    ast: &'a syn::DeriveInput,
    derive_attr: &str,
) -> Result<&'a DataStruct> {
    if let syn::Data::Struct(s) = &ast.data {
        Ok(s)
    } else {
        Err(Error::new_spanned(
            ast,
            format!("#[derive({derive_attr})] is only defined for structs."),
        ))
    }
}

pub(crate) fn attributes_include(attrs: &[Attribute], attr_name: &str) -> bool {
    for attr in attrs {
        if attr.path().is_ident("rlp") {
            if let Meta::List(meta) = &attr.meta {
                let mut found = false;
                let _ = meta.parse_nested_meta(|meta| {
                    if meta.path.is_ident(attr_name) {
                        found = true;
                    }
                    Ok(())
                });
                if found {
                    return true;
                }
            }
        }
    }
    false
}

pub(crate) fn parse_field_attrs(field: &Field) -> Result<FieldAttrs> {
    let mut attrs = FieldAttrs::default();

    for attr in &field.attrs {
        if !attr.path().is_ident("rlp") {
            continue;
        }

        let Meta::List(meta) = &attr.meta else {
            continue;
        };

        meta.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                attrs.default = true;
                return Ok(());
            }

            if meta.path.is_ident("skip") {
                attrs.skip = true;
                return Ok(());
            }

            if meta.path.is_ident("with") {
                let value = meta.value()?;
                let path = value.parse::<syn::Path>()?;
                if attrs.with.replace(path).is_some() {
                    return Err(meta.error("duplicate `with` argument"));
                }
            }

            Ok(())
        })?;
    }

    Ok(attrs)
}

pub(crate) fn parse_struct_attrs(ast: &syn::DeriveInput) -> Result<StructAttrs> {
    let mut attrs = StructAttrs::default();

    for attr in &ast.attrs {
        if !attr.path().is_ident("rlp") {
            continue;
        }

        let Meta::List(meta) = &attr.meta else {
            continue;
        };

        meta.parse_nested_meta(|meta| {
            if meta.path.is_ident("pre_encode_with") {
                let value = meta.value()?;
                let path = value.parse::<syn::Path>()?;
                if attrs.pre_encode_with.replace(path).is_some() {
                    return Err(meta.error("duplicate `pre_encode_with` argument"));
                }
            } else if meta.path.is_ident("post_encode_with") {
                let value = meta.value()?;
                let path = value.parse::<syn::Path>()?;
                if attrs.post_encode_with.replace(path).is_some() {
                    return Err(meta.error("duplicate `post_encode_with` argument"));
                }
            } else if meta.path.is_ident("pre_decode_with") {
                let value = meta.value()?;
                let path = value.parse::<syn::Path>()?;
                if attrs.pre_decode_with.replace(path).is_some() {
                    return Err(meta.error("duplicate `pre_decode_with` argument"));
                }
            } else if meta.path.is_ident("post_decode_with") {
                let value = meta.value()?;
                let path = value.parse::<syn::Path>()?;
                if attrs.post_decode_with.replace(path).is_some() {
                    return Err(meta.error("duplicate `post_decode_with` argument"));
                }
            }

            Ok(())
        })?;
    }

    Ok(attrs)
}

pub(crate) fn is_optional(field: &Field) -> bool {
    if let Type::Path(TypePath { qself, path }) = &field.ty {
        qself.is_none()
            && path.leading_colon.is_none()
            && path.segments.len() == 1
            && path.segments.first().unwrap().ident == "Option"
    } else {
        false
    }
}

pub(crate) fn field_ident(index: usize, field: &syn::Field) -> TokenStream {
    field.ident.as_ref().map_or_else(
        || {
            let index = syn::Index::from(index);
            quote! { #index }
        },
        |ident| quote! { #ident },
    )
}

pub(crate) fn make_generics(generics: &Generics, trait_name: TokenStream) -> Generics {
    let mut generics = generics.clone();
    generics.make_where_clause();
    let mut where_clause = generics.where_clause.take().unwrap();

    for generic in &generics.params {
        if let GenericParam::Type(ty) = &generic {
            let t = &ty.ident;
            let pred = parse_quote!(#t: #trait_name);
            where_clause.predicates.push(pred);
        }
    }
    generics.where_clause = Some(where_clause);
    generics
}
