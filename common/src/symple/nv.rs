//! [NameValue] and [MetaNameValue] definitions, implementations, and utils
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

// --------------------------------------------------
// local
// --------------------------------------------------
use super::value::MetaValue;

#[derive(Clone)]
/// A [MetaNameValue] wrapper, used as a utility for proc-macro parsing
/// 
/// # Example
/// 
/// ```ignore
/// // inside of proc-macro lib
/// struct Input {
///     struct_attribute_identifier: symple::NameValue<syn::Lit>
/// }
/// ```
/// 
/// ***Note that trait bounds for [From<MetaValue>] are required
/// for this to work.*** Custom implementations are possible, but currently
/// [From<MetaValue>] is implemented for:
/// 
/// * [syn::Lit]
/// * [syn::Type]
/// * [syn::Path]
/// * [syn::Expr]
/// * [syn::Ident]
/// 
/// ```ignore
/// // outisde of proc-macro lib
/// #[derive(MyProcMacro)]
/// #[identifier = "Hello World!"]
/// struct SomeStruct;
/// ```
/// 
/// Which can then be parsed using the [From<MetaValue>] implementation
/// into the following, to help with proc-macro parsing:
/// 
/// ```ignore
/// Input {
///     // note that this is called using `struct_attribute_identifier.value`
///     struct_attribute_identifier: Some(syn::Lit::new(syn::IntSuffix::None, "Hello World!")),
/// }
/// ```
pub struct NameValue<T: From<MetaValue> + ToTokens> {
    pub value: Option<T>,
}
/// [NameValue] implementation
impl<T: From<MetaValue> + ToTokens> NameValue<T> {
    #[allow(dead_code)]
    pub fn new(value: T) -> Self {
        NameValue { value: Some(value) }
    }
}
/// [NameValue] implementation of [Default]
impl<T: From<MetaValue> + ToTokens> Default for NameValue<T> {
    fn default() -> Self {
        NameValue { value: None }
    }
}
/// [NameValue] implementation of [std::fmt::Display]
impl<T: From<MetaValue> + ToTokens> std::fmt::Display for NameValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(v) => write!(f, "{}", v.to_token_stream().to_string()),
            None => write!(f, "None"),
        }
    }
}
crate::debug_from_display!(NameValue, From<MetaValue> + ToTokens);

/// [NameValue] implementation of [From] for [MetaNameValue]
impl<T: From<MetaValue> + ToTokens> From<MetaNameValue> for NameValue<T> {
    fn from(x: MetaNameValue) -> Self {
        NameValue::new(x.value.into())
    }
}
/// [NameValue] implementation of [From] for [MetaValue<T>]
impl<T: From<MetaValue> + ToTokens> From<MetaValue> for NameValue<T> {
    fn from(x: MetaValue) -> Self {
        NameValue::new(x.into())
    }
}

#[derive(Clone)]
/// [MetaNameValue]
/// 
/// Data structure which is consists of a name [syn::Ident] 
/// and a value [syn::Ident], separated by an equal sign `=`
/// 
/// # Example
/// 
/// ```ignore
/// name = value
/// ```
pub struct MetaNameValue {
    pub name: syn::Ident,
    sep: syn::Token![=],
    pub value: MetaValue,
}
/// [MetaNameValue] implementation of [syn::parse::Parse]
impl syn::parse::Parse for MetaNameValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(MetaNameValue {
            name: input.parse()?,
            sep: input.parse()?,
            value: input.parse()?,
        })
    }
}
/// [MetaNameValue] implementation of [std::fmt::Display]
impl std::fmt::Display for MetaNameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.name, self.sep.to_token_stream(), self.value.to_token_stream())
    }
}
crate::debug_from_display!(MetaNameValue);