// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{
    quote,
    ToTokens,
};
use syn::{
    Lit,
    Meta,
    Data,
    Type,
    Path,
    Attribute,
    DataStruct,
    NestedMeta,
    DeriveInput,
    parse_macro_input,
};
use std::any::Any;
use thiserror::Error;
use hashbrown::HashMap;
use proc_macro2::TokenTree;
use proc_macro::TokenStream;

// --------------------------------------------------
// local
// --------------------------------------------------
mod klv;
use klv::*;
mod types;
use tinyklv_common::prelude;

#[derive(Error, Debug)]
enum Error {
    #[error("`{0}` can only be derived for structs")]
    DeriveForNonStruct(String),
    // #[error("Missing #[armtype = ...] attribute {0}, required for `{1}`-derived enum")]
    // MissingArmType(String, String),
    // #[error("Missing #[value = ...] attribute, expected for `{0}`-derived enum")]
    // MissingValue(String),
    // #[error("Attemping to parse non-literal attribute for `value`: not yet supported")]
    // NonLiteralValue,
}

#[proc_macro_derive(Klv, attributes(
    klv,
    key_encoder,
    key_decoder,
    len_encoder,
    len_decoder,
    default_encoder,
    default_decoder,
    // ty,
    // func,
    // fixed,
    key,
    len,
    encoder,
    decoder
))]
pub fn klv(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // --------------------------------------------------
    // extract the name, variants, and values
    // --------------------------------------------------
    let struct_name = &input.ident;
    let fields = match input.data {
        Data::Struct(DataStruct { fields, .. }) => fields,
        _ => panic!("{}", Error::DeriveForNonStruct(KlvStructAttributes::Klv.value().into())),
    };
    // --------------------------------------------------
    // parse struct-level attributes
    // --------------------------------------------------
    let mut struct_attrs = StructAttrs::default();
    input
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident(KlvStructAttributes::Klv.value()))
        .for_each(|attr| parse_struct_attr(attr, &mut struct_attrs));
    // --------------------------------------------------
    // parse field-level attributes
    // --------------------------------------------------
    let mut field_attrs: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let mut attrs = FieldAttrs::default();
            field
                .attrs
                .iter()
                .find(|attr| attr.path.is_ident(KlvStructAttributes::Klv.value()))
                .map(|attr| parse_field_attr(attr, &mut attrs));
            attrs.name = field.ident.clone();
            attrs.typ = Some(field.ty.clone());
            Some(attrs)
        })
        .collect();
    // --------------------------------------------------
    // loop through fields
    // --------------------------------------------------
    field_attrs
        .iter_mut()
        .for_each(|field_attr| {
            // --------------------------------------------------
            // if field type has no decoder/encoder, AND the
            // default decoder/encoder exists for that type
            // within struct_attrs, use that
            // --------------------------------------------------
            if field_attr.enc.is_none() {
                match field_attr.typ.as_ref() {
                    Some(typ) => match struct_attrs.default_enc.get(&typ.type_id()) {
                        Some(func) => field_attr.enc = Some(func.clone()),
                        None => (),
                    },
                    None => (),
                }
            }
            if field_attr.dec.is_none() {
                match field_attr.typ.as_ref() {
                    Some(typ) => match struct_attrs.default_dec.get(&typ.type_id()) {
                        Some(func) => field_attr.dec = Some(func.clone()),
                        None => (),
                    },
                    None => (),
                }
            }
        });
    // --------------------------------------------------
    // debug
    // --------------------------------------------------
    println!("{:#?}", struct_attrs);
    println!("{:#?}", field_attrs);
    // --------------------------------------------------
    // generate code
    // --------------------------------------------------
    // let expanded = quote! {
    //     // #[derive(Clone, Copy)]
    //     // #input
    //     // #field_attrs
    //     // #struct_attrs
    // };
    // TokenStream::from(expanded)
    unimplemented!()
}

fn parse_struct_attr(attr: &Attribute, struct_attrs: &mut StructAttrs) {
    match attr.parse_meta() {
        Ok(meta) => println!("meta: {:#?}", meta.to_token_stream().to_string()),
        Err(_) => println!("attr: {:#?}", attr.to_token_stream().to_string()),
    }
    match attr.parse_meta() {
        Ok(meta) => match meta {
            Meta::List(meta) => for nested in meta.nested {
                if let NestedMeta::Meta(Meta::NameValue(mnv)) = nested {
                    match mnv
                        .path
                        .get_ident()
                        .map(|id| id.to_string())
                    {
                        Some(val) => match if let Ok(val) = KlvStructAttributes::try_from(val.as_str()) { val } else { continue } {
                            KlvStructAttributes::KeyDec => struct_attrs.key_dec = Some(match &mnv.lit {
                                Lit::Str(lit) => lit.parse().unwrap(),
                                _ => continue,
                            }),
                            KlvStructAttributes::KeyEnc => struct_attrs.key_enc = Some(match &mnv.lit {
                                Lit::Str(lit) => lit.parse().unwrap(),
                                _ => continue,
                            }),
                            KlvStructAttributes::LenDec => struct_attrs.len_dec = Some(match &mnv.lit {
                                Lit::Str(lit) => lit.parse().unwrap(),
                                _ => continue,
                            }),
                            KlvStructAttributes::LenEnc => struct_attrs.len_enc = Some(match &mnv.lit {
                                Lit::Str(lit) => lit.parse().unwrap(),
                                _ => continue,
                            }),
                            _ => continue,
                        }
                        None => continue,
                    }
                }
            }
            _ => return,
        },
        Err(_) => {
            if let Some(meta) = attr.tokens.clone().into_iter().next() {
                if let TokenTree::Group(group) = meta {
                    if let Some(inner) = group.stream().into_iter().next() {
                        let ident = match inner {
                            TokenTree::Ident(ident) => ident,
                            _ => return,
                        };
                        let wattr = match KlvStructAttributes::try_from(ident.to_string().as_str()) {
                            Ok(wattr) => wattr,
                            _ => return,
                        };
                        let tree = group.stream().into_iter().nth(1).unwrap();
                        let _ = match parse_xcoder_attribute(tree) {
                            Some((ty, func)) => match wattr {
                                KlvStructAttributes::DefaultDec => struct_attrs.default_dec.insert(ty.type_id(), func),
                                KlvStructAttributes::DefaultEnc => struct_attrs.default_enc.insert(ty.type_id(), func),
                                _ => None,
                            },
                            None => return,
                        };
                    }
                }
            }
        },
    };
}

fn parse_xcoder_attribute(inner: impl ToString) -> Option<(Type, Path)> {
    let parts = split_inner_xcoder_attribute(inner)?;
    match (parts.get(KlvXcoderArguments::Type.value()), parts.get(KlvXcoderArguments::Func.value())) {
        (Some(ty), Some(func)) => {
            let ty: Type = syn::parse_str(ty.trim_matches('"')).ok()?;
            let func: Path = syn::parse_str(func.trim_matches('"')).ok()?;
            Some((ty, func))
        },
        _ => None
    }
}

fn split_inner_xcoder_attribute(inner: impl ToString) -> Option<HashMap<String, String>> {
    let parts: HashMap<String, String> = inner
        .to_string()
        .trim_matches('(')
        .trim_matches(')')
        .split(',')
        .filter_map(|part| {
            let mut split = part.splitn(2, '=');
            match (split.next(), split.next()) {
                (Some(key), Some(value)) => Some((key.trim().to_string(), value.trim().to_string())),
                _ => None
            }
        })
        .collect();
    match parts.is_empty() {
        true => None,
        false => Some(parts),
    }
}

fn parse_field_attr(attr: &Attribute, field_attrs: &mut FieldAttrs) {
    if let Ok(Meta::List(meta)) = attr.parse_meta() {
        for nested in meta.nested {
            if let NestedMeta::Meta(Meta::NameValue(mnv)) = nested {
                match mnv
                    .path
                    .get_ident()
                    .map(|id| id.to_string())
                {
                    Some(val) => match if let Ok(val) = KlvFieldAttributes::try_from(val.as_str()) { val } else { continue } {
                        KlvFieldAttributes::Key => field_attrs.key = Some(match &mnv.lit {
                            Lit::ByteStr(lit) => lit.value(),
                            _ => continue,
                        }),
                        KlvFieldAttributes::Len => field_attrs.len = Some(match &mnv.lit {
                            Lit::Int(lit) => lit.base10_parse().unwrap(),
                            _ => continue,
                        }),
                        KlvFieldAttributes::Dec => field_attrs.dec = Some(match &mnv.lit {
                            Lit::Str(lit) => lit.parse().unwrap(),
                            _ => continue,
                        }),
                        KlvFieldAttributes::Enc => field_attrs.enc = Some(match &mnv.lit {
                            Lit::Str(lit) => lit.parse().unwrap(),
                            _ => continue,
                        }),
                        _ => continue,
                    }
                    None => continue,
                }
            }
        }
    }
}