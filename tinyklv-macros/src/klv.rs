
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
pub(crate) enum KlvStructAttrSum {
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
pub(crate) struct KlvStructAttrProd {
    pub key_dec: Option<KlvXcoderArgProd>,
    pub key_enc: Option<KlvXcoderArgProd>,
    pub len_dec: Option<KlvXcoderArgProd>,
    pub len_enc: Option<KlvXcoderArgProd>,
    pub default_dec: HashMap<std::any::TypeId, KlvXcoderArgProd>,
    pub default_enc: HashMap<std::any::TypeId, KlvXcoderArgProd>,
}
/// [`KlvStructAttrProd`] implementation of [`Push<StructAttribute>`]
impl KlvStructAttrProd {
    fn push(&mut self, x: nonlit2lit::StructAttribute) {
        match KlvStructAttrSum::try_from(x.path().as_str()) {
            Ok(KlvStructAttrSum::KeyEnc) => self.key_enc = Some(x.contents.into()),
            Ok(KlvStructAttrSum::KeyDec) => self.key_dec = Some(x.contents.into()),
            Ok(KlvStructAttrSum::LenEnc) => self.len_enc = Some(x.contents.into()),
            Ok(KlvStructAttrSum::LenDec) => self.len_dec = Some(x.contents.into()),
            Ok(KlvStructAttrSum::DefaultEnc) => {
                let mut kxap = KlvXcoderArgProd::default();
                x.contents
                    .iter()
                    .for_each(|x| {
                        if x.key.is_none() || x.val.is_none() { return }
                        match KlvXcoderArgSum::try_from(x.key.unwrap().to_token_stream().to_string().as_str()) {
                            Ok(KlvXcoderArgSum::Type) => kxap.ty = Some(x.val.unwrap()),
                            Ok(KlvXcoderArgSum::Func) => kxap.func = Some(x.val.unwrap()),
                            Ok(KlvXcoderArgSum::Fixed) => kxap.fixed = x.val.unwrap(),
                            _ => {}
                        }
                    });
                self.default_enc.insert(kxap.ty.unwrap().type_id(), kxap);
            },
            Ok(KlvStructAttrSum::DefaultDec) => {
                let mut kxap = KlvXcoderArgProd::default();
                x.contents
                    .iter()
                    .for_each(|x| {
                        if x.key.is_none() || x.val.is_none() { return }
                        match KlvXcoderArgSum::try_from(x.key.unwrap().to_token_stream().to_string().as_str()) {
                            Ok(KlvXcoderArgSum::Type) => kxap.ty = Some(x.val.unwrap()),
                            Ok(KlvXcoderArgSum::Func) => kxap.func = Some(x.val.unwrap()),
                            Ok(KlvXcoderArgSum::Fixed) => kxap.fixed = x.val.unwrap(),
                            _ => {}
                        }
                    });
                self.default_dec.insert(kxap.ty.unwrap().type_id(), kxap);
            },
            Err(_) => {}
        }
    }
}
/// [`KlvStructAttrProd`] implementation of [`From<Vec<KeyValPair>>`]
impl From<Vec<nonlit2lit::KeyValPair>> for KlvStructAttrProd {
    fn from(v: Vec<nonlit2lit::KeyValPair>) -> Self {
        let mut ret = KlvStructAttrProd::default();
        v.iter().for_each(|x| ret.push(x.clone()));
        ret
    }
}

#[derive(Const)]
#[armtype(&str)]
pub(crate) enum KlvXcoderArgSum {
    #[value = "type"]
    Type,
    #[value = "func"]
    Func,
    #[value = "fixed"]
    Fixed,
}
pub(crate) struct KlvXcoderArgProd {
    pub ty: Option<syn::Type>,
    pub func: Option<syn::Path>,
    pub fixed: bool,
}
impl std::default::Default for KlvXcoderArgProd {
    fn default() -> Self {
        Self {
            ty: Some(crate::types::u8_slice()),
            func: None,
            fixed: false,
        }
    }
}
impl std::fmt::Debug for KlvXcoderArgProd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KlvXcoderArgProd")
            .field("ty", &self.ty.to_token_stream().to_string())
            .field("func", &self.func.to_token_stream().to_string())
            .field("fixed", &self.fixed)
            .finish()
    }
}

#[derive(Const)]
#[armtype(&str)]
pub(crate) enum KlvFieldAttrSum {
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
pub(crate) struct KlvFieldAttrProd {
    pub name: Option<syn::Ident>,
    pub typ: Option<syn::Type>,
    pub key: Option<Vec<u8>>,
    pub len: Option<usize>,
    pub dec: Option<syn::Path>,
    pub enc: Option<syn::Path>,
}
impl std::fmt::Debug for KlvFieldAttrProd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KlvFieldAttrProd")
            .field("name", &self.name)
            .field("typ", &self.typ.to_token_stream().to_string())
            .field("key", &self.key)
            .field("len", &self.len)
            .field("dec", &self.dec.to_token_stream().to_string())
            .field("enc", &self.enc.to_token_stream().to_string())
            .finish()
    }
}