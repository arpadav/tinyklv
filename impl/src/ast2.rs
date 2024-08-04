use core::str;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use syn::spanned::Spanned;

/// A [`MetaTuple`] wrapper
pub struct Tuple<T: From<MetaTuple>> {
    _marker: std::marker::PhantomData<T>,
}
/// [`Tuple`] implementation of [`Default`]
impl<T: From<MetaTuple>> Default for Tuple<T> {
    fn default() -> Self {
        Tuple {
            _marker: std::marker::PhantomData,
        }
    }
}

/// [`MetaTupleStatus`] to indicate whether
/// an input token stream or string is expected to be a 
/// [`MetaTuple`]. If not, will return [`MetaTupleStatus::None`]
/// 
/// Used in [`MetaTuple::status`]
enum MetaTupleStatus {
    Partial,
    Complete,
    None,
}

/// [`MetaTuple`]
/// 
/// Data structure which is consists of a name [`syn::Ident`]
/// and listed value(s) [`MetaUnorderedContents`]
/// 
/// # Example
/// 
/// ```ignore
/// name(a = 1, b(x = 2), c = 3)
/// ```
pub struct MetaTuple {
    pub n: syn::Ident,
    pub v: MetaTupleContents,
}
/// [`MetaTuple`] implementation
impl MetaTuple {
    /// Parses the input by removing whitespace
    fn pinput(input: impl Into<String>) -> String {
        input
            .into()
            .chars()
            .filter(|&c| !c.is_whitespace() || c == '\n')
            .collect()
    }

    /// Status of parsing when splitting using
    /// [`MetaUnorderedContents::parse`]
    fn status(input: impl Into<String>) -> MetaTupleStatus {
        let (_, input) = Self::pop_name(input);
        let rparen = input.chars().filter(|c| *c == char::from(b"("[0])).count();
        let lparen = input.chars().filter(|c| *c == char::from(b")"[0])).count();
        match (rparen, lparen, rparen == lparen, rparen > lparen) {
            (_, _, true, _) => MetaTupleStatus::Complete,
            (_, _, _, true) => MetaTupleStatus::Partial,
            (0, 0, _, _) => MetaTupleStatus::None,
            (_, _, _, false) => MetaTupleStatus::None,
        }
    }

    /// Pop's the name of the to-be-parsed
    /// [`MetaTuple`], if it exists
    fn pop_name(input: impl Into<String>) -> (Option<String>, String) {
        let input = Self::pinput(input);
        if !input.contains("(") { return (None, input) };
        match input.split_once("(") {
            Some((name, input)) => (Some(name.into()), input.into()),
            None => (None, input)
        }
    }
}
/// [`MetaTuple`] implementation of [`TryFrom`] for [`syn::Attribute`]
impl TryFrom<syn::Attribute> for MetaTuple {
    type Error = syn::Error;
    fn try_from(value: syn::Attribute) -> Result<Self, Self::Error> {
        let n = match value.path.get_ident() {
            Some(i) => i,
            None => return Err(syn::Error::new(value.span(), "expected ident")),
        };
        Ok(MetaTuple {
            n: n.clone(),
            v: syn::parse2::<MetaTupleContents>(value.tokens)?,
        })
    }
}
// /// [`MetaTuple`] implementation of [`TryFrom`] for [`ToTokens`]
// impl<T: ToTokens> TryFrom<T> for MetaTuple {
//     type Error = syn::Error;
//     fn try_from(value: T) -> Result<Self, Self::Error> {
//         let value = value.to_token_stream();
//         syn::parse2::<MetaTuple>(value)
//     }
// }
/// [`MetaTuple`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaTuple {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = Self::pop_name(input.clone());
        let contents;
        syn::parenthesized!(contents in input);
    }
}

/// [`MetaTupleContents`]
/// 
/// Data structure which is of listed value(s) [`MetaUnorderedContents`]
/// wrapped with parentheses
/// 
/// # Example
/// 
/// ```ignore
/// (a = 1, b(x = 2), c = 3)
/// ```
struct MetaTupleContents {
    pub v: MetaUnorderedContents,
}
/// [`MetaTupleContents`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaTupleContents {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);
        let v = content.parse::<MetaUnorderedContents>()?;
        Ok(MetaTupleContents { v })
    }
}

/// A [`MetaNameValue`] wrapper
pub struct NameValue<T: From<MetaNameValue>> {
    _marker: std::marker::PhantomData<T>,
}
/// [`NameValue`] implementation of [`Default`]
impl<T: From<MetaNameValue>> Default for NameValue<T> {
    fn default() -> Self {
        NameValue {
            _marker: std::marker::PhantomData,
        }
    }
}

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
/// Listed contents inside a tuple, delimited
/// by a comma `,`
/// 
/// # Example
/// 
/// ```ignore
/// a = 1, b(x = 2), c = 3
/// ```
pub struct MetaUnorderedContents {
    pub nvs: Vec<MetaNameValue>,
    pub tps: Vec<MetaTuple>,
    sep: syn::token::Comma,
}
/// [`MetaUnorderedContents`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaUnorderedContents {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut result = MetaUnorderedContents::default();
        let sep = result
            .sep
            .to_token_stream()
            .to_string();
        let second = input.to_string();
        let mut prev = String::new();
        loop {
            let (first, second) = Self::step(second.as_ref(), sep.as_str());
            prev.push_str(first);
            match MetaTuple::status(prev) {
                MetaTupleStatus::Complete => {
                    let item = syn::parse2::<MetaTuple>(prev.to_token_stream())?;
                    prev.clear();
                    result.tps.push(item)
                },
                MetaTupleStatus::Partial => prev.push_str(first),
                MetaTupleStatus::None => {
                    let item = MetaNameValue::try_from(prev)?;
                    prev.clear();
                    result.nvs.push(item)
                },
            }
            if second.is_empty() { break; }
        }
        // proc_macro2::TokenTree
        Err(syn::Error::new(input.span(), "msg"))
    }
}
/// [`MetaUnorderedContents`] implementation
impl MetaUnorderedContents {
    fn step<'a>(input: &'a str, sep: &'static str) -> (&'a str, &'a str) {
        match input.split_once(sep) {
            Some((x, y)) => (x, y),
            None => (input, "")
        }
    }
}