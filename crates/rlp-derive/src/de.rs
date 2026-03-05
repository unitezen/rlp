use crate::utils::{
    attributes_include, field_ident, is_optional, make_generics, parse_field_attrs, parse_struct,
    parse_struct_attrs, EMPTY_STRING_CODE,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Result};

pub(crate) fn impl_decodable(ast: &syn::DeriveInput) -> Result<TokenStream> {
    let body = parse_struct(ast, "RlpDecodable")?;
    let struct_attrs = parse_struct_attrs(ast)?;

    let fields = body
        .fields
        .iter()
        .enumerate()
        .map(|(index, field)| Ok((index, field, parse_field_attrs(field)?)))
        .collect::<Result<Vec<_>>>()?;

    let supports_trailing_opt = attributes_include(&ast.attrs, "trailing");

    let mut encountered_opt_item = false;
    let mut decode_stmts = Vec::with_capacity(body.fields.len());
    for (i, field, attrs) in fields {
        let is_opt = is_optional(field);
        if is_opt {
            if !supports_trailing_opt {
                let msg = "optional fields are disabled.\nAdd the `#[rlp(trailing)]` attribute to the struct in order to enable optional fields";
                return Err(Error::new_spanned(field, msg));
            }
            encountered_opt_item = true;
        } else if encountered_opt_item && !attrs.default {
            let msg =
                "all the fields after the first optional field must be either optional or default";
            return Err(Error::new_spanned(field, msg));
        }

        decode_stmts.push(decodable_field(i, field, &attrs, is_opt, struct_attrs.nolist));
    }

    let name = &ast.ident;
    let generics = make_generics(&ast.generics, quote!(alloy_rlp::Decodable));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let pre_decode = struct_attrs.pre_decode_with.map(|path| quote! { #path(b)?; });
    let post_decode = struct_attrs.post_decode_with.map(|path| quote! { #path(b)?; });
    let decode_body = if struct_attrs.nolist {
        quote! {
            let this = Self {
                #(#decode_stmts)*
            };

            #post_decode

            Ok(this)
        }
    } else {
        quote! {
            let alloy_rlp::Header { list, payload_length } = alloy_rlp::Header::decode(b)?;
            if !list {
                return Err(alloy_rlp::Error::UnexpectedString);
            }

            let started_len = b.len();
            if started_len < payload_length {
                return Err(alloy_rlp::DecodeError::InputTooShort);
            }

            let this = Self {
                #(#decode_stmts)*
            };

            let consumed = started_len - b.len();
            if consumed != payload_length {
                return Err(alloy_rlp::Error::ListLengthMismatch {
                    expected: payload_length,
                    got: consumed,
                });
            }

            #post_decode

            Ok(this)
        }
    };

    Ok(quote! {
        const _: () = {
            extern crate alloy_rlp;

            impl #impl_generics alloy_rlp::Decodable for #name #ty_generics #where_clause {
                #[inline]
                fn decode(b: &mut &[u8]) -> alloy_rlp::Result<Self> {
                    #pre_decode

                    #decode_body
                }
            }
        };
    })
}

pub(crate) fn impl_decodable_wrapper(ast: &syn::DeriveInput) -> Result<TokenStream> {
    let body = parse_struct(ast, "RlpEncodableWrapper")?;

    if body.fields.iter().count() != 1 {
        let msg = "`RlpEncodableWrapper` is only defined for structs with one field.";
        return Err(Error::new(ast.ident.span(), msg));
    }

    let name = &ast.ident;
    let generics = make_generics(&ast.generics, quote!(alloy_rlp::Decodable));
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    Ok(quote! {
        const _: () = {
            extern crate alloy_rlp;

            impl #impl_generics alloy_rlp::Decodable for #name #ty_generics #where_clause {
                #[inline]
                fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
                    alloy_rlp::private::Result::map(alloy_rlp::Decodable::decode(buf), Self)
                }
            }
        };
    })
}

fn decodable_field(
    index: usize,
    field: &syn::Field,
    attrs: &crate::utils::FieldAttrs,
    is_opt: bool,
    nolist: bool,
) -> TokenStream {
    let ident = field_ident(index, field);
    let decoded = attrs
        .with
        .as_ref()
        .map(|with| quote! { #with::decode })
        .unwrap_or_else(|| quote! { alloy_rlp::Decodable::decode });

    if attrs.default {
        quote! { #ident: alloy_rlp::private::Default::default(), }
    } else if is_opt {
        if nolist {
            quote! {
                #ident: if !b.is_empty() {
                    if alloy_rlp::private::Option::map_or(b.first(), false, |b| *b == #EMPTY_STRING_CODE) {
                        alloy_rlp::Buf::advance(b, 1);
                        None
                    } else {
                        Some(#decoded(b)?)
                    }
                } else {
                    None
                },
            }
        } else {
            quote! {
                #ident: if started_len - b.len() < payload_length {
                    if alloy_rlp::private::Option::map_or(b.first(), false, |b| *b == #EMPTY_STRING_CODE) {
                        alloy_rlp::Buf::advance(b, 1);
                        None
                    } else {
                        Some(#decoded(b)?)
                    }
                } else {
                    None
                },
            }
        }
    } else {
        quote! { #ident: #decoded(b)?, }
    }
}
