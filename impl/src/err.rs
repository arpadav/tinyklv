//! Internal `err!` macro for building [`crate::Error`] diagnostic messages
//!
//! Provides the [`err!`] macro, which constructs a [`std::borrow::Cow<str>`]
//! from a [`crate::Error`] variant. The result is passed directly to
//! [`crate::Ctxt::error_spanned_by`] or [`crate::Ctxt::syn_error`] so that
//! error messages are built in one consistent place rather than scattered
//! through the attribute-parsing code
//!
//! Author: aav

/// Constructs a [`std::borrow::Cow<str>`] from a [`crate::Error`] variant
///
/// Accepts four calling conventions depending on the variant's argument shape:
///
/// * `err!(Variant)` - zero-argument variant
/// * `err!(Variant(expr, ..))` - one or more expressions (converted via
///   [`quote::ToTokens`] then [`ToString`])
/// * `err!(Variant("literal", ..))` - one or more string literals (converted
///   via [`ToString`])
/// * `err!(Variant(expr, .. ; "literal", ..))` - mixed expressions and literals
/// * `err!(Variant(@String expr))` - a pre-built [`String`], passed as-is
///   without token-stream conversion
///
/// The macro calls `crate::Error::$variant(..).as_str()` and returns the
/// resulting [`std::borrow::Cow<str>`]
#[doc(hidden)]
macro_rules! err {
    // --------------------------------------------------
    // 1+ expr; 1+ literals
    // --------------------------------------------------
    ($variant:ident($($expr:expr),* ; $($litstr:literal),*)) => {
        $crate::Error::$variant(
            $($expr.to_token_stream().to_string()),*,
            $($litstr.to_string()),*
        ).as_str()
    };

    // --------------------------------------------------
    // 1+ literals
    // --------------------------------------------------
    ($variant:ident($($litstr:literal),*)) => {
        $crate::Error::$variant(
            $($litstr.to_string()),*
        ).as_str()
    };

    // --------------------------------------------------
    // 1+ expressions
    // --------------------------------------------------
    ($variant:ident($($expr:expr),*)) => {
        $crate::Error::$variant(
            $($expr.to_token_stream().to_string()),*
        ).as_str()
    };

    // --------------------------------------------------
    // @String operator
    // --------------------------------------------------
    ($variant:ident(@String $expr:expr)) => {
        $crate::Error::$variant(
            $expr
        ).as_str()
    };

    // --------------------------------------------------
    // no arguments
    // --------------------------------------------------
    ($variant:ident) => {
        $crate::Error::$variant.as_str()
    };
}
