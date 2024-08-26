#![doc = include_str!("./README.md")]
pub mod nv;
pub mod tuple;
pub mod value;
pub mod contents;

pub use nv::*;
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
