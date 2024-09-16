// --------------------------------------------------
// external
// --------------------------------------------------
use tinyklv_common::StructNames;
use tinyklv_common::symple::{
    Tuple,
    NameValue,
    MetaItem,
    MetaTuple,
    prelude::*,
};
use quote::ToTokens;
use hashbrown::HashSet;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ATTR;
use crate::kst::xcoder::{
    KeyLenXcoder,
    DefaultXcoder,
};

#[derive(Default)]
pub(crate) struct StructAttrSchema {
    pub stream: NameValue<syn::Type>,
    // pub elem: NameValue<syn::Type>,
    // pub owned_stream: Option<NameValue<syn::Type>>,
    pub sentinel: Option<NameValue<syn::Lit>>,
    pub key: Tuple<KeyLenXcoder>,
    pub len: Tuple<KeyLenXcoder>,
    pub defaults: HashSet<Tuple<DefaultXcoder>>,
    pub allow_unimplemented_decode: bool,
    pub allow_unimplemented_encode: bool,
}
/// [`StructAttrSchema`] implementation
impl StructAttrSchema {
    pub fn from_syn(input: &syn::DeriveInput) -> Option<Self> {
        let parsed: Vec<MetaTuple> = input
            .attrs
            .iter()
            .filter(|attr| match attr.path.get_ident() {
                Some(ident) => ident.to_string() == ATTR,
                None => false,
            })
            .map(|attr| MetaTuple::from(format!("{}{}", ATTR, attr.tokens.to_string())))
            .collect();
        match parsed.merge_all() {
            Some(parsed) => Some(parsed.into()),
            None => None,
        }
    }
}
/// [`StructAttrSchema`] implementation of [`From<MetaTuple>`]
impl From<MetaTuple> for StructAttrSchema {
    fn from(input: MetaTuple) -> Self {
        let mut output = Self::default();
        input
            .into_iter()
            .for_each(|item| match item.clone() {
                MetaItem::Tuple(x) => match StructNames::try_from(x.name.to_string().as_str()) {
                    Ok(StructNames::KeyTuple) => output.key = x.into(),
                    Ok(StructNames::LengthTuple) => output.len = x.into(),
                    Ok(StructNames::DefaultTuple) => { output.defaults.insert(x.into()); },
                    _ => (),
                },
                MetaItem::NameValue(x) => match StructNames::try_from(x.name.to_string().as_str()) {
                    Ok(StructNames::Stream) => output.stream = x.into(),
                    Ok(StructNames::Sentinel) => output.sentinel = Some(x.into()),
                    _ => (),
                },
                MetaItem::Value(x) => match StructNames::try_from(x.to_string().as_str()) {
                    Ok(StructNames::AllowUnimplementedDecode) => output.allow_unimplemented_decode = true,
                    Ok(StructNames::AllowUnimplementedEncode) => output.allow_unimplemented_encode = true,
                    _ => (),
                },
            }
        );
        println!("{:#?}", output);
        output
    }
}
/// [`StructAttrSchema`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for StructAttrSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StructAttrSchema {{ stream: {}, sentinel: {}, key: {}, len: {}, defaults: {:#?}, allow_unimplemented_decode: {}, allow_unimplemented_encode: {} }}",
            self.stream.get().to_token_stream().to_string(),
            self.sentinel.clone().map_or("None".to_string(), |v| v.get().to_token_stream().to_string()),
            self.key,
            self.len,
            self.defaults,
            self.allow_unimplemented_decode,
            self.allow_unimplemented_encode,
        )
    }
}
// symple::debug_from_display!(StructAttrSchema);
tinyklv_common::debug_from_display!(StructAttrSchema);