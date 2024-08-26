//! [Tuple] and [MetaTuple] definitions, implementations, and utils
//! 
//! A [MetaTuple] contains a key of type [syn::Ident] and a list of contents of type [MetaContents]
//! 
//! The [Tuple] struct is used to say "parse this token-stream as a [MetaTuple]"
// --------------------------------------------------
// local
// --------------------------------------------------
use super::contents::{
    MetaContents,
    MetaContentsIterator,
};
use super::item::MetaItem;

#[derive(Eq, Hash, Clone, PartialEq)]
/// A [MetaTuple] wrapper, used as a utility for proc-macro parsing
/// 
/// # Example
/// 
/// ```ignore
/// // inside of proc-macro lib
/// struct Input {
///     struct_attributes: symple::Tuple<StructAttributes>
/// }
/// struct StructAttributes {
///     value1: u32,
///     value2: u32,
///     value3: u32
/// }
/// impl From<symple::MetaContents> for StructAttributes {
///     fn from(meta: symple::MetaContents) -> Self {
///         todo!()
///     }
/// }
/// ```
/// 
/// ***Note that trait bounds for [From<MetaContents>] are required
/// for this to work.***
/// 
/// This expects ***any*** attribute on the proc-macro derived
/// struct, for example:
/// 
/// ```ignore
/// // outside of proc-macro lib
/// #[derive(MyProcMacro)]
/// #[my_proc_macro(value1 = 1, value2 = 2, inside_tuple(value3 = 3))]
/// struct SomeStruct;
/// ```
/// 
/// Which can then be parsed using the [From<MetaContents>] implementation
/// into the following, to help with proc-macro parsing:
/// 
/// ```ignore
/// Input {
///     // note that this is called using `struct_attributes.value`
///     struct_attributes: Some(StructAttributes {
///         value1: 1,
///         value2: 2
///         value3: 3
///     })
/// }
/// ```
pub struct Tuple<T: From<MetaContents> + std::fmt::Display> {
    pub value: Option<T>,
}
/// [Tuple] implementation of [From] for [MetaTuple]
impl <T: From<MetaContents> + std::fmt::Display> From<&MetaTuple> for Tuple<T> {
    fn from(meta: &MetaTuple) -> Self {
        Tuple { value: Some(meta.contents.clone().into()), }
    }
}
/// [Tuple] implementation of [From] for T
impl <T: From<MetaContents> + std::fmt::Display> From<T> for Tuple<T> {
    fn from(value: T) -> Self {
        Tuple { value: Some(value) }
    }
}
/// [Tuple] implementation of [Default]
impl <T: From<MetaContents> + std::fmt::Display> Default for Tuple<T> {
    fn default() -> Self {
        Tuple { value: None }
    }
}
/// [Tuple] implementation of [std::fmt::Display]
impl <T: From<MetaContents> + std::fmt::Display> std::fmt::Display for Tuple<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(x) => write!(f, "{}", x),
            None => write!(f, "None"),
        }
    }
}
crate::debug_from_display!(Tuple, From<MetaContents> + std::fmt::Display);

#[derive(Clone)]
/// [MetaTuple]
/// 
/// Innter data structure which is consists of a name [syn::Ident]
/// and listed value(s) [MetaContents]
/// 
/// # Example
/// 
/// ```ignore
/// name(a = 1, b(x = 2), c = 3)
/// ```
pub struct MetaTuple {
    pub name: syn::Ident,
    _paren: syn::token::Paren,
    pub contents: MetaContents,
}
/// [MetaTuple] implementation of [syn::parse::Parse]
impl syn::parse::Parse for MetaTuple {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(MetaTuple {
            name: input.parse()?,
            _paren: syn::parenthesized!(content in input),
            contents: content.parse()?,
        })
    }
}
/// [MetaTuple] implementation of [From<String>]
impl From<String> for MetaTuple {
    fn from(s: String) -> Self {
        // Convert the string into a TokenStream
        let inner_tokens: proc_macro2::TokenStream = s.parse().expect("Failed to parse string into TokenStream");
        // Attempt to parse the TokenStream as a MetaTuple
        match syn::parse2::<MetaTuple>(inner_tokens) {
            Ok(metatuple) => metatuple,
            Err(err) => panic!("Failed to parse MetaTuple from String: '{}'\n{}", s, err),
        }
    }
}
/// [MetaTuple] implementation of [IntoIterator]
impl<'a> IntoIterator for &'a MetaTuple {
    type Item = &'a MetaItem;
    type IntoIter = MetaContentsIterator<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.contents.into_iter()
    }
}
/// [MetaTuple] implementation of [std::fmt::Display]
impl std::fmt::Display for MetaTuple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}({})", self.name, self.contents)
    }
}
crate::debug_from_display!(MetaTuple);

#[cfg(test)]
mod tests {
    use super::*;

    mod readme_tuple_example {
        use quote::ToTokens;

        use super::*;
        use crate::symple as symple;

        #[derive(Default)]
        // struct attributes
        struct StructAttributes {
            values: Tuple<Values>,
        }
        struct Values {
            value1: syn::LitInt,
            value2: syn::LitInt,
        }

        // field attributes
        #[derive(Default)]
        struct FieldAttribute {
            attr: symple::NameValue<syn::Lit>
        }

        // required for all items inside `symple::Tuple`
        impl From<symple::MetaContents> for StructAttributes {
            fn from(x: symple::MetaContents) -> Self {
                let mut output = StructAttributes::default();
                let mut value1 = None;
                let mut value2 = None;
                for item in x.into_iter() {
                    match item {
                        symple::MetaItem::NameValue(mnv) => {
                            match mnv.name.to_string().as_str() {
                                "value1" => value1 = Some(mnv.value.clone()),
                                "value2" => value2 = Some(mnv.value.clone()),
                                _ => panic!("expected NameValue"),
                            }
                        }
                        _ => (),
                    }
                }
                match (value1, value2) {
                    (Some(v1), Some(v2)) => {
                        let v1 = if let symple::MetaValue::Lit(syn::Lit::Int(lit_int)) = v1 { lit_int } else { panic!("expected a lit int") };
                        let v2 = if let symple::MetaValue::Lit(syn::Lit::Int(lit_int)) = v2 { lit_int } else { panic!("expected a lit int") };
                        output.values = Values {
                            value1: v1
                            value2: v2,
                        };
                    }
                    _ => panic!("expected two values"),
                }
                output
            }
        }

        // required for all items inside `symple::NameValue`
        impl From<symple::MetaValue> for FieldAttribute {
            fn from(x: symple::MetaValue) -> Self {
                let mut output = FieldAttribute::default();
                output.attr = x.into();
                output
            }
        }
    }
}