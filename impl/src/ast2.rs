pub struct Tuple<T: From<MetaTuple>> {
    _marker: std::marker::PhantomData<T>,
}
pub struct MetaTuple {
    pub n: syn::Ident,
    pub v: MetaUnorderedContents,
    outer: syn::token::Paren,
}

pub struct NameValue<T: From<MetaNameValue>> {
    _marker: std::marker::PhantomData<T>,
}
pub struct MetaNameValue {
    pub n: syn::Ident,
    pub v: syn::Ident,
    eq: syn::token::Eq,
}
macro_rules! impl_from_mnv {
    ($t:ty) => {
        #[doc = concat!(" [`MetaNameValue`] implementation of [`From`] for [`", stringify!($t), "`]")]
        impl From<MetaNameValue> for $t {
            fn from(x: MetaNameValue) -> Self {
                syn::parse_str::<$t>(x.v.to_string().as_str()).unwrap()
            }
        }
    };
}
impl_from_mnv!(syn::Lit);
impl_from_mnv!(syn::Path);
impl_from_mnv!(syn::Type);

#[derive(Default)]
/// [`MetaUnorderedContents`]
/// 
/// Contents inside a tuple
pub struct MetaUnorderedContents {
    pub nvs: Vec<MetaNameValue>,
    pub tps: Vec<MetaTuple>,
    sep: syn::token::Comma,
}