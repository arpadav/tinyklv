// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

#[derive(Clone)]
pub enum MetaValue {
    Lit(syn::Lit),
    Type(syn::Type),
    Path(syn::Path),
    Ident(syn::Ident),
}
/// [`MetaValue`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // --------------------------------------------------
        // - check for types
        // - check for paths
        // - check for literals
        // - check for identifiers
        // --------------------------------------------------
        if let Ok(x) = input.parse::<syn::Type>() { return Ok(MetaValue::Type(x)); }
        if let Ok(x) = input.parse::<syn::Path>() { return Ok(MetaValue::Path(x)); }
        if let Ok(x) = input.parse::<syn::Lit>() { return Ok(MetaValue::Lit(x)); }
        if let Ok(x) = input.parse::<syn::Ident>() { return Ok(MetaValue::Ident(x)); }
        // --------------------------------------------------
        // otherwise return an error
        // --------------------------------------------------
        Err(input.error("Expected a Lit, Type, Path, or Ident"))
    }
}
/// [`MetaValue`] implementation of [`From`]
macro_rules! impl_from_mnv {
    ($t:ty) => {
        #[doc = concat!(" [`MetaValue`] implementation of [`From`] for [`", stringify!($t), "`]")]
        impl From<MetaValue> for $t {
            fn from(x: MetaValue) -> Self {
                match x {
                    MetaValue::Lit(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    MetaValue::Type(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    MetaValue::Path(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                    MetaValue::Ident(x) => syn::parse_str::<$t>(x.to_token_stream().to_string().as_str()).unwrap(),
                }
            }
        }
    };
}
impl_from_mnv!(syn::Lit);
impl_from_mnv!(syn::Type);
impl_from_mnv!(syn::Path);
/// [`MetaValue`] implementation of [`quote::ToTokens`]
impl quote::ToTokens for MetaValue {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            MetaValue::Lit(x) => x.to_tokens(tokens),
            MetaValue::Type(x) => x.to_tokens(tokens),
            MetaValue::Path(x) => x.to_tokens(tokens),
            MetaValue::Ident(x) => x.to_tokens(tokens),
        }
    }
}