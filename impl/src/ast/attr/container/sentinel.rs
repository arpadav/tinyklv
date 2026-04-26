use quote::ToTokens;

use crate::ast::symbol;

pub(crate) struct Sentinel(pub Option<syn::Lit>);

impl std::fmt::Debug for Sentinel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_token_stream())
    }
}

impl TryFrom<&syn::MetaNameValue> for Sentinel {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaNameValue) -> syn::Result<Self> {
        match symbol::parse_nv_sentinel(input) {
            // `sentinel` keyword is detected, and value is parsed correctly
            Some(Ok(x)) => Ok(Sentinel(Some(x))),
            // `sentinel` keyword is detected, but value is not parsed correctly
            Some(Err(err)) => Err(err),
            // `sentinel` keyword is not detected
            None => Ok(Sentinel(None)),
        }
    }
}
