//! Logging-macro token selection shared across the code-generation passes
//!
//! Both the decode debug-trace sites and the encode back-patch overflow guard splice a logging
//! macro into generated code. The exact macro depends on the `tracing` feature, so it is chosen
//! here once and reused, rather than duplicated per pass
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Selects the debug-level logging macro spliced into generated decode output
///
/// Returns `::tracing::debug!` when the `tracing` feature is enabled, or `::std::println!`
/// otherwise. Used at the decode key/value trace sites, which are emitted only when the container
/// `debug` flag is set
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing either `::tracing::debug!` or `::std::println!`
pub(crate) fn debug_logger() -> proc_macro2::TokenStream {
    #[cfg(feature = "tracing")]
    {
        quote! { ::tracing::debug! }
    }
    #[cfg(not(feature = "tracing"))]
    {
        quote! { ::std::println! }
    }
}

/// Selects the error-level logging macro spliced into generated code
///
/// Returns `::tracing::error!` when the `tracing` feature is enabled, or `::std::eprintln!`
/// otherwise. Used by the encode back-patch overflow guard, which fires on a cold error path
/// regardless of the container `debug` flag so an overflow is never silently swallowed
///
/// # Returns
///
/// A [`proc_macro2::TokenStream`] containing either `::tracing::error!` or `::std::eprintln!`
pub(crate) fn error_logger() -> proc_macro2::TokenStream {
    #[cfg(feature = "tracing")]
    {
        quote! { ::tracing::error! }
    }
    #[cfg(not(feature = "tracing"))]
    {
        quote! { ::std::eprintln! }
    }
}
