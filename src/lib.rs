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
    Attribute,
    DataStruct,
    NestedMeta,
    DeriveInput,
    MetaNameValue,
    parse_macro_input,
};
use thisenum::Const;
use thiserror::Error;
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
    #[value = "klv"]
    Klv,
    #[value = "key_enc"]
    KeyEnc,
    #[value = "key_dec"]
    KeyDec,
    #[value = "len_enc"]
    LenEnc,
    #[value = "len_dec"]
    LenDec,
    #[value = "default_enc"]
    DefaultEnc,
    #[value = "default_dec"]
    DefaultDec,
    #[value = "key"]
    Key,
    #[value = "len"]
    Len,
    #[value = "enc"]
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
    // extract the type
    // --------------------------------------------------
    // let expanded = impl_klv(&input);
    // TokenStream::from(expanded)
    unimplemented!()
}

#[derive(Default)]
struct StructAttrs {
    key_dec: Option<syn::Path>,
    key_enc: Option<syn::Path>,
    len_dec: Option<syn::Path>,
    len_enc: Option<syn::Path>,
    default_dec: Option<(syn::Type, syn::Path)>,
    default_enc: Option<(syn::Type, syn::Path)>,
}

fn parse_struct_attr(attr: &Attribute, struct_attrs: &mut StructAttrs) {
    if let Ok(Meta::List(meta)) = attr.parse_meta() {
        for nested in meta.nested {
            if let NestedMeta::Meta(Meta::NameValue(mnv)) = nested {
                match mnv.path.get_ident().map(|id| id.to_string().as_str()) {
                    Some(val) => {
                        match val {
                            s if KlvAttributes::KeyDec == s => {
                                if let Lit::Str(lit) = &mnv.lit {
                                    struct_attrs.key_dec = Some(lit.parse().unwrap());
                                }
                            },
                            s if KlvAttributes::KeyEnc == s => {
                                if let Lit::Str(lit) = &mnv.lit {
                                    struct_attrs.key_enc = Some(lit.parse().unwrap());
                                }
                            },
                            s if KlvAttributes::LenDec == s => {
                                if let Lit::Str(lit) = &mnv.lit {
                                    struct_attrs.len_dec = Some(lit.parse().unwrap());
                                }
                            },
                            s if KlvAttributes::LenEnc == s => {
                                if let Lit::Str(lit) = &mnv.lit {
                                    struct_attrs.len_enc = Some(lit.parse().unwrap());
                                }
                            },
                            s if KlvAttributes::DefaultDec == s => {
                                if let Lit::Str(lit) = &mnv.lit {
                                    struct_attrs.default_dec = Some(lit.parse().unwrap());
                                }
                            },
                            s if KlvAttributes::DefaultEnc == s => {
                                if let Lit::Str(lit) = &mnv.lit {
                                    struct_attrs.default_enc = Some(lit.parse().unwrap());
                                }
                            },
                            _ => {},
                        }
                    }
                    None => {},
                }
            }
        }
    }
}

#[derive(Default)]
struct FieldAttrs {
    key: Option<Vec<u8>>,
    len: Option<usize>,
    dec: Option<syn::Path>,
    enc: Option<syn::Path>,
}

fn parse_field_attr(attr: &Attribute, field_attrs: &mut FieldAttrs) {
    if let Ok(Meta::List(meta)) = attr.parse_meta() {
        for nested in meta.nested {
            if let NestedMeta::Meta(Meta::NameValue(mnv)) = nested {
                match mnv.path.get_ident().map(|id| id.to_string().as_str()) {
                    Some(KlvAttributes::Key.value()) => {
                        if let Lit::ByteStr(lit) = &mnv.lit {
                            field_attrs.key = Some(lit.value());
                        }
                    },
                    Some(KlvAttributes::Len) => {
                        if let Lit::Int(lit) = &mnv.lit {
                            field_attrs.len = Some(lit.base10_parse().unwrap());
                        }
                    },
                    Some(KlvAttributes::Dec) => {
                        if let Lit::Str(lit) = &mnv.lit {
                            field_attrs.dec = Some(lit.parse().unwrap());
                        }
                    },
                    Some(KlvAttributes::Enc) => {
                        if let Lit::Str(lit) = &mnv.lit {
                            field_attrs.enc = Some(lit.parse().unwrap());
                        }
                    },
                    _ => {},
                }
            }
        }
    }
}