//! Parsing of the container-level `key(..)` and `len(..)` sub-attributes
//!
//! Defines [`Xcoder`], which holds the optional encoder and decoder for the
//! key and length fields of a KLV container. Both are optional because
//! `allow_unimplemented_encode` / `allow_unimplemented_decode` may suppress
//! the missing-xcoder error. [`TryFrom<&syn::MetaList>`] drives the
//! `parse_nested_meta` parse for `enc = ..` and `dec = ..`
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::symbol;
use crate::ast::types::{SiguledXcoder, XcoderType};

#[derive(Debug, Clone)]
/// The encoder and decoder for a KLV key or length field
///
/// Both fields are optional because `allow_unimplemented_encode` or
/// `allow_unimplemented_decode` may be set on the container, in which case
/// a missing encoder or decoder is not an error
pub struct Xcoder {
    /// The encoder, or `None` when unimplemented encoding is allowed
    pub enc: Option<SiguledXcoder>,
    /// The decoder, or `None` when unimplemented decoding is allowed
    pub dec: Option<XcoderType>,
}
/// [`Xcoder`] implementation of [`TryFrom`] for [`syn::MetaList`]
///
/// Parses the `enc = ..` and `dec = ..` sub-fields from a `key(..)` or
/// `len(..)` attribute list into an [`Xcoder`]. Unknown fields and duplicate
/// encoder/decoder entries are returned as [`syn::Error`] values rather than
/// stored internally, so the caller can surface them via the [`crate::Ctxt`]
/// accumulator immediately
impl TryFrom<&syn::MetaList> for Xcoder {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaList) -> syn::Result<Self> {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut enc: Option<SiguledXcoder> = None;
        let mut dec: Option<XcoderType> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        input.parse_nested_meta(|meta| {
            tk_syn_macros::handle_unique_nested_meta_values! {
                meta;
                err!(UnknownKeyLenField(meta.path));
                2;
                enc: symbol::pnm_parse_maybestr_encoder => err!(DuplicateEncoderInKeyLen),
                dec: symbol::pnm_parse_maybestr_decoder => err!(DuplicateDecoderInKeyLen),
            }
        })?;
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(Xcoder { enc, dec })
    }
}
