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

#[derive(Eq, Hash, Clone, PartialEq)]
/// [`OptionalXcoder`]
/// 
/// This is an encoder/decoder pair where **either** is optional.
pub(crate) struct OptionalXcoder {
    pub enc: Option<syn::Path>,
    pub dec: Option<syn::Path>,
}
/// [`OptionalXcoder`] implementation of [`From`] for [`MetaContents`]
impl From<MetaContents> for OptionalXcoder {
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
            if let MetaItem::Tuple(x) = val {
                return OptionalXcoder::from(x.contents.clone());
            }
        }
        OptionalXcoder {
            enc,
            dec,
        }
    }
}
/// [OptionalXcoder] implementation of [`std::fmt::Display`]
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

// #[derive(Clone)]
// /// [`RequiredXcoder`]
// /// 
// /// This is an encoder/decoder pair where both are required
// pub(crate) struct RequiredXcoder {
//     pub enc: syn::Path,
//     pub dec: syn::Path,
// }
// /// [`RequiredXcoder`] implementation of [`From`] for [`OptionalXcoder`]
// impl From<OptionalXcoder> for RequiredXcoder {
//     fn from(x: OptionalXcoder) -> Self {
//         let oxcoder = OptionalXcoder::from(x);
//         if oxcoder.enc.is_none() { panic!("{}", crate::Error::MissingEncoder) }
//         if oxcoder.dec.is_none() { panic!("{}", crate::Error::MissingDecoder) }
//         RequiredXcoder {
//             enc: oxcoder.enc.unwrap(),
//             dec: oxcoder.dec.unwrap(),
//         }
//     }
// }
// /// [`RequiredXcoder`] implementation of [`From`] for [`MetaContents`]
// impl From<MetaContents> for RequiredXcoder {
//     fn from(x: MetaContents) -> Self {
//         OptionalXcoder::from(x).into()
//     }
// }
// /// [`RequiredXcoder`] implementation of [`std::fmt::Display`]
// impl std::fmt::Display for RequiredXcoder {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "enc: {}, dec: {}", self.enc.to_token_stream().to_string(), self.dec.to_token_stream().to_string())
//     }
// }
// // symple::debug_from_display!(RequiredXcoder);
// tinyklv_common::debug_from_display!(RequiredXcoder);


#[derive(Clone)]
/// [`KeyLenXcoder`]
/// 
/// An encoder/decoder pair for keys/lengths
pub(crate) struct KeyLenXcoder {
    pub xcoder: OptionalXcoder,
}
/// [`KeyLenXcoder`] implementation of [`From`] for [`MetaContents`]
impl From<MetaContents> for KeyLenXcoder {
    fn from(x: MetaContents) -> Self {
        KeyLenXcoder { xcoder: OptionalXcoder::from(x) }
    }
}
/// [`KeyLenXcoder`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for KeyLenXcoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.xcoder)
    }
}
// symple::debug_from_display!(KeyLenXcoder);
tinyklv_common::debug_from_display!(KeyLenXcoder);

#[derive(Eq, Hash, Clone, PartialEq)]
/// [`DefaultXcoder`]
/// 
/// This is an optional encoder/decoder pair
/// that is defaultly used for all associated
/// types without a specified encoder/decoder.
pub(crate) struct DefaultXcoder {
    pub ty: syn::Type,
    pub dynlen: Option<bool>,
    pub enc: Option<syn::Path>,
    pub dec: Option<syn::Path>,
}
/// [`DefaultXcoder`] implementation of [`From`] for [`MetaContents`]
impl From<MetaContents> for DefaultXcoder {
    fn from(x: MetaContents) -> Self {
        let oxcoder = OptionalXcoder::from(x.clone());
        let mut ty: Option<syn::Type> = None;
        let mut dynlen = None;
        for val in x.into_iter() {
            // --------------------------------------------------
            // if all are set, stop
            // --------------------------------------------------
            if ty.is_some() && dynlen.is_some() { break; }
            if let MetaItem::NameValue(x) = val {
                match XcoderNames::try_from(x.name.to_string().as_str()) {
                    Ok(XcoderNames::Type) => ty = Some(x.value.clone().into()),
                    Ok(XcoderNames::DynLen) => dynlen = if let symple::MetaValue::Lit(syn::Lit::Bool(syn::LitBool { value: v, .. })) = x.value { Some(v) } else { None },
                    _ => continue,
                }
            }
        }
        if ty.is_none() { panic!("{}", crate::Error::MissingType) }
        DefaultXcoder {
            ty: ty.unwrap(),
            dynlen,
            enc: oxcoder.enc,
            dec: oxcoder.dec,
        }
    }
}
/// [`DefaultXcoder`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for DefaultXcoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ty: {}, dyn: {:?}, enc: {}, dec: {}",
            self.ty.to_token_stream().to_string(),
            self.dynlen,
            self.enc.to_token_stream().to_string(),
            self.dec.to_token_stream().to_string(),
        )
    }
}
// symple::debug_from_display!(DefaultXcoder);
tinyklv_common::debug_from_display!(DefaultXcoder);

#[derive(Eq, Hash, Clone, Default, PartialEq)]
/// [`ValueXcoder`]
/// 
/// This is an optional encoder/decoder pair
/// that is associated to a specific value
pub(crate) struct ValueXcoder {
    pub dynlen: Option<bool>,
    pub enc: Option<syn::Path>,
    pub dec: Option<syn::Path>,
}
/// [`ValueXcoder`] implementation of [`From`] for [`MetaContents`]
impl From<MetaContents> for ValueXcoder {
    fn from(x: MetaContents) -> Self {
        let oxcoder = OptionalXcoder::from(x.clone());
        let mut dynlen = None;
        for val in x.into_iter() {
            // --------------------------------------------------
            // if all are set, stop
            // --------------------------------------------------
            if dynlen.is_some() { break; }
            if let MetaItem::NameValue(x) = val {
                match XcoderNames::try_from(x.name.to_string().as_str()) {
                    Ok(XcoderNames::DynLen) => dynlen = if let symple::MetaValue::Lit(syn::Lit::Bool(syn::LitBool { value: v, .. })) = x.value { Some(v) } else { None },
                    _ => continue,
                }
            }
        }
        ValueXcoder {
            dynlen,
            enc: oxcoder.enc,
            dec: oxcoder.dec,
        }
    }
}
/// [`ValueXcoder`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for ValueXcoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dyn: {:?}, enc: {}, dec: {}",
            self.dynlen,
            self.enc.to_token_stream().to_string(),
            self.dec.to_token_stream().to_string(),
        )
    }
}
// symple::debug_from_display!(ValueXcoder);
tinyklv_common::debug_from_display!(ValueXcoder);