// --------------------------------------------------
// local
// --------------------------------------------------
use super::item::MetaItem;

#[derive(Clone, Default)]
// TODO: Create a wrapper to be used as [symple::Contents]
pub struct Contents<T>
where
    T: std::fmt::Display,
    T: From<MetaContents>,
{
    pub value: Option<T>,
}
/// [Contents] implementation of [From<MetaContents>]
impl<T> From<&MetaContents> for Contents<T>
where
    T: std::fmt::Display,
    T: From<MetaContents>,
{
    fn from(meta: &MetaContents) -> Self {
        Contents {
            value: Some(T::from(meta.clone())),
        }
    }
}
/// [Contents] implementation of [std::fmt::Display]
impl<T> std::fmt::Display for Contents<T>
where
    T: std::fmt::Display,
    T: From<MetaContents>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value.as_ref().unwrap())
    }   
}
crate::debug_from_display!(Contents, From<MetaContents> + std::fmt::Display);

#[derive(Clone, Default)]
/// [MetaContents]
/// 
/// Listed contents delimited by a comma `,`
/// 
/// Contents can be of type:
/// 
/// * [super::MetaValue] - A single item of any types: [syn::Lit], [syn::Path], [syn::Type], [syn::Ident]
/// * [super::MetaNameValue] - A name ([syn::Ident]) and value ([super::MetaValue])
/// * [super::MetaTuple] - A key ([syn::Ident]) and value ([super::MetaContents])
/// 
/// # Syntax
/// 
/// ```ignore
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
/// [MetaContents] implementation of [syn::parse::Parse]
impl syn::parse::Parse for MetaContents {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(MetaContents {
            items: syn::punctuated::Punctuated::parse_terminated(input)?,
        })
    }
}
/// [MetaContents] implementation of [IntoIterator]
impl<'a> IntoIterator for &'a MetaContents {
    type Item = &'a MetaItem;
    type IntoIter = MetaContentsIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        MetaContentsIterator::new(&self.items)
    }
}
/// [MetaContents] implementation of [std::fmt::Display]
impl std::fmt::Display for MetaContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items: Vec<String> = self.items.iter().map(|item| format!("{}", item)).collect();
        write!(f, "{}", items.join(", "))
    }
}
crate::debug_from_display!(MetaContents);

#[derive(Clone)]
/// [MetaContentsIterator]
/// 
/// Iterator over [MetaContents] items
pub struct MetaContentsIterator<'a> {
    iter: syn::punctuated::Iter<'a, MetaItem>,
}
/// [MetaContentsIterator] implementation
impl<'a> MetaContentsIterator<'a> {
    fn new(items: &'a syn::punctuated::Punctuated<MetaItem, syn::token::Comma>) -> Self {
        MetaContentsIterator {
            iter: items.iter(),
        }
    }
}
/// [MetaContentsIterator] implementation of [Iterator]
impl<'a> Iterator for MetaContentsIterator<'a> {
    type Item = &'a MetaItem;
    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

#[cfg(test)]
mod tests {
    use crate::symple::NameValue;

    use super::*;

    #[test]
    fn meta_contents_doc() {
        use quote::quote;
        let input = quote! { "str_literal", a = 1, b(x = 2), c = 3, ident, };
        let meta = syn::parse2::<MetaContents>(input);
        assert!(meta.is_ok());
        let meta = meta.unwrap();
        for (idx, item) in meta.into_iter().enumerate() {
            match idx {
                0 => assert_eq!(format!("{}", item), "\"value\""),
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
        use quote::quote;
        let input = quote! { a(b(c(d(e = "str_literal")))), b(some_ident, test = 2) };
        let meta = syn::parse2::<MetaContents>(input);
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
        struct KeysWithValues {
            key: NameValue<syn::Lit>,
            values: Vec<NameValue<syn::Lit>>,
        }
        impl std::fmt::Display for KeysWithValues {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let values_string: String = self.values.iter().map(|v| format!("{}, ", v.to_string())).collect();
                write!(f, "key: {}\n vals: [ {}]", self.key, values_string)
            }
        }
        impl From<MetaContents> for KeysWithValues {
            fn from(value: MetaContents) -> Self {
                let mut output = Self::default();
                for item in value.into_iter() {
                    match item {
                        MetaItem::NameValue(x) => {
                            if x.name.to_string() == "key" {
                                output.key = NameValue::new(x.value.clone().into())
                            }
                            if x.name.to_string() == "val" {
                                output.values.push(NameValue::new(x.value.clone().into()))
                            }
                        },
                        _ => continue,
                    }
                }
                output
            }
        }

        #[test]
        fn contents_doc() {
            use quote::quote;
            let input = quote! { key = 0x01, val=2, val="3", val=0x04 };
            let meta = syn::parse2::<MetaContents>(input);
            assert!(meta.is_ok());
            let meta = meta.unwrap();
            let struct_attribute = KeysWithValues::from(meta);
            assert_eq!(format!("{}", struct_attribute), "key: 0x01\n vals: [ 2, \"3\", 0x04, ]");
        }
    }
}