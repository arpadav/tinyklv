// --------------------------------------------------
// external
// --------------------------------------------------
use symple::{
    Tuple,
    MetaItem,
    MetaTuple,
    NameValue,
};
use thisenum::Const;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ATTR;
use crate::kst::xcoder::DefaultXcoder;

#[derive(Const)]
#[armtype(&str)]
/// Field Attribute Names
pub(crate) enum FieldNames {
    #[value = "key"]
    /// The key. Required: as a slice of `stream` type.
    /// 
    /// This is a required attribute, written using a literal (either bytes
    /// or str), to help identify the field during parsing.
    /// 
    /// Non-literal keys are currently not supported.
    Key,

    #[value = "dyn"]
    /// The dynamic length. Optional: defaults to `false`.
    /// 
    /// This is an optional attribute, which indicates the length of the field is dynamic.
    /// This is commonly used for Strings, but can be for other values as well. 
    /// 
    /// For example, if the field is of type [u8], it will almost always be a single byte which is 
    /// parsed as the length. For [u16], it will be two bytes. This indicates a **constant** length, 
    /// therefore the `dyn` length keyword can be omitted since the parser used will never use
    /// the input length.
    /// 
    /// In practice, streams would look like:
    /// 
    /// ```rust
    /// use tinyklv::Klv;
    /// use tinyklv::prelude::*;
    /// 
    /// #[derive(Klv)]
    /// #[klv(
    ///     sentinel = b"\x00\x00\x00",
    ///     key(dec = tinyklv::dec::binary::u8,
    ///         enc = tinyklv::enc::binary::u8),
    ///     len(dec = tinyklv::dec::binary::u8_as_usize,
    ///         enc = tinyklv::enc::binary::u8),
    /// )]
    /// struct Foo {
    ///     #[klv(key = 0x01, dyn = true, dec = tinyklv::dec::binary::to_string)]
    ///     name: String,
    /// 
    ///     #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16)]
    ///     number: u16,
    /// }
    /// 
    /// fn main() {
    ///     let mut stream1: &[u8] = &[
    ///         // sentinel: 0x00, 0x00, 0x00
    ///         0x00, 0x00, 0x00,
    ///         // packet length: 9 bytes
    ///         0x09,
    ///         // key: 0x01, len: 0x03
    ///         // since the len is dyn, it is used as an input in `tinyklv::dec::binary::to_string`
    ///         0x01, 0x03,
    ///         // value decoded: "KLV"
    ///         0x4B, 0x4C, 0x56,
    ///         // key: 0x02, len: 0x02
    ///         // since the len is not dyn, it is not used in `tinyklv::dec::binary::be_u16`
    ///         0x02, 0x02,
    ///         // value decoded: 258
    ///         0x01, 0x02,
    ///     ];
    ///     match Foo::decode(&mut stream1) {
    ///         Ok(foo) => {
    ///             assert_eq!(foo.name, "KLV");
    ///             assert_eq!(foo.number, 258);
    ///         },
    ///         Err(e) => panic!("{}", e),
    ///     }
    ///     
    ///     let mut stream2: &[u8] = &[
    ///         // sentinel: 0x00, 0x00, 0x00
    ///         0x00, 0x00, 0x00,
    ///         // packet length: 18 bytes
    ///         0x12,
    ///         // key: 0x01, len: 0x0C
    ///         // since the len is dyn, it is used as an input in `tinyklv::dec::binary::to_string`
    ///         0x01, 0x0C,
    ///         // value decoded: "Hello World!"
    ///         0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x20, 0x57, 0x6F, 0x72, 0x6C, 0x64, 0x21,
    ///         // key: 0x02, len: 0x02
    ///         // since the len is not dyn, it is not used in `tinyklv::dec::binary::be_u16`
    ///         0x02, 0x02,
    ///         // value decoded: 42
    ///         0x00, 0x2A,
    ///     ];
    ///     match Foo::decode(&mut stream2) {
    ///         Ok(foo) => {
    ///             assert_eq!(foo.name, "Hello World!");
    ///             assert_eq!(foo.number, 42);
    ///         },
    ///         Err(e) => panic!("{}", e),
    ///     }
    /// }
    /// ```
    DynLen,

    #[value = "enc"]
    /// The encoder
    Encoder,

    #[value = "dec"]
    /// The decoder]
    Decoder,
}

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
symple::debug_from_display!(FieldAttrSchema);

#[derive(Default)]
pub(crate) struct FieldAttrContents {
    pub key: NameValue<syn::Lit>,
    pub dec: Option<NameValue<syn::Path>>,
    pub enc: Option<NameValue<syn::Path>>,
    pub dynlen: Option<bool>,
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
/// [`FieldAttrContents`] implementation of [`From`] for [`MetaTuple`]
impl From<MetaTuple> for FieldAttrContents {
    fn from(input: MetaTuple) -> Self {
        let mut output = Self::default();
        input
            .into_iter()
            .for_each(|item| if let MetaItem::NameValue(x) = item {
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
/// [`FieldAttrContents`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for FieldAttrContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "key: {}, enc: {:?}, dec: {:?}, dyn: {:?}", self.key, self.enc, self.dec, self.dynlen)
    }
}
symple::debug_from_display!(FieldAttrContents);