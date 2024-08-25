// --------------------------------------------------
// external
// --------------------------------------------------
use tinyklv_common::XcoderNames;
use tinyklv_common::symple::{
    self,
    MetaItem,
    MetaContents,
};
use quote::ToTokens;

#[derive(Clone)]
/// [`RequiredXcoder`]
/// 
/// This is an encoder/decoder pair
/// which is **not** optional.
pub struct RequiredXcoder {
    pub enc: syn::Path,
    pub dec: syn::Path,
}
/// [`RequiredXcoder`] implementation of [`From`] for [`MetaContents`]
impl From<MetaContents> for RequiredXcoder {
    fn from(x: MetaContents) -> Self {
        let mut enc: Option<syn::Path> = None;
        let mut dec: Option<syn::Path> = None;
        for val in x.into_iter() {
            // --------------------------------------------------
            // if both are set, stop
            // --------------------------------------------------
            if enc.is_some() && dec.is_some() { break; }
            if let MetaItem::NameValue(x) = val {
                match XcoderNames::try_from(x.name.to_string().as_str()) {
                    Ok(XcoderNames::Encoder) => enc = Some(x.value.clone().into()),
                    Ok(XcoderNames::Decoder) => dec = Some(x.value.clone().into()),
                    _ => continue,
                }
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
/// [`RequiredXcoder`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for RequiredXcoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "enc: {}, dec: {}", self.enc.to_token_stream().to_string(), self.dec.to_token_stream().to_string())
    }
}
// symple::debug_from_display!(RequiredXcoder);
tinyklv_common::debug_from_display!(RequiredXcoder);

#[derive(Eq, Hash, Clone, PartialEq)]
/// [`OptionalXcoder`]
/// 
/// This is an encoder/decoder pair
/// where **either** is optional.
pub(crate) struct OptionalXcoder {
    pub enc: Option<syn::Path>,
    pub dec: Option<syn::Path>,
}
/// [`OptionalXcoder`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for OptionalXcoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let enc_str = match &self.enc {
            Some(x) => x.to_token_stream().to_string(),
            None => "None".to_string(),
        };
        let dec_str = match &self.dec {
            Some(x) => x.to_token_stream().to_string(),
            None => "None".to_string(),
        };
        write!(f, "enc: {}, dec: {}", enc_str, dec_str)
    }
}
// symple::debug_from_display!(OptionalXcoder);
tinyklv_common::debug_from_display!(OptionalXcoder);

#[derive(Eq, Hash, Clone, PartialEq)]
/// [`DefaultXcoder`]
/// 
/// This is an optional encoder/decoder pair
/// that is defaultly used for all associated
/// types without a specified encoder/decoder.
pub(crate) struct DefaultXcoder {
    pub ty: syn::Type,
    pub dynlen: Option<bool>,
    pub xcoder: OptionalXcoder,
}
/// [`DefaultXcoder`] implementation of [`From`] for [`MetaContents`]
impl From<MetaContents> for DefaultXcoder {
    fn from(x: MetaContents) -> Self {
        let mut ty: Option<syn::Type> = None;
        let mut dynlen = None;
        let mut enc: Option<syn::Path> = None;
        let mut dec: Option<syn::Path> = None;
        for val in x.into_iter() {
            // --------------------------------------------------
            // if all are set, stop
            // --------------------------------------------------
            if ty.is_some() && enc.is_some() && dec.is_some() { break; }
            if let MetaItem::NameValue(x) = val {
                match XcoderNames::try_from(x.name.to_string().as_str()) {
                    Ok(XcoderNames::Type) => ty = Some(x.value.clone().into()),
                    Ok(XcoderNames::DynLen) => dynlen = if let symple::MetaValue::Lit(syn::Lit::Bool(syn::LitBool { value: v, .. })) = x.value { Some(v) } else { None },
                    Ok(XcoderNames::Encoder) => enc = Some(x.value.clone().into()),
                    Ok(XcoderNames::Decoder) => dec = Some(x.value.clone().into()),
                    _ => continue,
                }
            }
        }
        if ty.is_none() { panic!("Missing type") }
        DefaultXcoder {
            ty: ty.unwrap(),
            dynlen,
            xcoder: OptionalXcoder { enc, dec },
        }
    }
}
/// [`DefaultXcoder`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for DefaultXcoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ty: {}, dyn: {:?}, {}", self.ty.to_token_stream().to_string(), self.dynlen, self.xcoder)
    }
}
// symple::debug_from_display!(DefaultXcoder);
tinyklv_common::debug_from_display!(DefaultXcoder);