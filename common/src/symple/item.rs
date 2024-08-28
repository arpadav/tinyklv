//! [MetaItem] definitions, implementations, and utils
//! 
//! A [MetaItem] can be either a [MetaTuple] or a [MetaNameValue]
// --------------------------------------------------
// local
// --------------------------------------------------
use super::tuple::MetaTuple;
use super::value::MetaValue;
use super::nv::MetaNameValue;

#[derive(Clone)]
/// Enum to handle both [MetaNameValue] and [MetaTuple]
/// 
/// # Example
/// 
/// ```ignore
/// name = value
/// // OR
/// tname(name = value, name = value)
/// ```
pub enum MetaItem {
    Tuple(MetaTuple),
    Value(MetaValue),
    NameValue(MetaNameValue),
}
/// [MetaItem] implementation of [syn::parse::Parse]
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
/// [MetaItem] implementation of [std::fmt::Display]
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