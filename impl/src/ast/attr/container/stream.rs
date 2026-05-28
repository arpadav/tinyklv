//! Parsing of the `stream = <type>` container-level attribute
//!
//! Defines [`Stream`], a thin wrapper around an optional [`TypeType`] that is
//! produced by parsing the `stream = <type>` name-value form in a `#[klv(..)]`
//! annotation. When absent the stream type defaults to `&[u8]`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;
use crate::ast::types::TypeType;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

/// Parsed value of the `stream = <type>` container attribute
///
/// Wraps an `Option<TypeType>`: `Some` when the attribute was present and
/// successfully parsed, `None` when the attribute was absent. A missing
/// `stream` attribute causes the decode codegen to default to `&[u8]`
pub(crate) struct Stream(pub Option<TypeType>);

/// [`Stream`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_token_stream())
    }
}

/// [`Stream`] implementation of [`TryFrom`] for [`syn::MetaNameValue`]
///
/// Delegates to [`symbol::nv_parse_maybestr_stream`] to handle the three cases:
/// the keyword is present and parses successfully, the keyword is present but
/// the value is malformed (returns `Err`), or the keyword is absent entirely
/// (returns `Ok(Stream(None))`)
impl TryFrom<&syn::MetaNameValue> for Stream {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaNameValue) -> syn::Result<Self> {
        match symbol::nv_parse_maybestr_stream(input) {
            // `stream` keyword is detected, and value is parsed correctly
            Some(Ok(x)) => Ok(Stream(Some(x))),
            // `stream` keyword is detected, but value is not parsed correctly
            Some(Err(err)) => Err(err),
            // `stream` keyword is not detected
            None => Ok(Stream(None)),
        }
    }
}
