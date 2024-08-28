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
pub struct Tuple<T: From<MetaContents>> {
    pub value: Option<T>,
}
/// [Tuple] implementation of [From] for [MetaTuple]
impl <T: From<MetaContents>> From<&MetaTuple> for Tuple<T> {
    fn from(meta: &MetaTuple) -> Self {
        Tuple { value: Some(meta.contents.clone().into()), }
    }
}
/// [Tuple] implementation of [From] for T
impl <T: From<MetaContents>> From<T> for Tuple<T> {
    fn from(value: T) -> Self {
        Tuple { value: Some(value) }
    }
}
/// [Tuple] implementation of [Default]
impl <T: From<MetaContents>> Default for Tuple<T> {
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
        #![allow(dead_code)]
        use super::*;
        use crate::symple as symple;

        /// Struct attributes
        /// 
        /// See example below, and example in README.md
        struct StructAttributes {
            values: Tuple<Values>,
        }
        #[derive(Default)]
        struct Values {
            value1: Option<syn::LitInt>,
            value2: Option<syn::LitInt>,
        }

        /// Field attributes
        /// 
        /// See example below, and example in README.md
        #[derive(Default)]
        struct FieldAttribute {
            attr: symple::NameValue<syn::Lit>
        }

        /// [Values] implementation of [From] for [symple::MetaValue]
        /// 
        /// This is required for all items inside [symple::Tuple]
        /// 
        /// This is an example parsing implementation using [symple] types
        /// 
        /// See example below, and example in README.md
        impl From<symple::MetaContents> for Values {
            fn from(x: symple::MetaContents) -> Self {
                let mut output = Values::default();
                let mut value1 = None;
                let mut value2 = None;
                for item in x.into_iter() {
                    match item {
                        symple::MetaItem::Tuple(tpl) => {
                            if tpl.name.to_string() != "my_proc_macro" { continue; }
                            for item in tpl.into_iter() {
                                if let symple::MetaItem::NameValue(mnv) = item {
                                    let value_as_str = mnv.value.to_string();
                                    match &mnv.value {
                                        symple::MetaValue::Lit(someting) => match someting {
                                            syn::Lit::Str(s) => println!("{}: {}", mnv.name, s.value()),
                                            syn::Lit::Int(i) => println!("{}: {}", mnv.name, i.base10_digits()),
                                            _ => println!("{}: {}", mnv.name, value_as_str),
                                        }
                                        _ => println!("{}: {}", mnv.name, value_as_str),
                                    }
                                    match mnv.name.to_string().as_str() {
                                        "value1" => value1 = if let symple::MetaValue::Lit(syn::Lit::Int(lit_int)) = &mnv.value { Some(lit_int) } else { None },
                                        "value2" => value2 = if let symple::MetaValue::Lit(syn::Lit::Int(lit_int)) = &mnv.value { Some(lit_int) } else { None },
                                        _ => (),
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
                output.value1 = value1.cloned();
                output.value2 = value2.cloned();
                output
            }
        }

        /// [FieldAttribute] implementation of [From] for [symple::MetaValue]
        /// 
        /// This is required for all items inside [symple::NameValue]
        /// 
        /// This is an example parsing implementation using [symple] types
        /// 
        /// See example below, and example in README.md
        impl From<symple::MetaValue> for FieldAttribute {
            fn from(x: symple::MetaValue) -> Self {
                let mut output = FieldAttribute::default();
                output.attr = x.into();
                output
            }
        }

        #[test]
        /// Parses the struct attributes of the following:
        /// 
        /// ```ignore
        /// #[derive(MyProcMacro)]
        /// #[my_proc_macro(value1 = 1, value2 = 2)]
        /// // ^^ This is a `symple::Tuple` ^^
        /// // key: my_proc_macro
        /// // contents: { nv: { name = value1, value = 1 }, nv: { name = value2, value = 2 } }
        /// struct SomeStruct {
        ///     #[my_proc_macro(attr = "foo")]
        ///     // ^^ This is a `symple::Tuple` ^^
        ///     // key: my_proc_macro
        ///     // contents: { nv: { name = attr, value = "foo" } }
        ///     name: String,
        ///     #[my_proc_macro(attr = "bar")]
        ///     // ^^ This is a `symple::Tuple` ^^
        ///     // key: my_proc_macro
        ///     // contents: { nv: { name = attr, value = "bar" } }
        ///     age: u32,
        /// }
        /// ```
        fn parse_struct_attributes() {
            let input = quote::quote! {
                my_proc_macro(value1 = 1, value2 = 2)
            };
            let meta = syn::parse2::<MetaContents>(input);
            assert!(meta.is_ok());
            let meta = meta.unwrap();
            let values = Values::from(meta);
            assert!(values.value1.is_some());
            assert!(values.value2.is_some());
            assert_eq!(values.value1.unwrap().base10_digits(), "1");
            assert_eq!(values.value2.unwrap().base10_digits(), "2");
        }

        #[test]
        /// Parses the field attributes of the following:
        /// 
        /// ```ignore
        /// #[derive(MyProcMacro)]
        /// #[my_proc_macro(value1 = 1, value2 = 2)]
        /// // ^^ This is a `symple::Tuple` ^^
        /// // key: my_proc_macro
        /// // contents: { nv: { name = value1, value = 1 }, nv: { name = value2, value = 2 } }
        /// struct SomeStruct {
        ///     #[my_proc_macro(attr = "foo")]
        ///     // ^^ This is a `symple::Tuple` ^^
        ///     // key: my_proc_macro
        ///     // contents: { nv: { name = attr, value = "foo" } }
        ///     name: String,
        ///     #[my_proc_macro(attr = "bar")]
        ///     // ^^ This is a `symple::Tuple` ^^
        ///     // key: my_proc_macro
        ///     // contents: { nv: { name = attr, value = "bar" } }
        ///     age: u32,
        /// }
        /// ```
        fn parse_field_attributes() {
            let input = quote::quote! {
                my_proc_macro(attr = "foo")
            };
            let meta = syn::parse2::<MetaTuple>(input);
            assert!(meta.is_ok());
            let meta = meta.unwrap();
            for item in meta.contents.into_iter() {
                if let MetaItem::NameValue(mnv) = item {
                    assert_eq!(mnv.name.to_string(), "attr");
                    assert_eq!(mnv.value.to_string(), "\"foo\"");
                    let field_attribute = FieldAttribute::from(mnv.value.clone());
                    assert!(true);
                    assert_eq!(field_attribute.attr.to_string(), "\"foo\"");
                    break;
                }
            }
        }
    }
}