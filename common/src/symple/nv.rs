//! [`NameValue`] and [`MetaNameValue`] definitions, implementations, and utils
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

// --------------------------------------------------
// local
// --------------------------------------------------
use super::value::MetaValue;

#[derive(Clone)]
/// A [`MetaNameValue`] wrapper, used as a utility for proc-macro parsing
/// 
/// # Example
/// 
/// ```no_run
/// // inside of proc-macro lib
/// struct Input {
///     struct_attribute_identifier: symple::NameValue<syn::Lit>
/// }
/// ```
/// 
/// ***Note that trait bounds for [`From<MetaValue>`] are required
/// for this to work.*** Custom implementations are possible, but currently
/// [`From<MetaValue>`] is implemented for:
/// 
/// * [`enum@syn::Lit`]
/// * [`syn::Type`]
/// * [`syn::Path`]
/// * [`syn::Expr`]
/// * [`struct@syn::Ident`]
/// 
/// ```no_run
/// // outisde of proc-macro lib
/// #[`derive(MyProcMacro)`]
/// #[`identifier = "Hello World!"`]
/// struct SomeStruct;
/// ```
/// 
/// Which can then be parsed using the [`From<MetaValue>`] implementation
/// into the following, to help with proc-macro parsing:
/// 
/// ```no_run
/// Input {
///     // note that this is called using `struct_attribute_identifier.value`
///     struct_attribute_identifier: Some(syn::Lit::new(syn::IntSuffix::None, "Hello World!")),
/// }
/// ```
pub struct NameValue<T: From<MetaValue> + ToTokens> {
    pub value: Option<T>,
}
/// [`NameValue`] implementation of [`Default`]
impl<T: From<MetaValue> + ToTokens> Default for NameValue<T> {
    fn default() -> Self {
        NameValue { value: None }
    }
}
crate::impl_hasvalue!(NameValue, From<MetaValue> + ToTokens);
crate::debug_from_display!(NameValue, From<MetaValue> + ToTokens + std::fmt::Display);

/// [`NameValue`] implementation of [`From`] for [`MetaNameValue`]
impl<T: From<MetaValue> + ToTokens> From<MetaNameValue> for NameValue<T> {
    fn from(x: MetaNameValue) -> Self {
        NameValue::new(x.value.into())
    }
}
/// [`NameValue`] implementation of [`From`] for [`MetaValue<T>`]
impl<T: From<MetaValue> + ToTokens> From<MetaValue> for NameValue<T> {
    fn from(x: MetaValue) -> Self {
        NameValue::new(x.into())
    }
}

#[derive(Clone)]
/// [`MetaNameValue`]
/// 
/// Data structure which is consists of a name [`struct@syn::Ident`] 
/// and a value [`struct@syn::Ident`], separated by an equal sign `=`
/// 
/// # Example
/// 
/// ```no_run
/// name = value
/// ```
pub struct MetaNameValue {
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

/// [`MetaNameValue`] implementation of [`From`]
macro_rules! impl_from_mnv {
    ($t:ty) => {
        impl_from_mnv!($t, "");
    };
    ($t:ty, $prefix:expr) => {
        #[doc = concat!(" [`MetaNameValue`] implementation of [`From`] for [`", stringify!($prefix), stringify!($t), "`]")]
        impl From<MetaNameValue> for $t {
            fn from(x: MetaNameValue) -> Self {
                x.value.into()
            }
        }
    };
}
impl_from_mnv!(syn::Lit, "enum@");
impl_from_mnv!(syn::Path);
impl_from_mnv!(syn::Expr);
impl_from_mnv!(syn::Type);
// impl_from_mnv!(syn::Ident); // do NOT UNCOMMENT