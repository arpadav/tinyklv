#[doc(hidden)]
/// A quick way to add enum variants of [`crate::Error`] to the [`crate::Ctxt`]
/// error or [`syn::Error`] by transforming it into a [`str`].
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
