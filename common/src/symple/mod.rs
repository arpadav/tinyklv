//! # `symple`: Simple syn
//! 
//! Right now, this is being exclusively used under the [tinyklv](https://crates.io/crates/tinyklv) crate
//! for parsing of it's proc-macro's. If enough development goes into this, might publish it as stand-alone crate.
//! 
//! This is essentially just a [syn] wrapper, which automatically parses attributes into the following formats:
//! 
//! * [crate::Tuple] - `name(a = 1, b(x = 2), c = 3)`
//! * [crate::NameValue] - `name = value`
//! 
//! Therefore, to parse a tuple, you can do:
//! 
//! ```ignore
//! // outside proc-macro lib
//! #[derive(MyProcMacro)]
//! #[my_proc_macro(value1 = 1, value2 = 2)]
//! struct SomeStruct {
//!     #[my_proc_macro(attr = "foo")]
//!     name: String,
//!     #[my_proc_macro(attr = "bar")]
//!     age: u32,
//! }
//! ```
//! 
//! ```
//! // inside proc-macro lib
//! struct Input {
//!     struct_attributes: symple::Tuple<StructAttributes>
//!     field_attributes: symple::Tuple<FieldAttributes>
//! }
//! struct StructAttributes {
//!     value1: u32,
//!     value2: u32,
//! }
//! struct FieldAttributes {
//!     attr: symple::NameValue<syn::Lit>
//! }
//! 
//! // required for all items inside `symple::Tuple`
//! impl From<symple::MetaContents> for StructAttributes {
//!     fn from(x: symple::MetaContents) -> Self {
//!         todo!()
//!     }
//! }
//! 
//! // required for all items inside `symple::Tuple`
//! impl From<symple::MetaItem> for FieldAttributes {
//!     fn from(x: symple::MetaContents) -> Self {
//!         todo!()
//!     }
//! }
//! 
//! // required for all items inside `symple::NameValue`
//! impl From<symple::MetaValue> for FieldAttributes {
//!     fn from(x: symple::MetaValue) -> Self {
//!         self.attr = x.into()
//!     }
//! }
//! ```
//! 
//! More details are under the [crate::Tuple] and [crate::NameValue]
//! 
//! The following types have yet to be implemented:
//! 
//! * [crate::Value] - `value`
//! * [crate::Contents] - `a = 1, b(x = 2), c = 3`
//! 
//! Will likely release this crate once all of the above are implemented.
//! 
//! ## Example implementations:
//! 
//! * TODO: add tinyklv implementations here
#![allow(dead_code)]
pub mod nv;
pub mod item;
pub mod tuple;
pub mod value;
pub mod contents;

pub use nv::*;
pub use item::*;
pub use tuple::*;
pub use value::*;
pub use contents::*;

#[macro_export]
macro_rules! debug_from_display {
    ($t:ident, $($constraint:tt)*) => {
        impl<T> std::fmt::Debug for $t<T>
        where
            T: $($constraint)*
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self)
            }
        }
    };
    ($t:ty) => {
        impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self)
            }
        }
    };
}
