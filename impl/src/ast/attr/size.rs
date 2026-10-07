//! Parsing of the shared `size(..)` sub-attribute
//!
//! Defines [`SizeSpec`], the declared size shape of a field value parsed from a
//! `size(lengthed, exact = N | reserve = N)` list, and [`SizeBytes`], its byte count, which is also
//! the whole `size(exact = N | reserve = N)` of a `key(..)` or `len(..)`. Unlike every other attr
//! parser this one is shared by BOTH field-level (`attr/field`, `attr/container/default`) and frame
//! header (`attr/container/header`) scopes, so it lives one tier up, directly under `attr`. Each
//! type's [`TryFrom<&syn::meta::ParseNestedMeta>`] drives the `parse_nested_meta` loop via
//! [`handle_unique_nested_meta_values!`], matching the [`FieldXcoder`](super::field) /
//! `container::header::HeaderXcoder` / [`DefaultXcoder`](super::container::DefaultXcoder)
//! convention
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;
use tk_syn_macros::handle_unique_nested_meta_values;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use syn::spanned::Spanned;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
/// The byte count of a `size(..)`, from its `exact = N` / `reserve = N` items: the whole size of
/// a `key(..)` or `len(..)`, and one axis of a field's [`SizeSpec`]
///
/// * [`SizeBytes::Unset`] - no byte count given (`size()` / `size(lengthed)`)
/// * [`SizeBytes::Exact`] - `exact = N`: a field value is *exactly* `N` bytes, so decode rejects a
///   frame length other than `N` and the encoder takes the fixed-width fast path (on `len` it is
///   the fixed prefix width)
/// * [`SizeBytes::Reserve`] - `reserve = N`: an encode reserve only, never the wire (an
///   over-estimate is harmless)
///
/// `Exact` and `Reserve` are mutually exclusive - a size is one or the other, never both
pub(crate) enum SizeBytes {
    /// No byte count was declared
    #[default]
    Unset,
    /// `exact = N`: exactly `N` bytes (a field's checked length and fast path / fixed `len` prefix)
    Exact {
        /// `N`
        width: usize,
        /// `N` as written, e.g. `0x10`
        lit: syn::LitInt,
    },
    /// `reserve = N`: an `N`-byte encode reserve
    Reserve(usize),
}
/// [`SizeBytes`] implementation
impl SizeBytes {
    /// Resolves the byte count from the `exact = N` and `reserve = N` items of one `size(..)` list
    ///
    /// The mutually-exclusive `exact`⊕`reserve` is rejected here as a post-validation step,
    /// mirroring how [`DefaultXcoder`](super::container::DefaultXcoder) validates its combination
    /// after the loop
    ///
    /// # Arguments
    ///
    /// * `exact` - The `exact = N` literal, if written
    /// * `reserve` - The `reserve = N` literal, if written
    ///
    /// # Errors
    ///
    /// Both items written (spanned at `reserve`), or a literal that does not parse as a `usize`
    fn from_items(exact: Option<syn::LitInt>, reserve: Option<syn::LitInt>) -> syn::Result<Self> {
        match (exact, reserve) {
            (Some(_), Some(r)) => Err(syn::Error::new_spanned(r, err!(SizeExactReserveConflict))),
            (Some(lit), None) => Ok(Self::Exact {
                width: lit.base10_parse()?,
                lit,
            }),
            (None, Some(r)) => r.base10_parse().map(Self::Reserve),
            (None, None) => Ok(Self::Unset),
        }
    }

    /// The exact wire width this declares and the literal it is written as, if any (`exact = N`)
    fn exact(&self) -> Option<(usize, &syn::LitInt)> {
        match self {
            Self::Exact { width, lit } => Some((*width, lit)),
            Self::Unset | Self::Reserve(_) => None,
        }
    }

    /// The exact wire width this declares, if any (`exact = N`)
    ///
    /// Drives a field value's decode length check and fixed-width encode fast path, and the
    /// fixed-width back-patch slot on `len`. A `reserve`/unset size has no exact width
    pub(crate) fn exact_width(&self) -> Option<usize> {
        self.exact().map(|(width, _)| width)
    }

    /// The encode capacity estimate in bytes, if any: both `exact = N` and `reserve = N` contribute
    /// (an exact width is also the tightest capacity)
    pub(crate) fn capacity_bytes(&self) -> Option<usize> {
        match self {
            Self::Exact { width: n, .. } | Self::Reserve(n) => Some(*n),
            Self::Unset => None,
        }
    }
}
/// [`SizeBytes`] implementation of [`TryFrom`] for [`syn::meta::ParseNestedMeta`]
///
/// Parses a `key(..)` or `len(..)` list's `size(exact = N | reserve = N)` group the way
/// [`SizeSpec`] parses a field's, one tier deeper than the list it sits in. A key or length prefix
/// has no `lengthed`, so the macro reports it as unknown, naming where it belongs
impl TryFrom<&syn::meta::ParseNestedMeta<'_>> for SizeBytes {
    type Error = syn::Error;
    fn try_from(input: &syn::meta::ParseNestedMeta) -> syn::Result<Self> {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut exact: Option<syn::LitInt> = None;
        let mut reserve: Option<syn::LitInt> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        input.parse_nested_meta(|meta| {
            handle_unique_nested_meta_values! {
                meta;
                err!(UnknownSizeFieldInHeader(meta.path));
                2;
                exact: symbol::parse_pnm_exact,
                reserve: symbol::parse_pnm_reserve,
            }
        })?;
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Self::from_items(exact, reserve)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Where the length a `lengthed` decoder is passed comes from ([`SizeSpec::len_source`])
pub(crate) enum LenSource {
    /// The frame's own length
    Frame,
    /// `N` from `exact = N`, the width decode checks the frame's length against before taking the
    /// value
    Exact(usize),
}

#[derive(Debug, Clone)]
/// The declared size shape of a field value, parsed from `size(..)`
///
/// Two orthogonal axes:
///
/// * `lengthed` - the decoder is passed the frame's runtime `len`. Absent ⇒ no `len` arg
/// * `bytes` ([`SizeBytes`]) - the value's byte count: `exact = N` (decode rejects any other
///   length, encode takes the fast path) or `reserve = N` (an encode reserve), or unset
///
/// Forms: `size(lengthed)`, `size(exact = N)`, `size(reserve = N)`, `size(lengthed, exact = N)`,
/// `size(lengthed, reserve = N)`. `size(exact = N, reserve = N)` is rejected at parse time
pub(crate) struct SizeSpec {
    /// the `lengthed` keyword, if written: the field decoder is passed the frame's length
    lengthed: Option<proc_macro2::Span>,

    /// the declared byte count, if any (see [`SizeBytes`])
    bytes: SizeBytes,

    /// the path of the `size` that declared `bytes`, for diagnostics on it
    span: proc_macro2::Span,
}
/// [`SizeSpec`] implementation
impl SizeSpec {
    /// The size of a field whose type has a container `default(typ = ..)`
    ///
    /// `lengthed` describes the decoder's shape, so it comes from the side whose `dec` the field
    /// uses. The byte count is the field's own unless it left it unset
    ///
    /// # Arguments
    ///
    /// * `own` - The field's own `size(..)`
    /// * `default` - The default's `size(..)`
    /// * `default_dec` - Whether the field uses the default's `dec`
    ///
    /// # Returns
    ///
    /// The filled size, or `None` when neither side wrote one
    ///
    /// # Errors
    ///
    /// The field's own `lengthed` on the default's `dec` that is not `lengthed`, spanned at the
    /// field's `lengthed`
    pub(crate) fn filled(
        own: Option<Self>,
        default: Option<&Self>,
        default_dec: bool,
    ) -> syn::Result<Option<Self>> {
        let own_lengthed = own.as_ref().and_then(|own| own.lengthed);
        let lengthed = if default_dec {
            let default_lengthed = default.and_then(|default| default.lengthed);
            if let (Some(span), None) = (own_lengthed, default_lengthed) {
                return Err(syn::Error::new(span, err!(SizeLengthedOnDefaultDec)));
            }
            default_lengthed
        } else {
            own_lengthed
        };
        let bytes_side = match (own, default) {
            (Some(own), _) if own.bytes != SizeBytes::Unset => Some(own),
            (_, Some(default)) => Some(default.clone()),
            (own, None) => own,
        };
        Ok(bytes_side.map(|Self { bytes, span, .. }| Self {
            lengthed,
            bytes,
            span,
        }))
    }

    /// Whether the field decoder takes the runtime `len` (the length-parameterised decode arity)
    pub(crate) fn takes_len(&self) -> bool {
        self.lengthed.is_some()
    }

    /// The span of the `lengthed` keyword, if written
    pub(crate) fn lengthed(&self) -> Option<proc_macro2::Span> {
        self.lengthed
    }

    /// The exact wire width this declares, if any (`exact = N`); see [`SizeBytes::exact_width`]
    pub(crate) fn exact_width(&self) -> Option<usize> {
        self.bytes.exact_width()
    }

    /// The exact wire width this declares and the literal it is written as, if any (`exact = N`)
    pub(crate) fn exact(&self) -> Option<(usize, &syn::LitInt)> {
        self.bytes.exact()
    }

    /// The span of the `size(..)` that declared the byte count
    pub(crate) fn span(&self) -> proc_macro2::Span {
        self.span
    }

    /// The encode capacity estimate in bytes, if any; see [`SizeBytes::capacity_bytes`]
    pub(crate) fn capacity_bytes(&self) -> Option<usize> {
        self.bytes.capacity_bytes()
    }

    /// Where the length a `lengthed` decoder is passed comes from
    ///
    /// # Returns
    ///
    /// The source, or `None` when the decoder reads without a length
    pub(crate) fn len_source(&self) -> Option<LenSource> {
        let source = self
            .exact_width()
            .map_or(LenSource::Frame, LenSource::Exact);
        self.takes_len().then_some(source)
    }
}
/// [`SizeSpec`] implementation of [`TryFrom`] for [`syn::meta::ParseNestedMeta`]
///
/// Parses the `size(lengthed, exact = N | reserve = N)` group via
/// [`handle_unique_nested_meta_values!`], recursing into the parenthesised list with
/// [`ParseNestedMeta::parse_nested_meta`] - the same idiom every other list attribute uses, but
/// one tier deeper since `size(..)` is itself nested inside a `#[klv(..)]` entry rather than the
/// top-level list: `lengthed` is a bare-path flag, `exact`/`reserve` are `= N`. The macro rejects
/// a duplicate of any one keyword; [`SizeBytes`] resolves the byte count
///
/// [`ParseNestedMeta::parse_nested_meta`]: syn::meta::ParseNestedMeta::parse_nested_meta
impl TryFrom<&syn::meta::ParseNestedMeta<'_>> for SizeSpec {
    type Error = syn::Error;
    fn try_from(input: &syn::meta::ParseNestedMeta) -> syn::Result<Self> {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut lengthed: Option<proc_macro2::Span> = None;
        let mut exact: Option<syn::LitInt> = None;
        let mut reserve: Option<syn::LitInt> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        input.parse_nested_meta(|meta| {
            handle_unique_nested_meta_values! {
                meta;
                err!(UnknownSizeField(meta.path));
                3;
                lengthed: symbol::parse_pnm_lengthed,
                exact: symbol::parse_pnm_exact,
                reserve: symbol::parse_pnm_reserve,
            }
        })?;
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(SizeSpec {
            lengthed,
            bytes: SizeBytes::from_items(exact, reserve)?,
            span: input.path.span(),
        })
    }
}
