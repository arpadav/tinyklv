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
/// [`MetaTuple`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaTuple {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let (n, input2) = Self::pop_name(input.to_string());
        let n = match n {
            Some(name) => syn::parse2::<syn::Ident>(name.to_token_stream())?,
            None => return Err(syn::Error::new(input.span(), "msg2")),
        };
        let v = syn::parse2::<MetaTupleContents>(input2.to_token_stream())?;
        Ok(MetaTuple { n, v })
    }
}
/// [`MetaTuple`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for MetaTuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaTuple")
            .field("n", &self.n)
            .field("v", &self.v)
            .finish()
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
pub struct MetaTupleContents {
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
/// [`MetaTupleContents`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for MetaTupleContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaTupleContents")
            .field("v", &self.v)
            .finish()
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
    eq: syn::token::Eq,
    pub v: syn::Ident,
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
/// [`MetaNameValue`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaNameValue {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(MetaNameValue {
            n: input.parse()?,
            eq: input.parse()?,
            v: input.parse()?,
        })
    }
}
/// [`MetaNameValue`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for MetaNameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaNameValue")
            .field("n", &self.n)
            .field("eq", &"=")
            .field("v", &self.v)
            .finish()
    }
}

/// // Parse a simplified tuple struct syntax like:
/// //
/// //     struct S(A, B);
/// struct TupleStruct {
///     struct_token: Token![struct],
///     ident: Ident,
///     paren_token: token::Paren,
///     fields: Punctuated<Type, Token![,]>,
///     semi_token: Token![;],
/// }
///
/// impl Parse for TupleStruct {
///     fn parse(input: ParseStream) -> Result<Self> {
///         let content;
///         Ok(TupleStruct {
///             struct_token: input.parse()?,
///             ident: input.parse()?,
///             paren_token: parenthesized!(content in input),
///             fields: content.parse_terminated(Type::parse)?,
///             semi_token: input.parse()?,
///         })
///     }
/// }


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
            let (first, second) = Self::step(second.as_ref(), sep.as_ref());
            prev.push_str(&sep);
            prev.push_str(first);
            match MetaTuple::status(prev.clone()) {
                MetaTupleStatus::Complete => {
                    let item = syn::parse2::<MetaTuple>(prev.to_token_stream())?;
                    prev.clear();
                    result.tps.push(item)
                },
                MetaTupleStatus::Partial => prev.push_str(first),
                MetaTupleStatus::None => {
                    let item = syn::parse2::<MetaNameValue>(prev.to_token_stream())?;
                    prev.clear();
                    result.nvs.push(item)
                },
            }
            if second.is_empty() { break; }
        }
        println!("result name values: {:#?}", result.nvs);
        println!("result tuples: {:#?}", result.tps);
        // proc_macro2::TokenTree
        Err(syn::Error::new(input.span(), "msg"))
    }
}
/// [`MetaUnorderedContents`] implementation
impl MetaUnorderedContents {
    fn step<'a, 'b>(input: &'a str, sep: &'b str) -> (&'a str, &'a str) {
        match input.split_once(sep) {
            Some((x, y)) => (x, y),
            None => (input, "")
        }
    }
}
/// [`MetaUnorderedContents`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for MetaUnorderedContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetaUnorderedContents")
            .field("nvs", &self.nvs)
            .field("tps", &self.tps)
            .finish()
    }
}