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
mod nonlit2lit;

#[derive(Error, Debug)]
enum Error {
    #[error("`{0}` can only be derived for structs.")]
    DeriveForNonStruct(String),
    #[error("Missing required attribute: function in `#[{0}(func = ?)]`.")]
    MissingFunc(String),
    #[error("Missing required attribute: type in `#[{0}(typ = ?)]`.")]
    MissingType(String),
    // #[error("Attemping to parse non-literal attribute for `value`: not yet supported")]
    // NonLiteralValue,
}

const NAME: &str = "Klv";
#[proc_macro_derive(Klv, attributes(
    key_encoder,
    key_decoder,
    len_encoder,
    len_decoder,
    default_encoder,
    default_decoder,
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
        _ => panic!("{}", Error::DeriveForNonStruct(NAME.into())),
    };
    // --------------------------------------------------
    // parse struct-level attributes
    // --------------------------------------------------
    let mut struct_attrs = KlvStructAttr::default();
    input
        .attrs
        .iter()
        .filter(|attr|
            match attr.path.get_ident() {
                Some(ident) => KlvStructAttrValue::try_from(ident.to_string().as_str()).is_ok(),
                None => false,
            }
        )
        .for_each(|attr| parse_struct_attr(attr, &mut struct_attrs));
    // --------------------------------------------------
    // parse field-level attributes
    // --------------------------------------------------
    let mut field_attrs: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let mut attrs = KlvFieldAttr::default();
            field
                .attrs
                .iter()
                .find(|attr| true)
                // .find(|attr| attr.path.is_ident(KlvStructAttributes::Klv.value()))
                .map(|attr| parse_field_attr(attr, &mut attrs));
            attrs.name = field.ident.clone();
            attrs.typ = Some(field.ty.clone());
            Some(attrs)
        })
        .collect();
    // // --------------------------------------------------
    // // loop through fields
    // // --------------------------------------------------
    // field_attrs
    //     .iter_mut()
    //     .for_each(|field_attr| {
    //         // --------------------------------------------------
    //         // if field type has no decoder/encoder, AND the
    //         // default decoder/encoder exists for that type
    //         // within struct_attrs, use that
    //         // --------------------------------------------------
    //         if field_attr.enc.is_none() {
    //             match field_attr.typ.as_ref() {
    //                 Some(typ) => match struct_attrs.default_enc.get(&typ.type_id()) {
    //                     Some(func) => field_attr.enc = Some(func.clone()),
    //                     None => (),
    //                 },
    //                 None => (),
    //             }
    //         }
    //         if field_attr.dec.is_none() {
    //             match field_attr.typ.as_ref() {
    //                 Some(typ) => match struct_attrs.default_dec.get(&typ.type_id()) {
    //                     Some(func) => field_attr.dec = Some(func.clone()),
    //                     None => (),
    //                 },
    //                 None => (),
    //             }
    //         }
    //     });
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

fn parse_struct_attr(attr: &Attribute, struct_attrs: &mut KlvStructAttr) {
    let sattr = match nonlit2lit::StructAttr::new(attr.to_token_stream().to_string()) {
        Ok(sattr) => sattr,
        _ => return,
    };
    struct_attrs.push(sattr);
}

fn parse_xcoder_attribute(inner: impl ToString) -> Option<(Type, Path)> {
    let parts = split_inner_xcoder_attribute(inner)?;
    match (parts.get(KlvXcoderArgValue::Type.value()), parts.get(KlvXcoderArgValue::Func.value())) {
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

fn parse_field_attr(attr: &Attribute, field_attrs: &mut KlvFieldAttr) {
    if let Ok(Meta::List(meta)) = attr.parse_meta() {
        for nested in meta.nested {
            if let NestedMeta::Meta(Meta::NameValue(mnv)) = nested {
                match mnv
                    .path
                    .get_ident()
                    .map(|id| id.to_string())
                {
                    Some(val) => match if let Ok(val) = KlvFieldAttrValue::try_from(val.as_str()) { val } else { continue } {
                        KlvFieldAttrValue::Key => field_attrs.key = Some(match &mnv.lit {
                            Lit::ByteStr(lit) => lit.value(),
                            _ => continue,
                        }),
                        KlvFieldAttrValue::Len => field_attrs.len = Some(match &mnv.lit {
                            Lit::Int(lit) => lit.base10_parse().unwrap(),
                            _ => continue,
                        }),
                        KlvFieldAttrValue::Dec => field_attrs.dec = Some(match &mnv.lit {
                            Lit::Str(lit) => lit.parse().unwrap(),
                            _ => continue,
                        }),
                        KlvFieldAttrValue::Enc => field_attrs.enc = Some(match &mnv.lit {
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