// --------------------------------------------------
// external
// --------------------------------------------------
use thisenum::Const;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::{
    NameValue,
    MetaItem,
    MetaTuple,
};
use crate::ATTR;

#[derive(Const)]
#[armtype(&str)]
/// Field Attribute Names
/// 
/// # Arms
/// 
/// * `Key` - The key, as a slice of `stream` type (usually bytes)
/// * `Length` - The length, as [`usize`]
/// * `Encoder` - The encoder
/// * `Decoder` - The decoder
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

pub(crate)struct FieldAttrSchema {
    name: syn::Ident,
    contents: FieldAttrContents,
}
/// [`FieldAttrSchema`] implementation
impl FieldAttrSchema {
    pub fn from_field(input: &syn::Field) -> Option<Self> {
        // --------------------------------------------------
        // can now safely unwrap
        // --------------------------------------------------
        if let None = input.ident { return None }
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
            Some(parsed) => Some(FieldAttrSchema {
                name: input.ident.clone().unwrap(),
                contents: parsed.into(),
            }),
            None => None,
        }
    }
}
/// [`FieldAttrSchema`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for FieldAttrSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "name: {}, contents: {}", self.name, self.contents)
    }
}
crate::debug_from_display!(FieldAttrSchema);

#[derive(Default)]
pub(crate)struct FieldAttrContents {
    // #[symple(rename = "key")]
    key: NameValue<syn::Lit>,
    len: NameValue<syn::Lit>,
    dec: Option<NameValue<syn::Path>>,
    enc: Option<NameValue<syn::Path>>,
}
/// [`FieldAttrContents`] implementation of [`From`] for [`MetaTuple`]
impl From<MetaTuple> for FieldAttrContents {
    fn from(input: MetaTuple) -> Self {
        let mut output = Self::default();
        input
            .into_iter()
            .for_each(|item| if let MetaItem::NameValue(x) = item {
                match FieldNames::try_from(x.name.to_string().as_str()) {
                    Ok(FieldNames::Key) => output.key = x.into(),
                    Ok(FieldNames::Length) => output.len = x.into(),
                    Ok(FieldNames::Encoder) => output.enc = Some(x.into()),
                    Ok(FieldNames::Decoder) => output.dec = Some(x.into()),
                    _ => (),
                }
            });
        output
    }
}
/// [`FieldAttrContents`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for FieldAttrContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "key: {}, len: {}, enc: {:?}, dec: {:?}", self.key, self.len, self.enc, self.dec)
    }
}
crate::debug_from_display!(FieldAttrContents);