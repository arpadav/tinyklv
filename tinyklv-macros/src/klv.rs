
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use thisenum::Const;
use hashbrown::HashMap;

#[derive(Const)]
#[armtype(&str)]
pub(crate) enum KlvStructAttributes {
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
pub(crate) struct StructAttrs {
    pub key_dec: Option<XcoderArgs>,
    pub key_enc: Option<XcoderArgs>,
    pub len_dec: Option<XcoderArgs>,
    pub len_enc: Option<XcoderArgs>,
    pub default_dec: HashMap<std::any::TypeId, XcoderArgs>,
    pub default_enc: HashMap<std::any::TypeId, XcoderArgs>,
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

#[derive(Const)]
#[armtype(&str)]
pub(crate) enum KlvXcoderArguments {
    #[value = "type"]
    Type,
    #[value = "func"]
    Func,
    #[value = "fixed"]
    Fixed,
}
pub(crate) struct XcoderArgs {
    pub ty: Option<syn::Type>,
    pub func: Option<syn::Path>,
    pub fixed: bool,
}
impl std::default::Default for XcoderArgs {
    fn default() -> Self {
        Self {
            ty: Some(crate::types::u8_slice()),
            func: None,
            fixed: false,
        }
    }
}
impl std::fmt::Debug for XcoderArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("XcoderArgs")
            .field("ty", &self.ty.to_token_stream().to_string())
            .field("func", &self.func.to_token_stream().to_string())
            .field("fixed", &self.fixed)
            .finish()
    }
}

#[derive(Const)]
#[armtype(&str)]
pub(crate) enum KlvFieldAttributes {
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
pub(crate) struct FieldAttrs {
    pub name: Option<syn::Ident>,
    pub typ: Option<syn::Type>,
    pub key: Option<Vec<u8>>,
    pub len: Option<usize>,
    pub dec: Option<syn::Path>,
    pub enc: Option<syn::Path>,
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