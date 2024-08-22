// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

// --------------------------------------------------
// local
// --------------------------------------------------
use super::value::MetaValue;

#[derive(Clone)]
/// [`MetaNameValue`]
/// 
/// Data structure which is consists of a name [`syn::Ident`] 
/// and a value [`syn::Ident`], separated by an equal sign `=`
/// 
/// # Example
/// 
/// ```ignore
/// name = value
/// ```
pub(crate) struct MetaNameValue {
    pub name: syn::Ident,
    sep: syn::Token![=],
    pub value: MetaValue,
}
/// [`MetaNameValue`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaNameValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(MetaNameValue {
            name: input.parse()?,
            sep: input.parse()?,
            value: input.parse()?,
        })
    }
}
/// [`MetaNameValue`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for MetaNameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.name, self.sep.to_token_stream(), self.value.to_token_stream())
    }
}
crate::debug_from_display!(MetaNameValue);

/// A [`MetaNameValue`] wrapper
pub(crate) struct NameValue<T: From<MetaValue> + ToTokens> {
    value: Option<T>,
}
/// [`NameValue`] implementation
impl<T: From<MetaValue> + ToTokens> NameValue<T> {
    pub fn new(value: T) -> Self {
        NameValue { value: Some(value) }
    }
}
/// [`NameValue`] implementation of [`From`] for [`MetaNameValue`]
impl<T: From<MetaValue> + ToTokens> From<&MetaNameValue> for NameValue<T> {
    fn from(meta: &MetaNameValue) -> Self {
        NameValue { value: Some(meta.value.clone().into()) }
    }
}
/// [`NameValue`] implementation of [`Default`]
impl<T: From<MetaValue> + ToTokens> Default for NameValue<T> {
    fn default() -> Self {
        NameValue { value: None }
    }
}
/// [`NameValue`] implementation of [`std::fmt::Display`]
impl<T: From<MetaValue> + ToTokens> std::fmt::Display for NameValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(v) => write!(f, "{}", v.to_token_stream().to_string()),
            None => write!(f, "None"),
        }
    }
}
crate::debug_from_display!(NameValue, From<MetaValue> + ToTokens);