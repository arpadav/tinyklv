//! Parsing of the container-level fused `keylen(..)` sub-attribute
//!
//! Defines [`KeylenXcoder`], the [`Codec`] of the fused key+length frame header, whose encoder
//! takes no sigil. [`TryFrom<&syn::MetaList>`] drives the `parse_nested_meta` parse for
//! `enc = ..` and `dec = ..`; the codec decides each frame's framing at run time, so `keylen(..)`
//! takes no `size(..)`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::codec::Codec;
use crate::ast::{
    symbol,
    types::{HeaderEncoder, XcoderType},
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

/// The encoder and decoder of a fused key+length frame header
pub(crate) type KeylenXcoder = Codec<XcoderType>;

/// [`KeylenXcoder`] implementation of [`TryFrom`] for [`syn::MetaList`]
///
/// Parses the `enc = ..` and `dec = ..` sub-fields from a `keylen(..)` attribute list into a
/// [`KeylenXcoder`]. A `size(..)` is rejected naming where it belongs; it and any other unknown
/// field or duplicate are returned as [`syn::Error`] values, so the caller can surface them via
/// the [`crate::Ctxt`] accumulator immediately
impl TryFrom<&syn::MetaList> for KeylenXcoder {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaList) -> syn::Result<Self> {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut enc: Option<HeaderEncoder> = None;
        let mut dec: Option<XcoderType> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        input.parse_nested_meta(|meta| {
            tk_syn_macros::handle_unique_nested_meta_values! {
                meta;
                if meta.path == symbol::SIZE {
                    err!(SizeInKeylen)
                } else {
                    err!(UnknownKeylenField(meta.path))
                };
                2;
                enc: symbol::pnm_parse_maybestr_header_encoder => err!(DuplicateEncoderInHeader),
                dec: symbol::pnm_parse_maybestr_decoder => err!(DuplicateDecoderInHeader),
            }
        })?;
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(KeylenXcoder {
            enc: enc.map(|HeaderEncoder(enc)| enc),
            dec,
        })
    }
}
