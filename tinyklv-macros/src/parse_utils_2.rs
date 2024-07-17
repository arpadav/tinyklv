use proc_macro::token_stream;
// --------------------------------------------------
// external
// --------------------------------------------------
use regex::Regex;
use thiserror::Error;
use lazy_static::lazy_static;
use quote::{format_ident, quote, ToTokens};

// --------------------------------------------------
// constants
// --------------------------------------------------
const ARGS_DELIM_CHAR: char = ',';
lazy_static! {
    static ref PATH_STRUCT_ATTR_RE: Regex = Regex::new(r"#\[(.*?)\((.*?)\)\]").unwrap();
    static ref CNTS_STRUCT_ATTR_RE: Regex = Regex::new(r"#\[.*\((.*?)\)\]").unwrap();
}

// --------------------------------------------------
// error
// --------------------------------------------------
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Can not match to find name/path, e.g. `#[THIS(*)]`: {0}")]
    NameError(String),
    #[error("Can not match to find contents/args, e.g. `#[*(THIS)]`: {0}")]
    ContentsError(String),
    #[error("No key found for contents/args, e.g. `key=val`: {0}")]
    KeyError(String),
    #[error("No value found for contents/args, e.g. `key=val`: {0}")]
    ValError(String),
}

#[derive(Clone)]
pub(crate) struct StructAttribute {
    pub path: syn::Path,
    raw_contents: String,
    pub contents: Vec<KeyValPair>
}
/// [`StructAttribute`] implementation
impl StructAttribute {
    pub fn new(s: String) -> Result<Self, ParseError> {
        // --------------------------------------------------
        // name
        // --------------------------------------------------
        let name = match PATH_STRUCT_ATTR_RE.captures(&s) {
            Some(captures) => match captures.get(1) {
                Some(inner) => inner,
                None => return Err(ParseError::NameError(s)),
            },
            None => return Err(ParseError::NameError(s)),
        };
        // --------------------------------------------------
        // contents
        // --------------------------------------------------
        let contents_raw = match CNTS_STRUCT_ATTR_RE.captures(&s) {
            Some(captures) => match captures.get(1) {
                Some(inner) => inner,
                None => return Err(ParseError::ContentsError(s)),
            },
            None => return Err(ParseError::ContentsError(s)),
        };
        // --------------------------------------------------
        // parse + return
        // --------------------------------------------------
        let name = syn::parse_str::<syn::Path>(name.as_str()).map_err(|_| ParseError::NameError(s.clone()))?;
        let contents: Vec<String> = contents_raw
            .as_str()
            .split(",")
            .map(|x| x.trim().into())
            .collect();
        let contents = contents
            .iter()
            .map(KeyValPair::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(StructAttribute {
            path: name,
            raw_contents: contents_raw.as_str().to_string(),
            contents: contents,
        })
    }

    pub fn as_attr(&self) -> syn::Attribute {
        let token_stream = self.to_token_stream();
        let result: syn::Attribute = syn::parse_quote! {
            #token_stream
        };
        result
    }
}
/// [`StructAttribute`] implementation of [`Into<TokenStream>`]
impl Into<proc_macro2::TokenStream> for StructAttribute {
    fn into(self) -> proc_macro2::TokenStream {
        let path = self.path;
        let contents = self.contents;
        quote! { #[#path(#(#contents),*)] }
    }
}
/// [`StructAttribute`] implementation of [`ToTokens`]
impl ToTokens for StructAttribute {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let token_stream: proc_macro2::TokenStream = self.clone().into();
        tokens.extend(token_stream);
    }
}

#[derive(Clone)]
pub(crate) struct KeyValPair {
    pub key: Option<syn::Ident>,
    pub val: Option<syn::Lit>,
}
/// [`KeyValPair`] implementation of [`TryFrom<&String>`]
impl TryFrom<&String> for KeyValPair {
    type Error = ParseError;
    fn try_from(s: &String) -> Result<Self, Self::Error> {
        // --------------------------------------------------
        // get default + split
        // --------------------------------------------------
        let mut split = s.split("=");
        // --------------------------------------------------
        // parse key
        // --------------------------------------------------
        let key = match split.next() {
            Some(x) => x.trim().to_string(),
            None => return Err(ParseError::KeyError(s.into())),
        };
        let key = match syn::parse_str::<syn::Ident>(&key) {
            Ok(x) => Some(x),
            Err(e) => return Err(ParseError::KeyError(format!("{}: {}", s, e))),
        };
        // --------------------------------------------------
        // parse val
        // --------------------------------------------------
        let val = match split.next() {
            Some(x) => x.trim().to_string(),
            None => return Err(ParseError::ValError(s.into())),
        };
        let val = match syn::parse_str::<syn::Lit>(&val) {
            Ok(x) => Some(x),
            Err(_) =>  match syn::parse_str::<syn::Lit>(&format!("\"{}\"", val)) {
                Ok(x) => Some(x),
                Err(e) => return Err(ParseError::ValError(format!("{}: {}", s, e))),
            }
        };
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(KeyValPair {
            key,
            val,
        })
    }
}
/// [`KeyValPair`] implementation of [`Into<TokenStream>`]
impl Into<proc_macro2::TokenStream> for KeyValPair {
    fn into(self) -> proc_macro2::TokenStream {
        let key = match self.key {
            Some(x) => x.to_token_stream(),
            None => quote!(),
        };
        let val = match self.val {
            Some(x) => x.to_token_stream(),
            None => quote!(),
        };
        quote!(#key = #val)
    }
}
/// [`KeyValPair`] implementation of [`ToTokens`]
impl ToTokens for KeyValPair {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let token_stream: proc_macro2::TokenStream = self.clone().into();
        tokens.extend(token_stream);
    }
}