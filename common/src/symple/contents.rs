//! [`Contents`] + [`MetaContents`] + [`MetaItem`] definitions, implementations, and utils
//! 
//! A [`MetaContents`] is a list of [`MetaItem`]
//! 
//! A [`MetaItem`] can be a [`MetaValue`], [`MetaTuple`], or [`MetaNameValue`]
// --------------------------------------------------
// local
// --------------------------------------------------
use super::tuple::MetaTuple;
use super::value::MetaValue;
use super::nv::MetaNameValue;

#[derive(Clone)]
/// A [`MetaContents`] wrapper, used as a utility for proc-macro parsing
/// 
/// # Example
/// 
/// ```no_run ignore
/// // inside of proc-macro lib
/// struct Input {
///     struct_attributes: symple::Contents<SomeStructToParseTo>
/// }
/// impl From<symple::MetaContents> for SomeStructToParseTo {
///     fn from(meta: symple::MetaContents) -> Self {
///         todo!()
///     }
/// }
/// ```
/// 
/// ***Note that trait bounds for [`From<MetaContents>`] are required
/// for this to work.*** Custom parsing implementations are required.
/// 
/// See [`MetaContents`] for more details and examples
pub struct Contents<T: From<MetaContents>> {
    pub value: Option<T>,
}
/// [`Contents`] implementation of [`std::fmt::Display`]
impl<T: From<MetaContents> + std::fmt::Display> std::fmt::Display for Contents<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        format!("{}", self.value.as_ref().map_or("None".to_string(), |v| v.to_string())).fmt(f)
    }
}
crate::impl_hasvalue!(Contents, From<MetaContents>);
crate::debug_from_display!(Contents, From<MetaContents> + std::fmt::Display);

/// [`Contents`] implementation of [`From`] for [`MetaContents`]
impl<T: From<MetaContents>> From<MetaContents> for Contents<T> {
    fn from(x: MetaContents) -> Self {
        Contents::new(x.into())
    }
}

#[derive(Clone, Default, PartialEq, Eq, Hash)]
/// [`MetaContents`]
/// 
/// Listed contents delimited by a comma `,`
/// 
/// Contents can be of type:
/// 
/// * [`super::MetaValue`] - A single item of any types: [`enum@syn::Lit`], [`syn::Path`], [`syn::Type`], [`struct@syn::Ident`], [`syn::Expr`]
/// * [`super::MetaNameValue`] - A name ([`struct@syn::Ident`]) and value ([`super::MetaValue`])
/// * [`super::MetaTuple`] - A key ([`struct@syn::Ident`]) and value ([`super::MetaContents`])
/// 
/// # Syntax
/// 
/// ```no_run ignore
/// a = 1, b(x = 2), c = 3, ident, "str_literal"
/// ```
/// 
/// # Example
/// 
/// ```
/// use quote::quote;
/// use tinyklv_common::symple;
/// 
/// let input = quote! { "str_literal", a = 1, b(x = 2), c = 3, ident };
/// let meta = syn::parse2::<symple::MetaContents>(input);
/// assert!(meta.is_ok());
/// let meta = meta.unwrap();
/// for (idx, item) in meta.into_iter().enumerate() {
///     match idx {
///         0 => assert_eq!(format!("{}", item), "\"str_literal\""),
///         1 => assert_eq!(format!("{}", item), "a = 1"),
///         2 => assert_eq!(format!("{}", item), "b(x = 2)"),
///         3 => assert_eq!(format!("{}", item), "c = 3"),
///         4 => assert_eq!(format!("{}", item), "ident"),
///         _ => (),
///     }
/// }
/// ```
pub struct MetaContents {
    items: syn::punctuated::Punctuated<MetaItem, syn::token::Comma>,
}
/// [`MetaContents`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaContents {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
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
/// [`MetaContents`] implementation of [`crate::symple::prelude::Merge`]
impl crate::symple::prelude::Merge for MetaContents {
    fn merge(&mut self, other: Self) {
        let set1 = self.items.iter().cloned().collect::<std::collections::HashSet<_>>();
        let set2 = other.items.iter().cloned().collect::<std::collections::HashSet<_>>();
        self.items.extend(set2.difference(&set1).cloned());
    }
}
/// [`MetaContents`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for MetaContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items: Vec<String> = self.items.iter().map(|item| format!("{}", item)).collect();
        write!(f, "{}", items.join(", "))
    }
}
crate::debug_from_display!(MetaContents);

/// [`MetaContents`] implementation of [`From`] for [`MetaTuple`]
impl From<MetaTuple> for MetaContents {
    fn from(x: MetaTuple) -> Self {
        MetaItem::Tuple(x).into()
    }
}
/// [`MetaContents`] implementation of [`From`] for [`MetaItem`]
impl From<MetaItem> for MetaContents {
    fn from(x: MetaItem) -> Self {
        let mut items = syn::punctuated::Punctuated::new();
        items.push_value(x);
        MetaContents { items }
    }
}

#[derive(Clone)]
/// [`MetaContentsIterator`]
/// 
/// Iterator over [`MetaContents`] items
pub struct MetaContentsIterator<'a> {
    iter: syn::punctuated::Iter<'a, MetaItem>,
}
/// [`MetaContentsIterator`] implementation
impl<'a> MetaContentsIterator<'a> {
    fn new(items: &'a syn::punctuated::Punctuated<MetaItem, syn::token::Comma>) -> Self {
        MetaContentsIterator {
            iter: items.iter(),
        }
    }
}
/// [`MetaContentsIterator`] implementation of [`Iterator`]
impl<'a> Iterator for MetaContentsIterator<'a> {
    type Item = &'a MetaItem;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
/// Enum to handle various meta data types
/// 
/// # Example
/// 
/// ```no_run ignore
/// name = value // <- This is a [`MetaNameValue`]
/// // OR
/// tname(name = value, name = value) // <- This is a [`MetaTuple`]
/// // OR
/// value // <- This is a [`MetaValue`]
/// ```
pub enum MetaItem {
    Tuple(MetaTuple),
    Value(MetaValue),
    NameValue(MetaNameValue),
}
/// [`MetaItem`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Attempt to parse as MetaValue
        if input.peek2(syn::token::Comma) {
            return Ok(MetaItem::Value(input.parse()?))
        }
        // Attempt to parse as MetaTuple
        if input.peek(syn::Ident) && input.peek2(syn::token::Paren) {
            match input.parse() {
                Ok(x) => return Ok(MetaItem::Tuple(x)),
                Err(_) => (),
            };
        }
        // Attempt to parse as MetaNameValue
        if input.peek(syn::Ident) && input.peek2(syn::token::Eq) {
            match input.parse() {
                Ok(x) => return Ok(MetaItem::NameValue(x)),
                Err(_) => (),
            };
        }
        // Attempt to parse as MetaValue
        Ok(MetaItem::Value(input.parse()?))
    }
}
/// [`MetaItem`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for MetaItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetaItem::Tuple(x) => write!(f, "{}", x),
            MetaItem::Value(x) => write!(f, "{}", x),
            MetaItem::NameValue(x) => write!(f, "{}", x),
        }
    }
}
crate::debug_from_display!(MetaItem);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symple as symple;

    #[test]
    fn meta_contents_doc() {
        let input = quote::quote! { "str_literal", a = 1, b(x = 2), c = 3, ident, };
        let meta = syn::parse2::<symple::MetaContents>(input);
        assert!(meta.is_ok());
        let meta = meta.unwrap();
        for (idx, item) in meta.into_iter().enumerate() {
            match idx {
                0 => assert_eq!(format!("{}", item), "\"str_literal\""),
                1 => assert_eq!(format!("{}", item), "a = 1"),
                2 => assert_eq!(format!("{}", item), "b(x = 2)"),
                3 => assert_eq!(format!("{}", item), "c = 3"),
                4 => assert_eq!(format!("{}", item), "ident"),
                _ => (),
            }
        }
    }

    #[test]
    fn meta_contents_nested() {
        let input = quote::quote! { a(b(c(d(e = "str_literal")))), b(some_ident, test = 2) };
        let meta = syn::parse2::<symple::MetaContents>(input);
        assert!(meta.is_ok());
        let meta = meta.unwrap();
        for (idx, item) in meta.into_iter().enumerate() {
            match idx {
                0 => assert_eq!(format!("{}", item), "a(b(c(d(e = \"str_literal\"))))"),
                1 => assert_eq!(format!("{}", item), "b(some_ident, test = 2)"),
                _ => (),
            }
        }
    }

    mod contents_doc {
        use super::*;

        #[derive(Default)]
        /// Example struct to be parsed using [`symple`] types
        struct KeysWithValues {
            key: symple::NameValue<syn::Lit>,
            values: Vec<symple::NameValue<syn::Lit>>,
        }

        /// [`KeysWithValues`] implementation of [`std::fmt::Display`]
        impl std::fmt::Display for KeysWithValues {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let values_string: String = self.values.iter().map(|v| format!("{}, ", v.to_string())).collect();
                write!(f, "key: {}\n vals: [ {}]", self.key.to_string(), values_string)
            }
        }
        
        /// [`KeysWithValues`] implementation of [`From`] for [`symple::MetaContents`]
        /// 
        /// This is an example parsing implementation using [`symple`] types
        /// 
        /// See example below, and example in README.md
        impl From<MetaContents> for KeysWithValues {
            fn from(value: MetaContents) -> Self {
                let mut output = Self::default();
                for item in value.into_iter() {
                    match item {
                        symple::MetaItem::NameValue(x) => {
                            if x.name.to_string() == "key" {
                                output.key = x.clone().into()
                            }
                            if x.name.to_string() == "val" {
                                output.values.push(x.clone().into())
                            }
                        },
                        _ => continue,
                    }
                }
                output
            }
        }

        #[test]
        /// Example for doc comments in `contents.rs`
        /// 
        /// Input: `key = 0x01, val=2, val="3", val=0x04`
        fn contents_doc() {
            let input = quote::quote! { key = 0x01, val=2, val="3", val=0x04 };
            let meta = syn::parse2::<symple::MetaContents>(input);
            assert!(meta.is_ok());
            let meta = meta.unwrap();
            let struct_attribute = KeysWithValues::from(meta);
            assert_eq!(format!("{}", struct_attribute), "key: 0x01\n vals: [ 2, \"3\", 0x04, ]");
        }
    }
}