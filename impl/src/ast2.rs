// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

/// A [`MetaTuple`] wrapper
pub(crate) struct Tuple<T: From<MetaTuple>> {
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

#[derive(Clone)]
/// [`MetaTuple`]
/// 
/// Data structure which is consists of a name [`syn::Ident`]
/// and listed value(s) [`MetaContents`]
/// 
/// # Example
/// 
/// ```ignore
/// name(a = 1, b(x = 2), c = 3)
/// ```
pub(crate) struct MetaTuple {
    pub name: syn::Ident,
    _paren: syn::token::Paren,
    pub contents: MetaContents,
}
/// [`MetaTuple`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaTuple {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // println!("I AM HERE IN MetaTuple");
        // println!("input: {}", input.to_string());
        // println!("------------------------------------");
        let content;
        Ok(MetaTuple {
            name: input.parse()?, // Parse the identifier before the parentheses
            _paren: syn::parenthesized!(content in input), // Parse the parentheses and the content inside it
            contents: content.parse()?, // Parse the contents inside the parentheses as MetaUnorderedContents
        })
    }
}
/// [`MetaTuple`] implementation of [`From<String>`]
impl From<String> for MetaTuple {
    fn from(s: String) -> Self {
        // Convert the string into a TokenStream
        let inner_tokens: proc_macro2::TokenStream = s.parse().expect("Failed to parse string into TokenStream");
        // Attempt to parse the TokenStream as a MetaTuple
        match syn::parse2::<MetaTuple>(inner_tokens) {
            Ok(metatuple) => metatuple,
            Err(err) => panic!("Failed to parse MetaTuple: {}", err),
        }
    }
}
// /// [`MetaTuple`] implementation of [`Iterator`]
// impl Iterator for MetaTuple {
//     type Item = MetaItem;
//     fn next(&mut self) -> Option<Self::Item> {
//         self.contents.next()
//     }
// }
/// [`MetaTuple`] implementation of [`IntoIterator`]
impl<'a> IntoIterator for &'a MetaTuple {
    type Item = &'a MetaItem;
    type IntoIter = MetaContentsIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.contents.into_iter()
    }
}
/// [`MetaTuple`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for MetaTuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name, self.contents)
    }
}

/// A [`MetaNameValue`] wrapper
pub(crate) struct NameValue<T: From<MetaNameValue>> {
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
macro_rules! impl_from_mnv {
    ($t:ty) => {
        #[doc = concat!(" [`MetaNameValue`] implementation of [`From`] for [`", stringify!($t), "`]")]
        impl From<MetaNameValue> for $t {
            fn from(x: MetaNameValue) -> Self {
                syn::parse_str::<$t>(x.value.to_token_stream().to_string().as_str()).unwrap()
                // syn::parse_str::<$t>(x.value.to_string().as_str()).unwrap()
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
        // println!("I AM HERE IN MetaNameValue");
        // println!("input: {}", input.to_string());
        // println!("------------------------------------");
        Ok(MetaNameValue {
            name: input.parse()?,
            sep: input.parse()?,
            value: input.parse()?,
        })
    }
}
/// [`MetaNameValue`] implementation of [`std::fmt::Debug`]
impl std::fmt::Display for MetaNameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.name, self.sep.to_token_stream(), self.value.to_token_stream())
    }
}

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
            _ if input.peek(syn::Ident) && input.peek2(syn::token::Colon2) => Ok(MetaValue::Path(input.parse()?)),
            _ if input.peek(syn::Ident) && input.peek2(syn::token::Lt) => Ok(MetaValue::Type(input.parse()?)),
            _ if input.peek(syn::Ident) => Ok(MetaValue::Ident(input.parse()?)),
            _ => Err(input.error("Expected a Lit, Type, Path, or Ident")),
        }
    }
}
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

#[derive(Clone)]
/// Enum to handle both [`MetaNameValue`] and [`MetaTuple`]
/// 
/// # Example
/// 
/// ```ignore
/// name = value
/// * OR
/// tname(name = value, name = value)
/// ```
pub(crate) enum MetaItem {
    Tuple(MetaTuple),
    NameValue(MetaNameValue),
}
/// [`MetaItem`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // println!("I AM HERE IN MetaItem::parse");
        // println!("input: {}", input.to_string());
        // println!("------------------------------------");
        // Attempt to parse as MetaTuple first
        if true
        && input.peek(syn::Ident)
        && input.peek2(syn::token::Paren)
        { Ok(MetaItem::Tuple(input.parse()?)) }
        // Otherwise, parse as MetaNameValue
        else { Ok(MetaItem::NameValue(input.parse()?)) }
    }
}
/// [`MetaItem`] implementation of [`std::fmt::Debug`]
impl std::fmt::Display for MetaItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetaItem::Tuple(x) => write!(f, "{}", x),
            MetaItem::NameValue(x) => write!(f, "{}", x),
        }
    }
}

#[derive(Clone, Default)]
/// [`MetaContents`]
/// 
/// Listed contents inside a tuple, delimited
/// by a comma `,`
/// 
/// # Example
/// 
/// ```ignore
/// a = 1, b(x = 2), c = 3
/// ```
pub(crate) struct MetaContents {
    items: syn::punctuated::Punctuated<MetaItem, syn::token::Comma>,
}
/// [`MetaContents`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaContents {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // println!("I AM HERE IN MetaContents");
        // println!("input: {}", input.to_string());
        // println!("------------------------------------");
        Ok(MetaContents {
            items: syn::punctuated::Punctuated::parse_terminated(input)?,
        })
    }
}
/// [`MetaContents`] implementation of [`IntoIterator`]
impl<'a> IntoIterator for &'a MetaContents {
    type Item = &'a MetaItem;
    type IntoIter = MetaContentsIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        MetaContentsIterator::new(&self.items)
    }
}
/// [`MetaContents`] implementation of [`std::fmt::Debug`]
impl std::fmt::Display for MetaContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items: Vec<String> = self.items.iter().map(|item| format!("{}", item)).collect();
        write!(f, "{}", items.join(", "))
    }
}

pub(crate) struct MetaContentsIterator<'a> {
    iter: syn::punctuated::Iter<'a, MetaItem>,
}
impl<'a> MetaContentsIterator<'a> {
    fn new(items: &'a syn::punctuated::Punctuated<MetaItem, syn::token::Comma>) -> Self {
        MetaContentsIterator {
            iter: items.iter(),
        }
    }
}
impl<'a> Iterator for MetaContentsIterator<'a> {
    type Item = &'a MetaItem;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}