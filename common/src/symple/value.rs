//! [`MetaValue`] definitions, implementations, and utils
//! 
//! A [`MetaValue`] can be either a [`enum@syn::Lit`], [`syn::Type`], [`syn::Path`], [`syn::Expr`], or [`struct@syn::Ident`]
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

#[derive(Clone)]
/// [`Value`], which can be [`enum@syn::Lit`], [`syn::Type`], [`syn::Path`], [`syn::Expr`], or [`struct@syn::Ident`]
pub struct Value<T: From<MetaValue>> {
    pub value: Option<T>,
}
/// [`Value`] implementation of [`std::fmt::Display`]
impl<T: From<MetaValue> + std::fmt::Display> std::fmt::Display for Value<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!("{}", self.value.as_ref().map_or("None".to_string(), |x| x.to_string())).fmt(f)
    }
}
crate::impl_hasvalue!(Value, From<MetaValue>);
crate::debug_from_display!(Value, From<MetaValue> + std::fmt::Display);

/// [`Value`] implementation of [`From`] for [`MetaValue`]
impl<T: From<MetaValue>> From<MetaValue> for Value<T> {
    fn from(x: MetaValue) -> Self {
        Value::new(x.into())
    }
}

#[derive(Clone, Debug)]
/// [`MetaValue`], which can be [`enum@syn::Lit`], [`syn::Type`], [`syn::Path`], or [`struct@syn::Ident`]
pub enum MetaValue {
    Lit(syn::Lit),
    Path(syn::Path),
    Expr(syn::Expr),
    Type(syn::Type),
    Ident(syn::Ident),
    Macro(syn::Macro),
}
/// [`MetaValue`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // --------------------------------------------------
        // prioritize macro: check for a path followed by '!'
        // --------------------------------------------------
        let fork = input.fork();
        if let Ok(_) = fork.parse::<syn::Path>() {
            if fork.peek(syn::Token![!]) {
                let macro_: syn::Macro = input.parse()?;
                return Ok(MetaValue::Macro(macro_));
            }
        }
        drop(fork);
        // --------------------------------------------------
        // prioritize expressions
        // --------------------------------------------------
        if input.peek(syn::token::Paren) || input.peek(syn::token::Brace) || input.peek(syn::Token![if]) || input.peek(syn::Token![match]) {
            if let Ok(x) = input.parse::<syn::Expr>() {
                return Ok(MetaValue::Expr(x));
            }
        }
        // --------------------------------------------------
        // then prioritize literals
        // --------------------------------------------------
        if input.peek(syn::Lit) {
            if let Ok(x) = input.parse::<syn::Lit>() {
                return Ok(MetaValue::Lit(x));
            }
        }
        // --------------------------------------------------
        // check for paths after types
        // --------------------------------------------------
        if let Ok(x) = input.parse::<syn::Path>() {
            return Ok(MetaValue::Path(x));
        }
        // --------------------------------------------------
        // check for types after expressions and literals
        // --------------------------------------------------
        if let Ok(x) = input.parse::<syn::Type>() {
            return Ok(MetaValue::Type(x));
        }
        // --------------------------------------------------
        // check for identifiers as a last resort
        // --------------------------------------------------
        if let Ok(x) = input.parse::<syn::Ident>() {
            return Ok(MetaValue::Ident(x));
        }
        // --------------------------------------------------
        // otherwise return an error
        // --------------------------------------------------
        Err(input.error("Expected a Expr, Lit, Type, Path, Ident, or Macro call"))
    }
}
/// [`MetaValue`] implementation of [`From`]
macro_rules! impl_from_mv {
    ($t:ty) => {
        impl_from_mv!($t, "");
    };
    ($t:ty, $prefix:expr) => {
        #[doc = concat!(" [`MetaValue`] implementation of [`From`] for [`", stringify!($prefix), stringify!($t), "`]")]
        impl From<MetaValue> for $t {
            fn from(x: MetaValue) -> Self {
                match x {
                    MetaValue::Lit(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    MetaValue::Path(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    MetaValue::Expr(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    MetaValue::Type(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    MetaValue::Ident(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    MetaValue::Macro(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    // MetaValue::MacroCall(x) => syn::parse_str::<$t>(x.to_string().as_str()).unwrap(),
                }
            }
        }
    };
}
impl_from_mv!(syn::Lit, "enum@");
impl_from_mv!(syn::Path);
impl_from_mv!(syn::Expr);
impl_from_mv!(syn::Type);
// impl_from_mv!(syn::Ident); // do NOT UNCOMMENT

/// [`MetaValue`] implementation of [`quote::ToTokens`]
impl quote::ToTokens for MetaValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            MetaValue::Lit(x) => x.to_tokens(tokens),
            MetaValue::Path(x) => x.to_tokens(tokens),
            MetaValue::Expr(x) => x.to_tokens(tokens),
            MetaValue::Type(x) => x.to_tokens(tokens),
            MetaValue::Ident(x) => x.to_tokens(tokens),
            MetaValue::Macro(x) => x.to_tokens(tokens),
            // MetaValue::MacroCall(x) => x.to_tokens(tokens),
        }
    }
}
/// [`MetaValue`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for MetaValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetaValue::Lit(x) => std::fmt::Display::fmt(&x.to_token_stream().to_string(), f),
            MetaValue::Path(x) => std::fmt::Display::fmt(&x.to_token_stream().to_string(), f),
            MetaValue::Expr(x) => std::fmt::Display::fmt(&x.to_token_stream().to_string(), f),
            MetaValue::Type(x) => std::fmt::Display::fmt(&x.to_token_stream().to_string(), f),
            MetaValue::Ident(x) => std::fmt::Display::fmt(&x.to_string(), f),
            MetaValue::Macro(x) => std::fmt::Display::fmt(&x.to_token_stream().to_string(), f),
            // MetaValue::MacroCall(x) => std::fmt::Display::fmt(&x.to_string(), f),
        }
    }
}