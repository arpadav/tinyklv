//! Parsing of the `default(..)` container-level attribute
//!
//! Defines [`DefaultXcoder`], which represents one entry in a `#[klv(default(..))]`
//! annotation on a struct. Each entry supplies a type together with an optional
//! encoder, decoder and size, each used for any field of that type that leaves it unset; the
//! size's `lengthed` goes with the decoder
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::size::SizeSpec;
use crate::ast::symbol;
use crate::ast::types::{SiguledXcoder, TypeType, XcoderType};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use tk_syn_macros::handle_unique_nested_meta_values;

#[derive(Debug)]
/// A default encoder / decoder for a specific type
///
/// # Syntax
///
/// `default(typ = <type>, enc = <path>, dec = <path>, size(lengthed, exact = N | reserve = N))`
///
/// Both `enc` and `dec` are optional, however at least one must be provided
///
/// `size` is optional; a matched field takes its `exact` / `reserve` where the field's own
/// `size(..)` has neither, and its `lengthed` when the field uses this `dec`. A `lengthed`
/// requires a `dec` in the same `default(..)`
pub(crate) struct DefaultXcoder {
    /// The type associated with the encoder / decoder
    pub typ: Option<TypeType>,

    /// The encoder
    pub enc: Option<SiguledXcoder>,

    /// The decoder
    pub dec: Option<XcoderType>,

    /// The value size shape propagated to matched fields; see [`SizeSpec`]
    pub size: Option<SizeSpec>,

    /// syn errors
    pub errors: Option<syn::Error>,
}
/// [`DefaultXcoder`] implementation of [`From`] for [`syn::MetaList`]
///
/// Parses the nested meta arguments from a `default(typ = .., enc = .., dec = .., size(..))`
/// attribute list into a [`DefaultXcoder`]
///
/// Validation errors (missing type, missing both enc and dec, duplicate fields) are
/// recorded in `DefaultXcoder::errors` rather than returned directly, so that the
/// caller can accumulate multiple diagnostics before aborting
impl From<&syn::MetaList> for DefaultXcoder {
    fn from(input: &syn::MetaList) -> Self {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut typ: Option<TypeType> = None;
        let mut enc: Option<SiguledXcoder> = None;
        let mut dec: Option<XcoderType> = None;
        let mut size: Option<SizeSpec> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        let mut errors = input.parse_nested_meta(|meta| {
            let maybe_typ = typ
                .clone()
                .map_or(String::from("not defined"), |typ| typ.to_token_stream().to_string());
            handle_unique_nested_meta_values! {
                meta;
                err!(UnknownDefaultField(meta.path));
                4;
                enc: symbol::pnm_parse_maybestr_encoder => err!(DuplicateEncoderInDefault(maybe_typ)),
                dec: symbol::pnm_parse_maybestr_decoder => err!(DuplicateDecoderInDefault(maybe_typ)),
                typ: symbol::pnm_parse_maybestr_type    => err!(DuplicateTypeInDefault),
                size: symbol::parse_pnm_size            => err!(DuplicateSizeInDefault),
            }
        }).err();
        // --------------------------------------------------
        // at least one encoder or one decoder must be provided
        // --------------------------------------------------
        if enc.is_none() && dec.is_none() {
            let e = syn::Error::new_spanned(input.clone(), err!(MissingEncDecInDefault));
            match errors {
                None => errors = Some(e),
                Some(ref mut errors) => errors.combine(e),
            }
        }
        // --------------------------------------------------
        // `lengthed` describes the decoder, so it needs this default's own `dec`
        // --------------------------------------------------
        if dec.is_none()
            && let Some(lengthed) = size.as_ref().and_then(SizeSpec::lengthed)
        {
            let e = syn::Error::new(lengthed, err!(SizeLengthedWithoutDecInDefault));
            match errors {
                None => errors = Some(e),
                Some(ref mut errors) => errors.combine(e),
            }
        }
        // --------------------------------------------------
        // type must be provided
        // --------------------------------------------------
        if typ.is_none() {
            let e = syn::Error::new_spanned(input.clone(), err!(MissingTypeInDefault));
            match errors {
                None => errors = Some(e),
                Some(ref mut errors) => errors.combine(e),
            }
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        DefaultXcoder {
            typ,
            enc,
            dec,
            size,
            errors,
        }
    }
}
