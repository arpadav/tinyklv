// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

#[derive(Clone)]
pub(crate) enum MetaValue {
    Lit(syn::Lit),
    Type(syn::Type),
    Path(syn::Path),
    Ident(syn::Ident),
}
/// [`MetaValue`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        match () {
            _ if input.peek(syn::Lit) => Ok(MetaValue::Lit(input.parse()?)),
            _ if (input.peek(syn::Ident) && input.peek2(syn::token::Colon2))
              || (input.peek3(syn::Ident) && input.peek(syn::token::Colon2)) => match input.parse::<syn::Type>() {
                Ok(x) => Ok(MetaValue::Type(x)),
                _ => Ok(MetaValue::Path(input.parse()?)),
            },
            _ if input.peek(syn::Ident) && input.peek2(syn::token::Lt) => Ok(MetaValue::Type(input.parse()?)),
            _ if input.peek(syn::Ident) => Ok(MetaValue::Ident(input.parse()?)),
            _ => Err(input.error("Expected a Lit, Type, Path, or Ident")),
        }
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