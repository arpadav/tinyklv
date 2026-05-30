//! # Codec Naming Conventions
//!
//! - **`_as_usize`** (decode): follows Rust's `as` cast convention, e.g. `be_u16_as_usize`
//! - **`_from_usize`** (encode): follows Rust's `From` conversion convention, e.g. `u16_from_usize`
//!
//! Both are intentional and consistent within their respective sides
//!
//! Author: aav
pub mod ber;
pub mod binary;
pub mod string;

/// Builds the flat `codecs::dec::<name>` and `codecs::enc::<name>` re-export modules
///
/// Takes a comma-separated list of codec module names (e.g. `ber`, `binary`, `string`) and
/// emits two public modules - `dec` and `enc` - each re-exporting the inner
/// `dec`/`enc` sub-module of every named codec under its original name
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
