// --------------------------------------------------
// external
// --------------------------------------------------
use thisenum::Const;
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

#[derive(Const)]
#[armtype(&str)]
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

pub struct OptionalXcoder {
    enc: Option<syn::Path>,
    dec: Option<syn::Path>,
}

pub struct DefaultsXcoder {
    ty: syn::Type,
    xcoder: OptionalXcoder,
}

struct StructAttrSchema {
    stream: NameValue<syn::Type>,
    sentinel: Option<NameValue<syn::Lit>>,
    key: Tuple<RequiredXcoder>,
    len: Tuple<RequiredXcoder>,
    defaults: HashSet<Tuple<DefaultsXcoder>>
}

struct FieldAttrSchema {
    key: NameValue<syn::Lit>,
    len: NameValue<syn::Lit>,
    dec: Option<NameValue<syn::Path>>,
    enc: Option<NameValue<syn::Path>>,
}