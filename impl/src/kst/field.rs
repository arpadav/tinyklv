// --------------------------------------------------
// external
// --------------------------------------------------
use tinyklv_common::FieldNames;
use tinyklv_common::symple::{
    self,
    Tuple,
    MetaItem,
    MetaTuple,
    NameValue,
};

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ATTR;
use crate::kst::xcoder::DefaultXcoder;

/// Field Attributes
/// 
/// See the [FieldNames] enum for the different attribute names.
/// 
/// # Example
/// 
/// ```rust ignore
/// use tinyklv_impl::Klv;
///
/// #[derive(Klv)]
/// #[klv(stream = &str, ...)]
/// struct Foo {
///     #[key = "01"]
///     checksum: u8,
///     
///     #[key = "02"]
///     name: String,
/// }
/// ```
pub(crate) struct FieldAttrSchema {
    pub name: syn::Ident,
    pub ty: syn::Type,
    pub contents: FieldAttrContents,
}
/// [FieldAttrSchema] implementation
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
/// [FieldAttrSchema] implementation of [std::fmt::Display]
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
    pub dec: Option<NameValue<syn::Path>>,
    pub enc: Option<NameValue<syn::Path>>,
    pub dynlen: Option<bool>,
}
/// [FieldAttrContents] implementation
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
        match &other.xcoder.enc {
            Some(enc) => match self.enc {
                Some(_) => (),
                None => self.enc = Some(symple::NameValue::new(enc.clone())),
            }
            None => (),
        }
        match &other.xcoder.dec {
            Some(dec) => match self.dec {
                Some(_) => (),
                None => self.dec = Some(symple::NameValue::new(dec.clone())),
            },
            None => (),
        }
        match &other.dynlen {
            Some(x) => match self.dynlen {
                Some(_) => (),
                None => self.dynlen = Some(*x),
            }
            None => (),
        }
    }
}
/// [FieldAttrContents] implementation of [From] for [MetaTuple]
impl From<MetaTuple> for FieldAttrContents {
    fn from(input: MetaTuple) -> Self {
        let mut output = Self::default();
        input
            .into_iter()
            .for_each(|item| if let MetaItem::NameValue(x) = item.clone() {
                match FieldNames::try_from(x.name.to_string().as_str()) {
                    Ok(FieldNames::Key) => output.key = x.into(),
                    Ok(FieldNames::DynLen) => output.dynlen = if let symple::MetaValue::Lit(syn::Lit::Bool(syn::LitBool { value: v, .. })) = x.value { Some(v) } else { None },
                    Ok(FieldNames::Encoder) => output.enc = Some(x.into()),
                    Ok(FieldNames::Decoder) => output.dec = Some(x.into()),
                    _ => (),
                }
            });
        output
    }
}
/// [FieldAttrContents] implementation of [std::fmt::Display]
impl std::fmt::Display for FieldAttrContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "key: {}, enc: {:?}, dec: {:?}, dyn: {:?}", self.key, self.enc, self.dec, self.dynlen)
    }
}
// symple::debug_from_display!(FieldAttrContents);
tinyklv_common::debug_from_display!(FieldAttrContents);