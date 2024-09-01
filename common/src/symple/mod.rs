#![doc = include_str!("./README.md")]
pub mod nv;
pub mod tuple;
pub mod value;
pub mod contents;

pub use nv::*;
pub use tuple::*;
pub use value::*;
pub use contents::*;

pub mod prelude;
pub use prelude::*;

#[macro_export]
macro_rules! impl_hasvalue {
    ($struct:ident, $($constraint:tt)+) => {
        #[automatically_derived]
        #[doc = concat!("[`", stringify!($struct), "`] implementation")]
        impl<T> $struct<T>
        where
            T: $($constraint)+
        {
            #[doc = concat!("Create a new [`", stringify!($struct), "`]")]
            pub fn new(value: T) -> Self {
                $struct { value: Some(value) }
            }   
        }
        #[automatically_derived]
        #[doc = concat!("[`", stringify!($struct), "`] implementation of [`symple::HasValue`]")]
        impl<T> crate::symple::prelude::HasValue<T> for $struct<T>
        where
            T: $($constraint)+
        {
            /// Get the value, once set
            fn get(&self) -> Option<&T> {
                self.value.as_ref()
            }
            /// Get the value, as mut
            fn get_mut(&mut self) -> Option<&mut T> {
                self.value.as_mut()
            }
            /// Set the value
            fn set(&mut self, value: T) {
                self.value = Some(value);
            }
        }
        #[automatically_derived]
        #[doc = concat!("[`", stringify!($struct), "`] implementation of [`std::fmt::Display`]")]
        impl<T> std::fmt::Display for $struct<T>
        where
            T: std::fmt::Display,
            T: $($constraint)+
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.value.as_ref().map_or("None".to_string(), |v| format!("{}", v)))
            }   
        }
    };
}

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
