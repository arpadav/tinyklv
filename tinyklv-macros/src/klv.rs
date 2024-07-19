
// --------------------------------------------------
// external
// --------------------------------------------------
use std::any::Any;
use quote::ToTokens;
use thisenum::Const;
use hashbrown::HashMap;

// --------------------------------------------------
// local
// --------------------------------------------------
use super::nonlit2lit;

#[derive(Const)]
#[armtype(&str)]
pub(crate) enum KlvStructAttrValue {
    // key encoder / decoder
    #[value = "key_encoder"]
    KeyEnc,
    #[value = "key_decoder"]
    KeyDec,
    // length encoder / decoder
    #[value = "len_encoder"]
    LenEnc,
    #[value = "len_decoder"]
    LenDec,
    // default encoder / decoder
    // with type + func + fixed
    #[value = "default_encoder"]
    DefaultEnc,
    #[value = "default_decoder"]
    DefaultDec,
}

#[derive(Default, Debug)]
pub(crate) struct KlvStructAttr {
    pub key_dec: Option<KlvXcoderArg>,
    pub key_enc: Option<KlvXcoderArg>,
    pub len_dec: Option<KlvXcoderArg>,
    pub len_enc: Option<KlvXcoderArg>,
    pub default_dec: HashMap<String, KlvXcoderArg>,
    pub default_enc: HashMap<String, KlvXcoderArg>,
}
/// [`KlvStructAttr`] implementation of [`Push<StructAttr>`]
impl KlvStructAttr {
    pub fn push(&mut self, x: nonlit2lit::StructAttr) {
        match KlvStructAttrValue::try_from(x.path().as_str()) {
            Ok(KlvStructAttrValue::KeyEnc) => self.key_enc = {
                let res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvStructAttrValue::KeyEnc.value().into())) }
                Some(res.deftype())
            },
            Ok(KlvStructAttrValue::KeyDec) => self.key_dec = {
                let res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvStructAttrValue::KeyDec.value().into())) }
                Some(res.deftype())
            },
            Ok(KlvStructAttrValue::LenEnc) => self.len_enc = {
                let res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvStructAttrValue::LenEnc.value().into())) }
                Some(res.deftype())
            },
            Ok(KlvStructAttrValue::LenDec) => self.len_dec = {
                let res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvStructAttrValue::LenDec.value().into())) }
                Some(res.deftype())
            },
            Ok(KlvStructAttrValue::DefaultEnc) => {
                let res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvStructAttrValue::DefaultEnc.value().into())) }
                if res.typ.is_none() { panic!("{}", crate::Error::MissingType(KlvStructAttrValue::DefaultEnc.value().into())) }
                self.default_enc.insert(res.typ.clone().unwrap().to_token_stream().to_string(), res);
            },
            Ok(KlvStructAttrValue::DefaultDec) => {
                let res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvStructAttrValue::DefaultDec.value().into())) }
                if res.typ.is_none() { panic!("{}", crate::Error::MissingType(KlvStructAttrValue::DefaultDec.value().into())) }
                self.default_dec.insert(res.typ.clone().unwrap().to_token_stream().to_string(), res);
            },
            Err(_) => {}
        }
    }
}

#[derive(Const)]
#[armtype(&str)]
pub(crate) enum KlvXcoderArgValue {
    #[value = "typ"]
    Type,
    #[value = "func"]
    Func,
    #[value = "fixed"]
    Fixed,
}
pub(crate) struct KlvXcoderArg {
    pub typ: Option<syn::Type>,
    pub func: Option<syn::Path>,
    pub fixed: bool,
}
impl KlvXcoderArg {
    pub fn deftype(mut self) -> Self {
        match self.typ {
            Some(_) => self,
            None => {
                self.typ = Some(crate::types::u8_slice());
                self
            }
        }
    }
}
impl std::default::Default for KlvXcoderArg {
    fn default() -> Self {
        Self {
            typ: None,
            func: None,
            fixed: false,
        }
    }
}
impl From<Vec<nonlit2lit::KeyValPair>> for KlvXcoderArg {
    fn from(v: Vec<nonlit2lit::KeyValPair>) -> Self {
        let mut ret = KlvXcoderArg::default();
        for x in v.iter() {
            if x.key.is_none() || x.val.is_none() { continue }
            let key_rf = x.key.as_ref().unwrap();
            let val_rf = x.val.as_ref().unwrap();
            match KlvXcoderArgValue::try_from(key_rf.to_token_stream().to_string().as_str()) {
                Ok(KlvXcoderArgValue::Type) => if let syn::Lit::Str(val) = val_rf {
                    if let Ok(val) = syn::parse_str::<syn::Type>(&val.value()) {
                        ret.typ = Some(val);
                    }
                },
                Ok(KlvXcoderArgValue::Func) => if let syn::Lit::Str(val) = val_rf {
                    if let Ok(val) = syn::parse_str::<syn::Path>(&val.value()) {
                        ret.func = Some(val);
                    }
                },
                Ok(KlvXcoderArgValue::Fixed) => if val_rf.to_token_stream().to_string() == "true" {
                    ret.fixed = true;
                },
                _ => {},
            }
        }
        ret
    }
}
impl std::fmt::Debug for KlvXcoderArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KlvXcoderArg")
            .field("typ", &self.typ.to_token_stream().to_string())
            .field("func", &self.func.to_token_stream().to_string())
            .field("fixed", &self.fixed)
            .finish()
    }
}

#[derive(Const)]
#[armtype(&str)]
pub(crate) enum KlvFieldAttrValue {
    // key
    #[value = "key"]
    Key,
    // length
    #[value = "len"]
    Len,
    // value encoder / decoder
    #[value = "encoder"]
    Enc,
    #[value = "decoder"]
    Dec,
}

#[derive(Default)]
pub(crate) struct KlvFieldAttr {
    pub name: Option<syn::Ident>,
    pub typ: Option<syn::Type>,
    pub key: Option<Vec<u8>>,
    pub len: Option<usize>,
    pub dec: Option<syn::Path>,
    pub enc: Option<syn::Path>,
}
impl std::fmt::Debug for KlvFieldAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KlvFieldAttr")
            .field("name", &self.name)
            .field("typ", &self.typ.to_token_stream().to_string())
            .field("key", &self.key)
            .field("len", &self.len)
            .field("dec", &self.dec.to_token_stream().to_string())
            .field("enc", &self.enc.to_token_stream().to_string())
            .finish()
    }
}