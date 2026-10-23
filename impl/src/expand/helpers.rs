//! Span helpers shared by the encode and decode code-generation passes
//!
//! Provides the call-site span that points a diagnostic at a user's token
//!
//! Author: aav

/// A span with the derive's call-site hygiene, located at `node`
///
/// Tokens emitted with it report a type error at `node` (e.g. a user's decoder) while still
/// resolving the derive's own locals, such as `input`. `node.span()` itself would resolve those
/// locals where `node` was written, which fails when `node` came in through a `macro_rules!`
/// fragment
///
/// # Arguments
///
/// * `node` - The tokens a diagnostic should point at
///
/// # Returns
///
/// [`proc_macro2::Span::call_site`] located at `node`
pub(super) fn call_site_at(node: &impl syn::spanned::Spanned) -> proc_macro2::Span {
    proc_macro2::Span::call_site().located_at(node.span())
}
