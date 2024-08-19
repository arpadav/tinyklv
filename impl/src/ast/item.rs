// --------------------------------------------------
// local
// --------------------------------------------------
use super::tuple::MetaTuple;
use super::nv::MetaNameValue;

#[derive(Clone)]
/// Enum to handle both [`MetaNameValue`] and [`MetaTuple`]
/// 
/// # Example
/// 
/// ```ignore
/// name = value
/// OR
/// tname(name = value, name = value)
/// ```
pub(crate) enum MetaItem {
    Tuple(MetaTuple),
    NameValue(MetaNameValue),
}
/// [`MetaItem`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for MetaItem {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        // Attempt to parse as MetaTuple first
        if input.peek(syn::Ident)
        && input.peek2(syn::token::Paren)
        { Ok(MetaItem::Tuple(input.parse()?)) }
        // Otherwise, parse as MetaNameValue
        else { Ok(MetaItem::NameValue(input.parse()?)) }
    }
}
/// [`MetaItem`] implementation of [`std::fmt::Display`]
impl std::fmt::Display for MetaItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MetaItem::Tuple(x) => write!(f, "{}", x),
            MetaItem::NameValue(x) => write!(f, "{}", x),
        }
    }
}
crate::debug_from_display!(MetaItem);