//! Parsing of the shared `size(..)` sub-attribute
//!
//! Defines [`SizeSpec`] and its byte-count axis [`SizeBytes`] - the declared size shape parsed from
//! a `size(var, exact = N | hint = N)` list. Unlike every other attr parser this one is shared by
//! BOTH field-level (`attr/field`) and key/len (`attr/container/{keylen,default}`) scopes, so it
//! lives one tier up, directly under `attr`. [`TryFrom<&syn::MetaList>`] drives the
//! `parse_nested_meta` loop via [`handle_unique_nested_meta_values!`], matching the
//! [`FieldXcoder`](super::field) / [`keylen::Xcoder`](super::container::keylen) /
//! [`DefaultXcoder`](super::container::default) convention
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use tk_syn_macros::handle_unique_nested_meta_values;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// The byte-count axis of a [`SizeSpec`], from the `exact = N` / `hint = N` items of `size(..)`
///
/// * [`SizeBytes::Unset`] - no byte count given (`size()` / `size(var)`)
/// * [`SizeBytes::Exact`] - `exact = N`: the value encodes to *exactly* `N` bytes, so the encoder
///   takes the fixed-width fast path (and on `len` it is the fixed prefix width)
/// * [`SizeBytes::Hint`] - `hint = N`: a *soft* capacity estimate; only sizes the encode reserve,
///   never the wire (an over-estimate is harmless)
///
/// `Exact` and `Hint` are mutually exclusive - a size is one or the other, never both
pub(crate) enum SizeBytes {
    /// No byte count was declared
    #[default]
    Unset,
    /// `exact = N`: exactly `N` bytes (fixed-width fast path / fixed `len` prefix)
    Exact(usize),
    /// `hint = N`: a soft `N`-byte encode-capacity estimate
    Hint(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
/// The declared size shape of a `key`/`len` prefix or a field value, parsed from `size(..)`
///
/// Two orthogonal axes:
///
/// * `var` - variable-length. On a field the decoder takes the runtime `len`; on `key`/`len` the
///   prefix is variable-width (e.g. BER). Absent ⇒ fixed: no `len` arg / fixed-width prefix
/// * `bytes` ([`SizeBytes`]) - the value's byte count: `exact = N` (wire-exact, fast path) or
///   `hint = N` (a soft encode-capacity estimate), or unset
///
/// Forms: `size(var)`, `size(exact = N)`, `size(hint = N)`, `size(var, exact = N)`,
/// `size(var, hint = N)`. `size(exact = N, hint = N)` is rejected at parse time
pub(crate) struct SizeSpec {
    /// `var` was present: variable-length prefix / length-parameterised field decoder.
    /// Private so the `var`-vs-`bytes` disambiguation stays inside the accessors below
    var: bool,
    /// the declared byte count, if any (see [`SizeBytes`])
    bytes: SizeBytes,
}
/// [`SizeSpec`] implementation
impl SizeSpec {
    /// Whether the field decoder takes the runtime `len` (the variable-length decode arity)
    pub(crate) fn takes_len(self) -> bool {
        self.var
    }

    /// The exact wire width this declares, if any (`exact = N`)
    ///
    /// Drives the fixed-width encode fast path for a value, and the fixed-width back-patch slot on
    /// `len`. A `hint`/unset size has no exact width
    pub(crate) fn exact_width(self) -> Option<usize> {
        match self.bytes {
            SizeBytes::Exact(n) => Some(n),
            SizeBytes::Unset | SizeBytes::Hint(_) => None,
        }
    }

    /// The encode capacity estimate in bytes, if any: both `exact = N` and `hint = N` contribute
    /// (an exact width is also the tightest capacity)
    pub(crate) fn capacity_bytes(self) -> Option<usize> {
        match self.bytes {
            SizeBytes::Exact(n) | SizeBytes::Hint(n) => Some(n),
            SizeBytes::Unset => None,
        }
    }
}
/// [`SizeSpec`] implementation of [`TryFrom`] for [`syn::meta::ParseNestedMeta`]
///
/// Parses the `size(var, exact = N | hint = N)` group via [`handle_unique_nested_meta_values!`],
/// recursing into the parenthesised list with [`ParseNestedMeta::parse_nested_meta`] - the same
/// idiom every other list attribute uses, but one tier deeper since `size(..)` is itself nested
/// inside a `#[klv(..)]` entry rather than the top-level list:
/// `var` is a bare-path flag, `exact`/`hint` are `= N`. The macro rejects a duplicate of any one
/// keyword; the mutually-exclusive `exact`⊕`hint` is rejected here as a post-validation step,
/// mirroring how [`DefaultXcoder`](super::container::default) validates its combination after the loop
///
/// [`ParseNestedMeta::parse_nested_meta`]: syn::meta::ParseNestedMeta::parse_nested_meta
impl TryFrom<&syn::meta::ParseNestedMeta<'_>> for SizeSpec {
    type Error = syn::Error;
    fn try_from(input: &syn::meta::ParseNestedMeta) -> syn::Result<Self> {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut var: Option<()> = None;
        let mut exact: Option<syn::LitInt> = None;
        let mut hint: Option<syn::LitInt> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        input.parse_nested_meta(|meta| {
            handle_unique_nested_meta_values! {
                meta;
                err!(UnknownSizeField(meta.path));
                3;
                var: symbol::parse_pnm_var,
                exact: symbol::parse_pnm_exact,
                hint: symbol::parse_pnm_hint,
            }
        })?;
        // --------------------------------------------------
        // resolve the byte count - `exact` and `hint` are mutually exclusive
        // --------------------------------------------------
        let bytes = match (exact, hint) {
            (Some(_), Some(h)) => {
                return Err(syn::Error::new_spanned(h, err!(SizeExactHintConflict)));
            }
            (Some(e), None) => SizeBytes::Exact(e.base10_parse()?),
            (None, Some(h)) => SizeBytes::Hint(h.base10_parse()?),
            (None, None) => SizeBytes::Unset,
        };
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(SizeSpec {
            var: var.is_some(),
            bytes,
        })
    }
}
