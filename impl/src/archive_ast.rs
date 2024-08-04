
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use thisenum::Const;
use hashbrown::HashMap;
use syn::{
    Meta,
    Data,
    Attribute,
    DataStruct,
    DeriveInput,
};

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::archive_attr;
use crate::Error;

pub(crate) struct Input {
    pub name: syn::Ident,
    pub sattr: KlvStructAttr,
    pub fattrs: Vec<KlvFieldAttr>,
}
/// [`Input`] implementation
impl Input {
    pub fn from_syn(input: &DeriveInput) -> Result<Self, Error> {
        // --------------------------------------------------
        // extract the name, variants, and values
        // --------------------------------------------------
        let name = input.ident.clone();
        let fields = match &input.data {
            Data::Struct(DataStruct { fields, .. }) => fields,
            _ => panic!("{}", crate::Error::DeriveForNonStruct(crate::NAME.into(), name.to_string())),
        };
        let sattr = KlvStructAttr::from_syn(&input);
        let fattrs = KlvFieldAttr::from_syn(&fields);
        Self { name, sattr, fattrs }.update().verify()
    }

    /// Overwrites non-existing encoder / decoder
    /// with the default, if it exists
    fn update(mut self) -> Self {
        // --------------------------------------------------
        // loop through fields, update encoder / decoder
        // using default types and struct attributes for
        // default encoder / decoder, if needed
        // --------------------------------------------------
        self.fattrs
            .iter_mut()
            .for_each(|field_attr| {
                // --------------------------------------------------
                // if field has no encoder, AND the
                // default encoder exists for that type
                // within struct_attrs, use that
                // --------------------------------------------------
                // if field has no type, use the type
                // from the variant definition. otherwise,
                // if there is a mismatch, raise an error
                // --------------------------------------------------
                if field_attr.enc.is_none() {
                    match field_attr.typ.as_ref() {
                        Some(typ) => match self.sattr.default_enc.get(&typ.to_token_stream().to_string()) {
                            Some(func) => field_attr.enc = Some(func.clone()),
                            None => (),
                        },
                        None => (),
                    }
                } else if let Some(enc_typ) = &field_attr.enc.as_ref().unwrap().typ {
                    let enc_typ_str = enc_typ.to_token_stream().to_string();
                    let variant_typ_str = field_attr.typ.as_ref().unwrap().to_token_stream().to_string();
                    let variant_name_str = field_attr.name.as_ref().unwrap().to_token_stream().to_string();
                    if enc_typ_str != variant_typ_str { panic!("{}", Error::EncoderTypeMismatch(enc_typ_str, variant_typ_str, variant_name_str)); }
                } else {
                    field_attr.enc.as_mut().unwrap().typ = Some(field_attr.typ.as_ref().unwrap().clone());
                }
                // --------------------------------------------------
                // if field has no decoder, AND the
                // default decoder exists for that type
                // within struct_attrs, use that
                // --------------------------------------------------
                // if field has no type, use the type
                // from the variant definition. otherwise,
                // if there is a mismatch, raise an error
                // --------------------------------------------------
                if field_attr.dec.is_none() {
                    match field_attr.typ.as_ref() {
                        Some(typ) => match self.sattr.default_dec.get(&typ.to_token_stream().to_string()) {
                            Some(func) => field_attr.dec = Some(func.clone()),
                            None => (),
                        },
                        None => (),
                    }
                } else if let Some(dec_typ) = &field_attr.dec.as_ref().unwrap().typ {
                    let dec_typ_str = dec_typ.to_token_stream().to_string();
                    let variant_typ_str = field_attr.typ.as_ref().unwrap().to_token_stream().to_string();
                    let variant_name_str = field_attr.name.as_ref().unwrap().to_token_stream().to_string();
                    if dec_typ_str != variant_typ_str { panic!("{}", Error::DecoderTypeMismatch(dec_typ_str, variant_typ_str, variant_name_str)); }
                } else {
                    field_attr.dec.as_mut().unwrap().typ = Some(field_attr.typ.as_ref().unwrap().clone());
                }
            });
        self
    }

    fn verify(self) -> Result<Self, Error> {
        let name = self.name.to_token_stream().to_string();
        if self.sattr.key_dec.is_none() { return Err(Error::MissingFunc(KlvStructAttrValue::KeyDec.value().into(), name)) }
        if self.sattr.key_enc.is_none() { return Err(Error::MissingFunc(KlvStructAttrValue::KeyEnc.value().into(), name)) }
        if self.sattr.len_dec.is_none() { return Err(Error::MissingFunc(KlvStructAttrValue::LenDec.value().into(), name)) }
        if self.sattr.len_enc.is_none() { return Err(Error::MissingFunc(KlvStructAttrValue::LenEnc.value().into(), name)) }
        for fa in &self.fattrs {
            let var = fa.name.as_ref().unwrap().to_token_stream().to_string();
            if fa.key.is_none() { return Err(Error::MissingKey(var)) }
            if fa.len.is_none() { return Err(Error::MissingLength(var)) }
            match fa.enc {
                Some(ref enc) => {
                    if enc.typ.is_none() { return Err(Error::MissingType(KlvFieldAttrValue::Enc.value().into(), var)) }
                    if enc.func.is_none() { return Err(Error::MissingFunc(KlvFieldAttrValue::Enc.value().into(), var)) }
                },
                None => return Err(Error::MissingFunc(KlvFieldAttrValue::Enc.value().into(), var)),
            }
            match fa.dec {
                Some(ref dec) => {
                    if dec.typ.is_none() { return Err(Error::MissingType(KlvFieldAttrValue::Dec.value().into(), var)) }
                    if dec.func.is_none() { return Err(Error::MissingFunc(KlvFieldAttrValue::Dec.value().into(), var)) }
                },
                None => return Err(Error::MissingFunc(KlvFieldAttrValue::Dec.value().into(), var)),
            }
        }
        Ok(self)
    }
}
/// [`Input`] implementation of [`Into<TokenStream>`]
impl Into<proc_macro::TokenStream> for Input {
    fn into(self) -> proc_macro::TokenStream {
        crate::expand::from(self).into()
    }
}

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
impl KlvStructAttr {
    /// Creates a new [`KlvStructAttr`] struct from
    /// a [`syn::DeriveInput`]
    pub fn from_syn(input: &DeriveInput) -> Self {
        let mut me = KlvStructAttr::default();
        input
            .attrs
            .iter()
            .filter(|attr|
                match attr.path.get_ident() {
                    Some(ident) => KlvStructAttrValue::try_from(ident.to_string().as_str()).is_ok(),
                    None => false,
                }
            )
            .for_each(|attr| Self::parse_struct_attr(attr, &mut me));
        me
    }
    
    /// Parses a struct-level attribute and pushes it to the
    /// [`KlvStructAttr`] struct
    fn parse_struct_attr(attr: &Attribute, struct_attrs: &mut KlvStructAttr) {
        let sattr = match archive_attr::ListedAttr::new(attr.to_token_stream().to_string()) {
            Ok(sattr) => sattr,
            Err(e) => {
                println!("{}: {:#?}", crate::NAME, e);
                return
            },
        };
        struct_attrs.push(sattr);
    }

}
/// [`KlvStructAttr`] implementation of [`Push`] for [`attr::ListedAttr`]
impl Push<archive_attr::ListedAttr> for KlvStructAttr {
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
    fn push(&mut self, x: archive_attr::ListedAttr) {
        match KlvStructAttrValue::try_from(x.path().as_str()) {
            Ok(KlvStructAttrValue::KeyEnc) => self.key_enc = {
                let res: KlvXcoderArg = x.contents.into();
                Some(res.deftype())
            },
            Ok(KlvStructAttrValue::KeyDec) => self.key_dec = {
                let res: KlvXcoderArg = x.contents.into();
                Some(res.deftype())
            },
            Ok(KlvStructAttrValue::LenEnc) => self.len_enc = {
                let res: KlvXcoderArg = x.contents.into();
                Some(res.deftype())
            },
            Ok(KlvStructAttrValue::LenDec) => self.len_dec = {
                let res: KlvXcoderArg = x.contents.into();
                Some(res.deftype())
            },
            Ok(KlvStructAttrValue::DefaultEnc) => {
                let res: KlvXcoderArg = x.contents.into();
                self.default_enc.insert(res.typ.clone().unwrap().to_token_stream().to_string(), res);
            },
            Ok(KlvStructAttrValue::DefaultDec) => {
                let res: KlvXcoderArg = x.contents.into();
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
    #[value = "include_self"]
    IncludeSelf,
}

#[derive(Clone)]
/// [`KlvXcoderArg`], to hold all the
/// encoder/decoder input argument values
/// for the KLV-defined struct and/or fields
pub(crate) struct KlvXcoderArg {
    pub typ: Option<syn::Type>,
    pub func: Option<syn::Path>,
    pub include_self: bool,
}
/// [`KlvXcoderArg`] implementation
impl KlvXcoderArg {
    /// Return a default [`KlvXcoderArg`],
    /// with the addition of the `typ` field
    /// being set to [`vec_u8`]
    pub fn deftype(mut self) -> Self {
        match self.typ {
            Some(_) => self,
            None => {
                self.typ = Some(vec_u8());
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
            include_self: false,
        }
    }
}
/// [`KlvXcoderArg`] implementation of [`PartialEq`]
impl PartialEq for KlvXcoderArg {
    fn eq(&self, other: &Self) -> bool {
        self.typ.to_token_stream().to_string() == other.typ.to_token_stream().to_string()
            && self.func.to_token_stream().to_string() == other.func.to_token_stream().to_string()
            && self.include_self == other.include_self
    }
}
/// [`KlvXcoderArg`] implementation of [`Eq`]
impl Eq for KlvXcoderArg {}
/// [`KlvXcoderArg`] implementation of [`Hash`]
impl std::hash::Hash for KlvXcoderArg {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.typ.to_token_stream().to_string().hash(state);
        self.func.to_token_stream().to_string().hash(state);
        self.include_self.hash(state);
    }
}
/// [`KlvXcoderArg`] implementation of [`std::convert::From<Vec<attr::KeyValPair>>`](attr::KeyValPair)
impl From<Vec<archive_attr::KeyValPair>> for KlvXcoderArg {
    /// Iterate through a [`Vec`] of [`attr::KeyValPair`]
    /// and assigns them to the appropriate fields in
    /// the [`KlvXcoderArg`]
    fn from(v: Vec<archive_attr::KeyValPair>) -> Self {
        let mut ret = KlvXcoderArg::default();
        for x in v.iter() {
            match KlvXcoderArgValue::try_from(x.key.to_token_stream().to_string().as_str()) {
                Ok(KlvXcoderArgValue::Type) => if let syn::Lit::Str(val) = &x.val {
                    if let Ok(val) = syn::parse_str::<syn::Type>(&val.value()) {
                        ret.typ = Some(val);
                    }
                },
                Ok(KlvXcoderArgValue::Func) => if let syn::Lit::Str(val) = &x.val {
                    if let Ok(val) = syn::parse_str::<syn::Path>(&val.value()) {
                        ret.func = Some(val);
                    }
                },
                Ok(KlvXcoderArgValue::IncludeSelf) => if x.val.to_token_stream().to_string() == "true" {
                    ret.include_self = true;
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
            .field("include_self", &self.include_self)
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
/// [`KlvFieldAttr`] implementation
impl KlvFieldAttr {
    pub fn from_syn(fields: &syn::Fields) -> Vec<Self> {
        fields
            .iter()
            .filter_map(|field| {
                let mut attrs = KlvFieldAttr::default();
                field
                    .attrs
                    .iter()
                    .for_each(|attr| Self::parse_field_attr(attr, &mut attrs));
                attrs.name = field.ident.clone();
                attrs.typ = Some(field.ty.clone());
                Some(attrs)
            })
            .collect()
    }

    /// Parses a field-level attribute and pushes it to the
    /// [`KlvFieldAttr`] struct
    fn parse_field_attr(attr: &Attribute, field_attrs: &mut KlvFieldAttr) {
        match attr.parse_meta() {
            Ok(Meta::NameValue(mnv)) => {
                let kvp = match archive_attr::KeyValPair::try_from(mnv) {
                    Ok(kvp) => kvp,
                    Err(e) => panic!("{}", e),
                };
                field_attrs.push(kvp);
            },
            Err(_) => match archive_attr::ListedAttr::new(attr.to_token_stream().to_string()) {
                Ok(sattr) => field_attrs.push(sattr),
                _ => return,
            }
            _ => return,
        }
    }
}
/// [`KlvFieldAttr`] implementation of [`Push`] for [`attr::ListedAttr`]
impl Push<archive_attr::ListedAttr> for KlvFieldAttr {
    /// See [`Push::push`]
    /// 
    /// # Panics
    /// 
    /// Panics if the function attribute for `encoder` or `decoder`
    /// within [`KlvFieldAttr`] is missing
    fn push(&mut self, x: archive_attr::ListedAttr) {
        let name = self.name.to_token_stream().to_string();
        match KlvFieldAttrValue::try_from(x.path().as_str()) {
            Ok(KlvFieldAttrValue::Enc) => self.enc = {
                let mut res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvFieldAttrValue::Enc.value().into(), name)) }
                res.typ = self.typ.clone();
                Some(res)
            },
            Ok(KlvFieldAttrValue::Dec) => self.dec = {
                let mut res: KlvXcoderArg = x.contents.into();
                if res.func.is_none() { panic!("{}", crate::Error::MissingFunc(KlvFieldAttrValue::Dec.value().into(), name)) }
                res.typ = self.typ.clone();
                Some(res)
            },
            _ => {}
        }
    }
}
/// [`KlvFieldAttr`] implementation of [`Push`] for [`attr::KeyValPair`]
impl Push<archive_attr::KeyValPair> for KlvFieldAttr {
    /// See [`Push::push`]
    /// 
    /// # Panics
    /// 
    /// Panics if the function attribute for values for `key` or `len`
    /// [`KlvFieldAttr`] are invalid formats
    fn push(&mut self, x: archive_attr::KeyValPair) {
        let name = self.name.to_token_stream().to_string();
        match KlvFieldAttrValue::try_from(x.key().as_str()) {
            Ok(KlvFieldAttrValue::Key) => match x.val {
                syn::Lit::ByteStr(lit) => self.key = Some(lit.value()),
                _ => panic!("{}", crate::Error::NonByteStrKey(x.val(), name)),
            },
            Ok(KlvFieldAttrValue::Len) => match x.val {
                syn::Lit::Int(lit) => self.len = Some(match lit.base10_parse() {
                    Ok(val) => val,
                    Err(_) => panic!("{}", crate::Error::NonIntLength(lit.to_token_stream().to_string(), name)),
                }),
                _ => panic!("{}", crate::Error::NonIntLength(x.val(), name)),
            },
            _ => {}
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
}

pub fn _u8_slice() -> syn::Type {
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

pub fn vec_u8() -> syn::Type {
    syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::parse_quote! { Vec<u8> },
    })
}

pub fn _usize() -> syn::Type {
    syn::Type::Path(syn::TypePath {
        qself: None,
        path: syn::parse_quote! { usize },
    })
}