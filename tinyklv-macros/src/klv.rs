
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use thisenum::Const;
use hashbrown::HashMap;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::nonlit2lit;

#[derive(Const)]
#[armtype(&str)]
/// [`KlvStructAttrValue`], to hold a
/// reference to all the attribute names 
/// for the KLV-defined struct
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
/// [`KlvStructAttr`], to hold all the 
/// attribute values for the KLV-defined struct
pub(crate) struct KlvStructAttr {
    pub key_dec: Option<KlvXcoderArg>,
    pub key_enc: Option<KlvXcoderArg>,
    pub len_dec: Option<KlvXcoderArg>,
    pub len_enc: Option<KlvXcoderArg>,
    pub default_dec: HashMap<String, KlvXcoderArg>,
    pub default_enc: HashMap<String, KlvXcoderArg>,
}
/// [`KlvStructAttr`] implementation
impl Push<nonlit2lit::ListedAttr> for KlvStructAttr {
    /// See [`Push::push`]
    /// 
    /// # Panics
    /// 
    /// Panics if the function attribute for any of the
    /// following attribute names within [`KlvStructAttr`]
    /// has a missing `func` attribute value:
    /// 
    /// - `key_encoder`
    /// - `key_decoder`
    /// - `len_encoder`
    /// - `len_decoder`
    /// - `default_encoder`
    /// - `default_decoder` 
    fn push(&mut self, x: nonlit2lit::ListedAttr) {
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
/// [`KlvXcoderArgValue`], to hold
/// a reference to all the encoder/decoder
/// input argument names
pub(crate) enum KlvXcoderArgValue {
    #[value = "typ"]
    Type,
    #[value = "func"]
    Func,
    #[value = "fixed"]
    Fixed,
}

#[derive(Clone)]
/// [`KlvXcoderArg`], to hold all the
/// encoder/decoder input argument values
/// for the KLV-defined struct and/or fields
pub(crate) struct KlvXcoderArg {
    pub typ: Option<syn::Type>,
    pub func: Option<syn::Path>,
    pub fixed: bool,
}
/// [`KlvXcoderArg`] implementation
impl KlvXcoderArg {
    /// Return a default [`KlvXcoderArg`],
    /// with the addition of the `typ` field
    /// being set to [`u8_slice`]
    pub fn deftype(mut self) -> Self {
        match self.typ {
            Some(_) => self,
            None => {
                self.typ = Some(u8_slice());
                self
            }
        }
    }
}
/// [`KlvXcoderArg`] implementation of [`std::default::Default`]
impl std::default::Default for KlvXcoderArg {
    fn default() -> Self {
        Self {
            typ: None,
            func: None,
            fixed: false,
        }
    }
}
/// [`KlvXcoderArg`] implementation of [`std::convert::From<Vec<nonlit2lit::KeyValPair>>`](nonlit2lit::KeyValPair)
impl From<Vec<nonlit2lit::KeyValPair>> for KlvXcoderArg {
    /// Iterate through a [`Vec`] of [`nonlit2lit::KeyValPair`]
    /// and assigns them to the appropriate fields in
    /// the [`KlvXcoderArg`]
    fn from(v: Vec<nonlit2lit::KeyValPair>) -> Self {
        let mut ret = KlvXcoderArg::default();
        for x in v.iter() {
            if x.key.is_none() | x.val.is_none() { continue }
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
/// [`KlvXcoderArg`] implementation of [`std::fmt::Debug`]
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
    pub dec: Option<KlvXcoderArg>,
    pub enc: Option<KlvXcoderArg>,
}
/// [`KlvFieldAttr`] implementation of [`Push`] for [`nonlit2lit::ListedAttr`]
impl Push<nonlit2lit::ListedAttr> for KlvFieldAttr {
    /// See [`Push::push`]
    /// 
    /// # Panics
    /// 
    /// Panics if the function attribute for `encoder` or `decoder`
    /// within [`KlvFieldAttr`] is missing
    fn push(&mut self, x: nonlit2lit::ListedAttr) {
        match KlvFieldAttrValue::try_from(x.path().as_str()) {
            Ok(KlvFieldAttrValue::Enc) => self.enc = {
                let mut res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvFieldAttrValue::Enc.value().into())) }
                res.typ = self.typ.clone();
                Some(res)
            },
            Ok(KlvFieldAttrValue::Dec) => self.dec = {
                let mut res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvFieldAttrValue::Dec.value().into())) }
                res.typ = self.typ.clone();
                Some(res)
            },
            _ => {}
        }
    }
}
/// [`KlvFieldAttr`] implementation of [`Push`] for [`nonlit2lit::KeyValPair`]
impl Push<nonlit2lit::KeyValPair> for KlvFieldAttr {
    /// See [`Push::push`]
    /// 
    /// # Panics
    /// 
    /// Panics if the function attribute for values for `key` or `len`
    /// [`KlvFieldAttr`] are invalid formats
    fn push(&mut self, x: nonlit2lit::KeyValPair) {
        if let Some(key) = x.key() {
            match KlvFieldAttrValue::try_from(key.as_str()) {
                Ok(KlvFieldAttrValue::Key) => if let Some(val) = x.val { match val {
                    syn::Lit::ByteStr(lit) => self.key = Some(lit.value()),
                    _ => panic!("{}", crate::Error::NonByteStrKey(val.to_token_stream().to_string())),
                }},
                Ok(KlvFieldAttrValue::Len) => if let Some(val) = x.val { match val {
                    syn::Lit::Int(lit) => self.len = Some(match lit.base10_parse() {
                        Ok(val) => val,
                        Err(_) => panic!("{}", crate::Error::NonIntLength(lit.to_token_stream().to_string())),
                    }),
                    _ => panic!("{}", crate::Error::NonIntLength(val.to_token_stream().to_string())),
                }},
                _ => {}
            }
        }
    }
}
/// [`KlvFieldAttr`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for KlvFieldAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // let dec = self.dec.as_ref().map(|x| x.to_token_stream().to_string());
        f.debug_struct("KlvFieldAttr")
            .field("name", &self.name)
            .field("typ", &self.typ.to_token_stream().to_string())
            .field("key", &self.key)
            .field("len", &self.len)
            .field("dec", &self.dec)
            .field("enc", &self.enc)
            .finish()
    }
}

pub(crate) trait Push<T> {
    fn push(&mut self, item: T);
    // fn extend<I: Iterator<Item = T>>(&mut self, iter: I);
}

pub fn u8_slice() -> syn::Type {
    syn::Type::Reference(syn::TypeReference {
        and_token: Default::default(),
        lifetime: None,
        mutability: None,
        elem: Box::new(syn::Type::Slice(syn::TypeSlice {
            bracket_token: Default::default(),
            elem: Box::new(syn::Type::Path(syn::TypePath {
                qself: None,
                path: syn::parse_quote! { u8 },
            })),
        })),
    })
}

pub fn usize() -> syn::Type {
    syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::parse_quote! { usize },
    })
}