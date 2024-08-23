// --------------------------------------------------
// local
// --------------------------------------------------
use crate::item::MetaItem;

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
/// [`MetaContents`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for MetaContents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let items: Vec<String> = self.items.iter().map(|item| format!("{}", item)).collect();
        write!(f, "{}", items.join(", "))
    }
}
crate::debug_from_display!(MetaContents);

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