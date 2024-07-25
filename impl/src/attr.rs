// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{
    quote,
    ToTokens,
};
use regex::Regex;
use thiserror::Error;
use lazy_static::lazy_static;

// --------------------------------------------------
// constants
// --------------------------------------------------
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
/// [`ListedAttr`]
/// 
/// AKA un-successfully parsed [`syn::MetaList`]
/// 
/// # Example
/// 
/// `#[<path>(<contents>)]`
/// 
/// # Variants
/// 
/// * path: [`syn::Path`]
/// * contents: [`Vec<KeyValPair>`]
pub(crate) struct ListedAttr {
    pub path: syn::Path,
    pub contents: Vec<KeyValPair>
}
/// [`ListedAttr`] implementation
impl ListedAttr {
    /// Creates new [`ListedAttr`]
    pub fn new(s: String) -> Result<Self, ParseError> {
        let s: String = s.chars().filter(|&c| !c.is_whitespace() || c == '\n').collect();
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
        Ok(ListedAttr {
            path: name,
            contents: contents,
        })
    }

    /// Export the path as a [`String`]
    pub fn path(&self) -> String {
        self.path.to_token_stream().to_string()
    }
}
/// [`ListedAttr`] implementation of [`Into<TokenStream>`]
impl Into<proc_macro2::TokenStream> for ListedAttr {
    fn into(self) -> proc_macro2::TokenStream {
        let path = self.path;
        let contents = self.contents;
        quote! { #[#path(#(#contents),*)] }
    }
}
/// [`ListedAttr`] implementation of [`ToTokens`]
impl ToTokens for ListedAttr {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let token_stream: proc_macro2::TokenStream = self.clone().into();
        tokens.extend(token_stream);
    }
}

#[derive(Clone)]
/// Key-Value pair for an attribute input
/// 
/// AKA un-successfully parsed [`syn::MetaNameValue`]
/// 
/// This will correctly parse a non-literal value (raw
/// string without quotes) and convert it to a literal
/// to be used in later parsing.
/// 
/// # Example
/// 
/// `key = val`
/// 
/// # Variants
/// 
/// * key: [`syn::Ident`]
/// * val: [`syn::Lit`]
pub(crate) struct KeyValPair {
    pub key: syn::Ident,
    pub val: syn::Lit,
}
/// [`KeyValPair`] implementation
impl KeyValPair {
    /// Return the key as a [`String`]
    pub fn key(&self) -> String {
        self.key.to_token_stream().to_string()
    }
    /// Return the value as a [`String`]
    pub fn val(&self) -> String {
        self.val.to_token_stream().to_string()
    }
}
/// [`KeyValPair`] implementation of [`Into<TokenStream>`]
impl Into<proc_macro2::TokenStream> for KeyValPair {
    fn into(self) -> proc_macro2::TokenStream {
        let key = self.key.to_token_stream();
        let val = self.val.to_token_stream();
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
            Ok(x) => x,
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
            Ok(x) => x,
            Err(_) =>  match syn::parse_str::<syn::Lit>(&format!("\"{}\"", val)) {
                Ok(x) => x,
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
/// [`KeyValPair`] implementation of [`Into<MetaNameValue>`]
impl Into<syn::MetaNameValue> for KeyValPair {
    fn into(self) -> syn::MetaNameValue {
        syn::MetaNameValue {
            path: self.key.into(),
            eq_token: Default::default(),
            lit: self.val.into(),
        }
    }
}
/// [`KeyValPair`] implementation of [`TryFrom<MetaNameValue>`]
impl TryFrom<syn::MetaNameValue> for KeyValPair {
    type Error = ParseError;
    fn try_from(x: syn::MetaNameValue) -> Result<Self, Self::Error> { 
        let key = match syn::parse_str::<syn::Ident>(&x.path.to_token_stream().to_string().as_str()) {
            Ok(x) => x,
            Err(e) => return Err(ParseError::KeyError(format!("{}", e))),
        };
        Ok(KeyValPair {
            key: key,
            val: x.lit,
        })
    }
}