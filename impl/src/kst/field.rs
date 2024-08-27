// --------------------------------------------------
// external
// --------------------------------------------------
use tinyklv_common::FieldNames;
use tinyklv_common::symple::{
    self,
    prelude::*,
    Tuple,
    MetaItem,
    MetaTuple,
    NameValue,
};
use quote::ToTokens;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ATTR;
use crate::kst::xcoder::{
    ValueXcoder,
    DefaultXcoder,
};

/// Field Attributes
/// 
/// See the [`FieldNames`] enum for the different attribute names.
pub(crate) struct FieldAttrSchema {
    pub name: syn::Ident,
    pub ty: syn::Type,
    pub contents: FieldAttrContents,
}
/// [`FieldAttrSchema`] implementation
impl FieldAttrSchema {
    pub fn from_field(input: &syn::Field) -> Option<Self> {
        // --------------------------------------------------
        // can now safely unwrap
        // --------------------------------------------------
        if let None = input.ident { return None }
        // --------------------------------------------------
        // parse as `symple::MetaTuple`
        // --------------------------------------------------
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
                ty: input.ty.clone(),
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
// symple::debug_from_display!(FieldAttrSchema);
tinyklv_common::debug_from_display!(FieldAttrSchema);

#[derive(Default)]
pub(crate) struct FieldAttrContents {
    pub key: NameValue<syn::Lit>,
    pub xcoder: Tuple<ValueXcoder>,
}
/// [`FieldAttrContents`] implementation
impl FieldAttrContents {
    pub fn update(&mut self, ty: &syn::Type, other: &Tuple<DefaultXcoder>) {
        // --------------------------------------------------
        // now can safely unwrap
        // --------------------------------------------------
        if other.value.is_none() { return }
        let other = other.value.as_ref().unwrap();
        // --------------------------------------------------
        // return if types dont match
        // --------------------------------------------------
        if ty != &other.ty && match crate::parse::unwrap_option_type(&ty) {
            Some(f) => &other.ty != f,
            None => true,
        } { return }
        // --------------------------------------------------
        // set
        // --------------------------------------------------
        match &other.enc {
            Some(enc) => match self.enc() {
                Some(_) => (),
                None => self.set_enc(enc.clone()),
            }
            None => (),
        }
        match &other.dec {
            Some(dec) => match self.dec() {
                Some(_) => (),
                None => self.set_dec(dec.clone()),
            },
            None => (),
        }
        match &other.dynlen {
            Some(x) => match self.dynlen() {
                Some(_) => (),
                None => self.set_dynlen(*x),
            }
            None => (),
        }
    }

    fn set_enc(&mut self, enc: syn::Path) {
        match self.xcoder.get_mut() {
            Some(x) => x.enc = Some(enc),
            None => self.xcoder.value = Some(ValueXcoder{
                enc: Some(enc),
                ..Default::default()  
            })
        }
    }

    pub fn enc(&self) -> Option<&syn::Path> {
        self.xcoder.get().map_or(None, |x| x.enc.as_ref())
    }

    fn set_dec(&mut self, dec: syn::Path) {
        match self.xcoder.get_mut() {
            Some(x) => x.dec = Some(dec),
            None => self.xcoder.value = Some(ValueXcoder{
                dec: Some(dec),
                ..Default::default()
            })
        }
    }

    pub fn dec(&self) -> Option<&syn::Path> {
        self.xcoder.get().map_or(None, |x| x.dec.as_ref())
    }

    fn set_dynlen(&mut self, dynlen: bool) {
        match self.xcoder.get_mut() {
            Some(x) => x.dynlen = Some(dynlen),
            None => self.xcoder.value = Some(ValueXcoder{
                dynlen: Some(dynlen),
                ..Default::default()
            })
        }
    }

    pub fn dynlen(&self) -> Option<bool> {
        self.xcoder.get().map_or(None, |x| x.dynlen)
    }
}
/// [`FieldAttrContents`] implementation of [`From`] for [`MetaTuple`]
impl From<MetaTuple> for FieldAttrContents {
    fn from(input: MetaTuple) -> Self {
        let mut output = Self::default();
        let mut dynlen = None;
        let oxcoder = ValueXcoder::from(symple::MetaContents::from(input.clone()));
        input
            .into_iter()
            .for_each(|item| if let MetaItem::NameValue(x) = item.clone() {
                match FieldNames::try_from(x.name.to_string().as_str()) {
                    Ok(FieldNames::Key) => output.key = x.into(),
                    Ok(FieldNames::DynLen) => dynlen = if let symple::MetaValue::Lit(syn::Lit::Bool(syn::LitBool { value: v, .. })) = x.value { Some(v) } else { None },
                    _ => (),
                }
            });
        if let Some(dynlen) = dynlen { output.set_dynlen(dynlen) }
        if let Some(enc) = oxcoder.enc { output.set_enc(enc) }
        if let Some(dec) = oxcoder.dec { output.set_dec(dec) }
        output
    }
}
/// [`FieldAttrContents`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for FieldAttrContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "key: {}, {}",
            self.key.value.to_token_stream().to_string(),
            self.xcoder,
        )
    }
}
// symple::debug_from_display!(FieldAttrContents);
tinyklv_common::debug_from_display!(FieldAttrContents);