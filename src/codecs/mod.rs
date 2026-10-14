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

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{
    Parser,
    error::ContextError,
    stream::{Stream, StreamIsPartial},
    token::take,
};

/// [`take`]s `len` tokens, leaving the cursor at the end of the input when fewer remain
///
/// `take` leaves the cursor where it was on a short read, where a caller that tells a cut value
/// from a rejected one by the cursor reads it as rejected. The `len`-taking decoders read their
/// bytes through this, so a value the end of the input cuts reads as cut
///
/// # Arguments
///
/// * `len` - The number of tokens to take
///
/// # Returns
///
/// A parser yielding the `len` tokens, or `take`'s error with the cursor at the end of the input
fn take_or_finish<I>(len: usize) -> impl Parser<I, <I as Stream>::Slice, ContextError>
where
    I: Stream + StreamIsPartial,
{
    move |input: &mut I| {
        take(len).parse_next(input).inspect_err(|_| {
            let _ = input.finish();
        })
    }
}

/// Builds the flat `codecs::dec::<name>` and `codecs::enc::<name>` re-export modules
///
/// Takes a comma-separated list of codec module names (e.g. `ber`, `binary`, `string`) and
/// emits two public modules - `dec` and `enc` - each re-exporting the inner
/// `dec`/`enc` sub-module of every named codec under its original name
///
/// For example: [`crate::codecs::binary::dec`] -> [`crate::codecs::dec::binary`]
macro_rules! re_export {
    ($($module:ident),*) => {
        /// The decoders of every codec, a module per codec: `dec::binary` is
        /// [`crate::codecs::binary::dec`]
        pub mod dec {
            $(pub use super::$module::dec as $module;)*
        }
        /// The encoders of every codec, a module per codec: `enc::binary` is
        /// [`crate::codecs::binary::enc`]
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
