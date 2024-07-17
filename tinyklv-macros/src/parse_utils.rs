// --------------------------------------------------
// external
// --------------------------------------------------
use regex::Regex;
use quote::{format_ident, quote, ToTokens};

// --------------------------------------------------
// constants
// --------------------------------------------------
const ARG_SET_CHAR: char = '=';
const ARGS_DELIM_CHAR: char = ',';
const DEFAULT_ADD_QUOTES: bool = true;

pub(crate) struct KeyValPair {
    pub key: String,
    pub val: String,
    pub delim: String,
    pub add_quotes: bool,
}
impl std::default::Default for KeyValPair {
    fn default() -> Self {
        Self {
            key: String::new(),
            val: String::new(),
            delim: String::from(ARG_SET_CHAR),
            add_quotes: DEFAULT_ADD_QUOTES,
        }
    }
}
impl TryFrom<String> for KeyValPair {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let me = Self::default();
        let mut split = s.split(&me.delim);
        let key = match split.next() {
            Some(x) => x.trim().to_string(),
            None => return Err(String::from("no key found")),
        };
        let val = match split.next() {
            Some(x) => x.trim().to_string(),
            None => return Err(String::from("no val found")),
        };
        Ok(KeyValPair {
            key,
            val,
            delim: me.delim,
            add_quotes: me.add_quotes,
        })
    }
}
impl Into<String> for KeyValPair {
    fn into(self) -> String {
        match self.add_quotes {
            true => match syn::parse_str::<syn::Lit>(&self.val) {
                Ok(_) => format!("{} {} {}", self.key, self.delim, self.val),
                Err(_) => format!("{} {} \"{}\"", self.key, self.delim, self.val)
            },
            false => format!("{} {} {}", self.key, self.delim, self.val),
        }
    }
}

pub(crate) struct AttrArgs {
    pub elems: Vec<String>,
    pub delim: String,
}
impl AttrArgs {
    pub fn add_val_quotes(mut self) -> Result<Self, String> {
        for e in self.elems.iter_mut() {
            let kv = match KeyValPair::try_from(e.clone()) {
                Ok(kv) => kv,
                Err(e) => return Err(e),
            };
            *e = kv.into();
        }
        Ok(self)
    }
}
impl std::default::Default for AttrArgs {
    fn default() -> Self {
        Self {
            elems: Vec::new(),
            delim: String::from(ARGS_DELIM_CHAR),
        }
    }
}
impl From<String> for AttrArgs {
    fn from(s: String) -> Self {
        let me = Self::default();
        AttrArgs {
            elems: s
                .split(&me.delim)
                .map(|x| x.trim().into())
                .collect(),
            delim: me.delim,
        }
    }
}
impl Into<String> for AttrArgs {
    fn into(self) -> String {
        self.elems.join(&format!("{} ", self.delim.trim()))
    }
}

pub(crate) struct StructAttribute {
    pub name: String,
    pub contents: String,
}
impl TryFrom<String> for StructAttribute {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        let name_re = Regex::new(r"#\[(.*?)\(.*\)\]").unwrap();
        let name = match name_re.captures(&s) {
            Some(captures) => match captures.get(1) {
                Some(inner) => inner,
                None => return Err(format!("Can not match `s`: {:#?} to find name", s)),
            },
            None => return Err(format!("Can not match `s`: {:#?} name", s)),
        };
        let contents_re = Regex::new(r"#\[.*\((.*?)\)\]").unwrap();
        let contents = match contents_re.captures(&s) {
            Some(captures) => match captures.get(1) {
                Some(inner) => inner,
                None => return Err(format!("Can not match `s`: {:#?} to find contents", s)),
            },
            None => return Err(format!("Can not match `s`: {:#?} contents", s)),
        };
        Ok(StructAttribute {
            name: name.as_str().to_string(),
            contents: contents.as_str().to_string(),
        })
    }
}
impl TryFrom<&syn::Attribute> for StructAttribute {
    type Error = String;
    fn try_from(attr: &syn::Attribute) -> Result<Self, Self::Error> {
        let s = attr.to_token_stream().to_string();
        Self::try_from(s)
    }
}
impl StructAttribute {
    pub fn add_val_quotes(mut self) -> Result<Self, String> {
        self.contents = AttrArgs::from(self.contents.clone())
            .add_val_quotes()?
            .into();
        Ok(self)
    }
    pub fn reconstruct(&self) -> proc_macro2::TokenStream {
        // let name = format_ident!("{}", self.name);
        let something = format_args!("{}({})", self.name, self.contents);
        // let contents = format_ident!("{}", self.contents);
        // let result = format_ident!("{}({})", name, contents);
        quote! { #[#something] }
        // quote! { #[#name(#contents)] }
        // quote! { #[#self.name(#self.contents)] }
        // format!("#[{}({})]", self.name, self.contents)
    }
    pub fn as_attr(&self) -> syn::Attribute {
        // let self_tokens = self.reconstruct().to_token_stream();
        let self_tokens2 = self.reconstruct();
        // let self_tokens2 = quote! { #self_tokens2 };
        println!("self_tokens2: {}", self_tokens2.to_string());
        // println!("self: {}", self.reconstruct());
        // println!("self_tokens: {:?}", self_tokens);
        // println!("self_tokens: {}", self_tokens.to_string());
        // let test_self_tokens: proc_macro2::TokenStream = quote::quote! {
        //     #[key_encoder(func = "key_encoder")]
        // };
        // println!("self_tokens: {}", self_tokens.to_string());
        // println!("test_self_tokens: {}", test_self_tokens.to_string());
        // assert_eq!(self_tokens.to_string(), test_self_tokens.to_string());
        // println!("after assert");
        // let test_result: syn::Attribute = syn::parse_quote! {
        //     #test_self_tokens
        // };
        // println!("test_result: {}", test_result.to_token_stream().to_string());
        let result: syn::Attribute = syn::parse_quote! {
            #self_tokens2
        };
        println!("result: {}", result.to_token_stream().to_string());
        result
    }
}