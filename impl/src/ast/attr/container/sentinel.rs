//! Parsing of the `sentinel = ..` container-level attribute
//!
//! Defines [`Sentinel`], a thin newtype wrapper around an optional
//! [`syn::Lit`] produced by parsing the `sentinel = <literal>` name-value
//! form in a `#[klv(..)]` annotation. When absent the container has no
//! recognition sentinel and [`tinyklv::traits::SeekSentinel`] is not derived
//!
//! Author: aav
use quote::ToTokens;

use crate::ast::symbol;

/// Parsed value of the `sentinel = <literal>` container attribute
///
/// Wraps an `Option<syn::Lit>`: `Some` when the attribute was present and
/// successfully parsed, `None` when the attribute was absent. The contained
/// literal is typically a byte string (e.g. `b"HEARTBEAT"`) but the parser
/// accepts any literal form and leaves semantic validation to the expand pass
pub(crate) struct Sentinel(pub Option<syn::Lit>);

/// [`Sentinel`] implementation of [`std::fmt::Debug`]
impl std::fmt::Debug for Sentinel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.to_token_stream())
    }
}

/// [`Sentinel`] implementation of [`TryFrom`] for [`syn::MetaNameValue`]
///
/// Delegates to [`symbol::parse_nv_sentinel`] to handle the three cases:
/// the keyword is present and parses successfully, the keyword is present but
/// the value is malformed (returns `Err`), or the keyword is absent entirely
/// (returns `Ok(Sentinel(None))`)
impl TryFrom<&syn::MetaNameValue> for Sentinel {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaNameValue) -> syn::Result<Self> {
        match symbol::parse_nv_sentinel(input) {
            // `sentinel` keyword is detected, and value is parsed correctly
            Some(Ok(x)) => Ok(Sentinel(Some(x))),
            // `sentinel` keyword is detected, but value is not parsed correctly
            Some(Err(err)) => Err(err),
            // `sentinel` keyword is not detected
            None => Ok(Sentinel(None)),
        }
    }
}
