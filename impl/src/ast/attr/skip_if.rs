//! Parsing of the shared `skip_if_default` / `skip_if = <fn>` sub-attribute
//!
//! Defines [`SkipIf`], when a field's frame is left out of the encode. Like `size(..)`, it is
//! written on a field or once per type in a container `default(typ = ..)`, so it lives one tier
//! up, directly under `attr`
//!
//! Author: aav

// --------------------------------------------------
// external
// --------------------------------------------------
use syn::spanned::Spanned;

#[derive(Debug, Clone)]
/// When a field's frame is left out of the encode
///
/// A field or a `default(..)` sets at most one form; a field's own replaces its default's whole
pub(crate) enum SkipIf {
    /// `skip_if_default`: the field equals its type's `Default::default()`
    Default(proc_macro2::Span),
    /// `skip_if = <fn>`: the user's `fn(&T) -> bool` returns `true` for the field
    Path(syn::Path),
}
/// [`SkipIf`] implementation
impl SkipIf {
    /// Resolves the skip from the `skip_if_default` and `skip_if = <fn>` items of one list
    ///
    /// # Arguments
    ///
    /// * `default` - The `skip_if_default` flag's span, if written
    /// * `path` - The `skip_if` fn, if written
    ///
    /// # Returns
    ///
    /// The skip, or `None` when neither is written
    ///
    /// # Errors
    ///
    /// Both written, spanned at the `skip_if` fn
    pub(crate) fn from_items(
        default: Option<proc_macro2::Span>,
        path: Option<syn::Path>,
    ) -> syn::Result<Option<Self>> {
        match (default, path) {
            (Some(_), Some(path)) => Err(syn::Error::new_spanned(path, err!(SkipIfConflict))),
            (Some(span), None) => Ok(Some(Self::Default(span))),
            (None, path) => Ok(path.map(Self::Path)),
        }
    }

    /// The span of the keyword or fn that set the skip, for diagnostics on it
    pub(crate) fn span(&self) -> proc_macro2::Span {
        match self {
            Self::Default(span) => *span,
            Self::Path(path) => path.span(),
        }
    }
}
