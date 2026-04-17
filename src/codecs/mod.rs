//! # Codec Naming Conventions
//!
//! - **`_as_usize`** (decode): follows Rust's `as` cast convention, e.g. `be_u16_as_usize`
//! - **`_from_usize`** (encode): follows Rust's `From` conversion convention, e.g. `u16_from_usize`
//!
//! Both are intentional and consistent within their respective sides.
pub mod ber;
pub mod binary;
pub mod string;

/// Re-exports path from `codecs::name::dec/enc` -> `codecs::dec/enc::name`
///
/// For example: [`crate::codecs::binary::dec`] -> [`crate::codecs::dec::binary`]
macro_rules! re_export {
    ($($module:ident),*) => {
        pub mod dec {
            $(pub use super::$module::dec as $module;)*
        }
        pub mod enc {
            $(pub use super::$module::enc as $module;)*
        }
    };
}
re_export! {
    ber,
    string,
    binary
}
