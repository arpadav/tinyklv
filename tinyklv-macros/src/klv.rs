
// --------------------------------------------------
// external
// --------------------------------------------------
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
// impl std::fmt::Debug for StructAttrs {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         f.debug_struct("StructAttrs")
//             .field("key_dec", &self.key_dec.as_ref().map_or("None".to_string(), |v| v.to_token_stream().to_string()))
//             .field("key_enc", &self.key_enc.as_ref().map_or("None".to_string(), |v| v.to_token_stream().to_string()))
//             .field("len_dec", &self.len_dec.as_ref().map_or("None".to_string(), |v| v.to_token_stream().to_string()))
//             .field("len_enc", &self.len_enc.as_ref().map_or("None".to_string(), |v| v.to_token_stream().to_string()))
//             // .field("default_dec", &self.default_dec.as_ref().map_or("None".to_string(), |(t, p)| format!("type: {}, func: {}", t.to_token_stream().to_string(), p.to_token_stream().to_string())))
//             // .field("default_enc", &self.default_enc.as_ref().map_or("None".to_string(), |(t, p)| format!("type: {}, func: {}", t.to_token_stream().to_string(), p.to_token_stream().to_string())))
//             .finish()
//     }
// }
/// [`KlvStructAttrProd`] implementation of [`From<StructAttribute>`]
impl From<nonlit2lit::StructAttribute> for KlvStructAttrProd {
    fn from(x: nonlit2lit::StructAttribute) -> Self {
        match KlvStructAttrSum::try_from(x.path.to_token_stream().to_string().as_str()) {
            Ok(KlvStructAttrSum::KeyEnc) => Self {
                key_enc: Some(x.contents),
                ..Default::default()
            },
            Ok(KlvStructAttrSum::KeyDec) => Self {
                key_dec: Some(x.contents),
                ..Default::default()
            },
            Ok(KlvStructAttrSum::LenEnc) => Self {
                len_enc: Some(x.contents),
                ..Default::default()
            },
            Ok(KlvStructAttrSum::LenDec) => Self {
                len_dec: Some(x.contents),
                ..Default::default()
            },
            Ok(KlvStructAttrSum::DefaultEnc) => {
                let mut default_enc = HashMap::new();
                for (k, v) in x.contents.into_iter() {
                    let v = KlvXcoderArgProd {
                        ty: Some(v.ty),
                        func: Some(v.func),
                        fixed: v.fixed,
                    };
                    default_enc.insert(k, v);
                }
                Self {
                    default_enc,
                    ..Default::default()
                }
            }
            Ok(KlvStructAttrSum::DefaultDec) => {
                let mut default_dec = HashMap::new();
                for (k, v) in x.contents.into_iter() {
                    let v = KlvXcoderArgProd {
                        ty: Some(v.ty),
                        func: Some(v.func),
                        fixed: v.fixed,
                    };
                    default_dec.insert(k, v);
                }
                Self {
                    default_dec,
                    ..Default::default()
                }
            }
            _ => Self::default(),
        }
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