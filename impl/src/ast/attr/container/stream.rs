use quote::ToTokens;

use crate::ast::symbol;
use crate::ast::types::TypeType;

pub(crate) struct Stream(pub Option<TypeType>);

impl std::fmt::Debug for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_token_stream())
    }
}

impl TryFrom<&syn::MetaNameValue> for Stream {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaNameValue) -> syn::Result<Self> {
        match symbol::nv_parse_maybestr_stream(input) {
            // `stream` keyword is detected, and value is parsed correctly
            Some(Ok(x)) => Ok(Stream(Some(x))),
            // `stream` keyword is detected, but value is not parsed correctly
            Some(Err(err)) => Err(err),
            // `stream` keyword is not detected
            None => Ok(Stream(None)),
        }
    }
}
