use quote::ToTokens;
// --------------------------------------------------
// external
// --------------------------------------------------
use thisenum::Const;
use thiserror::Error;
use hashbrown::HashSet;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast2::{
    Tuple,
    NameValue,
    MetaTuple,
    MetaNameValue,
    MetaUnorderedContents,
};
use super::ATTR;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("Missing attribute")]
    Missing
}

pub(crate) struct Input {
    pub name: syn::Ident,
    pub sattr: StructAttrSchema,
    pub fattrs: Vec<FieldAttrSchema>,
}

/// [`Input`] implementation
impl Input {
    pub fn from_syn(input: &syn::DeriveInput) -> Result<Self, Error> {
        // --------------------------------------------------
        // extract the name, variants, and values
        // --------------------------------------------------
        let name = input.ident.clone();
        let fields = match &input.data {
            syn::Data::Struct(syn::DataStruct { fields, .. }) => fields,
            _ => panic!("{}", crate::Error::DeriveForNonStruct(crate::NAME.into(), name.to_string())),
        };
        let sattr = StructAttrSchema::from_syn(&input);
        // let fattrs = FieldAttrSchema::from_syn(&fields);
        // Self { name, sattr, fattrs }.update().verify()
        Err(Error::Missing)
    }
}

#[derive(Default)]
pub(crate) struct StructAttrSchema {
    stream: NameValue<syn::Type>,
    sentinel: Option<NameValue<syn::Lit>>,
    key: Tuple<RequiredXcoder>,
    len: Tuple<RequiredXcoder>,
    // defaults: HashSet<Tuple<DefaultXcoder>>
}
/// [`StructAttrSchema`] implementation
impl StructAttrSchema {
    pub fn from_syn(input: &syn::DeriveInput) -> Self {
        // let mut me = StructAttrSchema::default();
        // println!(" input: {:#?}", input.data);
        input
            .attrs
            .iter()
            .filter(|attr|
                match attr.path.get_ident() {
                    Some(ident) => {
                        println!("i am true");
                        println!("ident: {:#?}", ident.to_string());
                        println!("tokens: {:#?}", attr.tokens.to_string());
                        StructNames::try_from(ident.to_string().as_str()).is_ok()
                    },
                    None => {
                        println!("i am false");
                        false
                    },
                }
            )
            .for_each(|attr| {
                println!("attr: {:#?}", attr.to_token_stream().to_string());
                // Self::parse_struct_attr(attr, &mut me)
            });
        // me
        panic!("not implemented")
    }
}

#[derive(Default)]
pub(crate)struct FieldAttrSchema {
    key: NameValue<syn::Lit>,
    len: NameValue<syn::Lit>,
    dec: Option<NameValue<syn::Path>>,
    enc: Option<NameValue<syn::Path>>,
}

#[derive(Const)]
#[armtype(&str)]
/// Struct Attribute Names
/// 
/// # Arms
/// 
/// * `stream` - The stream type. Defaults to [`u8`]
/// * `sentinel` - The sentinel value. Defaults to `None`
/// * `key` - The key xcoder tuple
/// * `len` - The length xcoder tuple
/// * `default` - The default xcoder tuple
enum StructNames {
    #[value = "stream"]
    Stream,
    #[value = "sentinel"]
    Sentinel,
    #[value = "key"]
    KeyTuple,
    #[value = "len"]
    LengthTuple,
    #[value = "default"]
    DefaultTuple,
}

#[derive(Const)]
#[armtype(&str)]
/// Field Attribute Names
/// 
/// # Arms
/// 
/// * `key` - The key, as a slice of `stream` type (usually bytes)
/// * `len` - The length, as [`usize`]
/// * `enc` - The encoder
/// * `dec` - The decoder
enum FieldNames {
    #[value = "key"]
    Key,
    #[value = "len"]
    Length,
    #[value = "enc"]
    Encoder,
    #[value = "dec"]
    Decoder,
}

/// [`RequiredXcoder`]
/// 
/// This is an encoder/decoder pair
/// which is **not** optional.
pub struct RequiredXcoder {
    enc: syn::Path,
    dec: syn::Path,
}
/// [`MetaTuple`] implementation of [`From`] for [`RequiredXcoder`]
impl From<MetaTuple> for RequiredXcoder {
    fn from(x: MetaTuple) -> Self {
        let mut enc: Option<syn::Path> = None;
        let mut dec: Option<syn::Path> = None;
        for val in x.v.nvs {
            // --------------------------------------------------
            // if both are set, stop
            // --------------------------------------------------
            if enc.is_some() && dec.is_some() { break; }
            match FieldNames::try_from(val.n.to_string().as_str()) {
                Ok(FieldNames::Encoder) => enc = Some(syn::parse_str::<syn::Path>(val.v.to_string().as_str()).unwrap()),
                Ok(FieldNames::Decoder) => dec = Some(syn::parse_str::<syn::Path>(val.v.to_string().as_str()).unwrap()),
                _ => {}
            }
        }
        if enc.is_none() { panic!("Missing encoder") }
        if dec.is_none() { panic!("Missing decoder") }
        RequiredXcoder {
            enc: enc.unwrap(),
            dec: dec.unwrap(),
        }
    }
}

/// [`OptionalXcoder`]
/// 
/// This is an encoder/decoder pair
/// where **either** is optional.
pub struct OptionalXcoder {
    enc: Option<syn::Path>,
    dec: Option<syn::Path>,
}

/// [`DefaultXcoder`]
/// 
/// This is an optional encoder/decoder pair
/// that is defaultly used for all associated
/// types without a specified encoder/decoder.
pub struct DefaultXcoder {
    ty: syn::Type,
    xcoder: OptionalXcoder,
}