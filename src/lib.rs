use std::{any::Any, borrow::BorrowMut};

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
use thisenum::Const;
use thiserror::Error;
use hashbrown::HashMap;
use proc_macro2::TokenTree;
use proc_macro::TokenStream;

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

#[derive(Const)]
#[armtype(&str)]
enum KlvAttributes {
    // master attribute
    #[value = "klv"]
    Klv,
    // key encoder / decoder
    #[value = "key_enc"]
    KeyEnc,
    #[value = "key_dec"]
    KeyDec,
    // length encoder / decoder
    #[value = "len_enc"]
    LenEnc,
    #[value = "len_dec"]
    LenDec,
    // default encoder / decoder
    // with type + func
    #[value = "default_enc"]
    DefaultEnc,
    #[value = "default_dec"]
    DefaultDec,
    #[value = "ty"]
    Type,
    #[value = "func"]
    Func,
    // key
    #[value = "key"]
    Key,
    #[value = "len"]
    // length
    Len,
    #[value = "enc"]
    // value encoder / decoder
    Enc,
    #[value = "dec"]
    Dec,
}

#[proc_macro_derive(Klv, attributes(
    klv,
    key_enc,
    key_dec,
    len_enc,
    len_dec,
    default_enc,
    default_dec,
    key,
    len,
    enc,
    dec,
))]
pub fn klv(input: TokenStream) -> TokenStream {
    let name = "Klv";
    let input = parse_macro_input!(input as DeriveInput);
    // --------------------------------------------------
    // extract the name, variants, and values
    // --------------------------------------------------
    let struct_name = &input.ident;
    let fields = match input.data {
        Data::Struct(DataStruct { fields, .. }) => fields,
        _ => panic!("{}", Error::DeriveForNonStruct(name.into())),
    };
    // --------------------------------------------------
    // parse struct-level attributes
    // --------------------------------------------------
    let mut struct_attrs = StructAttrs::default();
    input
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident(KlvAttributes::Klv.value()))
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
                .find(|attr| attr.path.is_ident(KlvAttributes::Klv.value()))
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

#[derive(Default)]
struct StructAttrs {
    key_dec: Option<syn::Path>,
    key_enc: Option<syn::Path>,
    len_dec: Option<syn::Path>,
    len_enc: Option<syn::Path>,
    default_dec: HashMap<std::any::TypeId, syn::Path>,
    default_enc: HashMap<std::any::TypeId, syn::Path>,
}
impl std::fmt::Debug for StructAttrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StructAttrs")
            .field("key_dec", &self.key_dec.as_ref().map_or("None".to_string(), |v| v.to_token_stream().to_string()))
            .field("key_enc", &self.key_enc.as_ref().map_or("None".to_string(), |v| v.to_token_stream().to_string()))
            .field("len_dec", &self.len_dec.as_ref().map_or("None".to_string(), |v| v.to_token_stream().to_string()))
            .field("len_enc", &self.len_enc.as_ref().map_or("None".to_string(), |v| v.to_token_stream().to_string()))
            // .field("default_dec", &self.default_dec.as_ref().map_or("None".to_string(), |(t, p)| format!("type: {}, func: {}", t.to_token_stream().to_string(), p.to_token_stream().to_string())))
            // .field("default_enc", &self.default_enc.as_ref().map_or("None".to_string(), |(t, p)| format!("type: {}, func: {}", t.to_token_stream().to_string(), p.to_token_stream().to_string())))
            .finish()
    }
}

fn parse_struct_attr(attr: &Attribute, struct_attrs: &mut StructAttrs) {
    match attr.parse_meta() {
        Ok(meta) => match meta {
            Meta::List(meta) => for nested in meta.nested {
                if let NestedMeta::Meta(Meta::NameValue(mnv)) = nested {
                    match mnv
                        .path
                        .get_ident()
                        .map(|id| id.to_string())
                    {
                        Some(val) => match match val.as_str() {
                            s if KlvAttributes::KeyDec == s => KlvAttributes::KeyDec,
                            s if KlvAttributes::KeyEnc == s => KlvAttributes::KeyEnc,
                            s if KlvAttributes::LenDec == s => KlvAttributes::LenDec,
                            s if KlvAttributes::LenEnc == s => KlvAttributes::LenEnc,
                            _ => continue,
                        } {
                            KlvAttributes::KeyDec => struct_attrs.key_dec = Some(match &mnv.lit {
                                Lit::Str(lit) => lit.parse().unwrap(),
                                _ => continue,
                            }),
                            KlvAttributes::KeyEnc => struct_attrs.key_enc = Some(match &mnv.lit {
                                Lit::Str(lit) => lit.parse().unwrap(),
                                _ => continue,
                            }),
                            KlvAttributes::LenDec => struct_attrs.len_dec = Some(match &mnv.lit {
                                Lit::Str(lit) => lit.parse().unwrap(),
                                _ => continue,
                            }),
                            KlvAttributes::LenEnc => struct_attrs.len_enc = Some(match &mnv.lit {
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
                        let wattr = match ident.to_string().as_str() {
                            s if KlvAttributes::DefaultDec.value() == s => KlvAttributes::DefaultDec,
                            s if KlvAttributes::DefaultEnc.value() == s => KlvAttributes::DefaultEnc,
                            _ => return,
                        };
                        let tree = group.stream().into_iter().nth(1).unwrap();
                        let _ = match parse_xcoder_attribute(tree) {
                            Some((ty, func)) => match wattr {
                                KlvAttributes::DefaultDec => struct_attrs.default_dec.insert(ty.type_id(), func),
                                KlvAttributes::DefaultEnc => struct_attrs.default_enc.insert(ty.type_id(), func),
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
    match (parts.get(KlvAttributes::Type.value()), parts.get(KlvAttributes::Func.value())) {
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

#[derive(Default)]
struct FieldAttrs {
    name: Option<syn::Ident>,
    typ: Option<Type>,
    key: Option<Vec<u8>>,
    len: Option<usize>,
    dec: Option<syn::Path>,
    enc: Option<syn::Path>,
}
impl std::fmt::Debug for FieldAttrs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldAttrs")
            .field("name", &self.name)
            .field("typ", &self.typ.to_token_stream().to_string())
            .field("key", &self.key)
            .field("len", &self.len)
            .field("dec", &self.dec.to_token_stream().to_string())
            .field("enc", &self.enc.to_token_stream().to_string())
            .finish()
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
                    Some(val) => match match val.as_str() {
                        s if KlvAttributes::Key.value() == s => KlvAttributes::Key,
                        s if KlvAttributes::Len.value() == s => KlvAttributes::Len,
                        s if KlvAttributes::Dec.value() == s => KlvAttributes::Dec,
                        s if KlvAttributes::Enc.value() == s => KlvAttributes::Enc,
                        _ => return,
                    } {
                        KlvAttributes::Key => field_attrs.key = Some(match &mnv.lit {
                            Lit::ByteStr(lit) => lit.value(),
                            _ => continue,
                        }),
                        KlvAttributes::Len => field_attrs.len = Some(match &mnv.lit {
                            Lit::Int(lit) => lit.base10_parse().unwrap(),
                            _ => continue,
                        }),
                        KlvAttributes::Dec => field_attrs.dec = Some(match &mnv.lit {
                            Lit::Str(lit) => lit.parse().unwrap(),
                            _ => continue,
                        }),
                        KlvAttributes::Enc => field_attrs.enc = Some(match &mnv.lit {
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