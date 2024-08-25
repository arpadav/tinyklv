// --------------------------------------------------
// external
// --------------------------------------------------
use symple::{
    Tuple,
    NameValue,
    MetaItem,
    MetaTuple,
};
use thisenum::Const;
use hashbrown::HashSet;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ATTR;
use crate::kst::xcoder::{
    DefaultXcoder,
    RequiredXcoder,
};

#[derive(Const, Debug)]
#[armtype(&str)]
/// Struct Attribute Names
enum StructNames {
    /// The stream type. Defaults to &[u8]
    #[value = "stream"]
    Stream,
    /// The sentinel value. Defaults to `None`
    #[value = "sentinel"]
    Sentinel,
    /// The key xcoder tuple
    #[value = "key"]
    KeyTuple,
    /// The length xcoder tuple
    #[value = "len"]
    LengthTuple,
    /// The default xcoder tuple
    #[value = "default"]
    DefaultTuple,
}

#[derive(Default)]
pub(crate) struct StructAttrSchema {
    pub sentinel: NameValue<syn::Lit>,
    pub key: Tuple<RequiredXcoder>,
    pub len: Tuple<RequiredXcoder>,
    pub defaults: HashSet<Tuple<DefaultXcoder>>
}
/// [`StructAttrSchema`] implementation
impl StructAttrSchema {
    pub fn from_syn(input: &syn::DeriveInput) -> Option<Self> {
        let parsed: Option<MetaTuple> = input
            .attrs
            .iter()
            .filter(|attr| match attr.path.get_ident() {
                Some(ident) => ident.to_string() == ATTR,
                None => false,
            })
            .next()
            .map(|attr| MetaTuple::from(format!("{}{}", ATTR, attr.tokens.to_string())));
        match parsed {
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
            .for_each(|item| match item {
                MetaItem::Tuple(x) => match StructNames::try_from(x.name.to_string().as_str()) {
                    Ok(StructNames::KeyTuple) => output.key = x.into(),
                    Ok(StructNames::LengthTuple) => output.len = x.into(),
                    Ok(StructNames::DefaultTuple) => { output.defaults.insert(x.into()); },
                    _ => (),
                },
                MetaItem::NameValue(x) => match StructNames::try_from(x.name.to_string().as_str()) {
                    Ok(StructNames::Sentinel) => output.sentinel = x.into(),
                    _ => (),
                },
            }
        );
        output
    }
}
/// [`StructAttrSchema`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for StructAttrSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "StructAttrSchema {{ sentinel: {}, key: {}, len: {}, defaults: {:#?} }}", self.sentinel, self.key, self.len, self.defaults)
    }
}
symple::debug_from_display!(StructAttrSchema);