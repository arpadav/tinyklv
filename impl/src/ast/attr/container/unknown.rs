//! Parsing of the container-level `unknown(..)` sub-attribute
//!
//! Defines [`UnknownXcoder`], the [`Codec`] of the frames whose key no field declares, whose
//! encoder takes a sigil like a field's. [`TryFrom<&syn::MetaList>`] drives the
//! `parse_nested_meta` parse for `enc = ..` and `dec = ..`; the decoder reads the rest of the
//! frame as the header codec left it, so `unknown(..)` takes no `size(..)`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::codec::Codec;
use crate::ast::{
    symbol,
    types::{SiguledXcoder, XcoderType},
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

/// The encoder and decoder of the frames whose key no field declares
pub(crate) type UnknownXcoder = Codec<SiguledXcoder>;

/// [`UnknownXcoder`] implementation of [`TryFrom`] for [`syn::MetaList`]
///
/// Parses the `enc = ..` and `dec = ..` sub-fields from an `unknown(..)` attribute list into an
/// [`UnknownXcoder`]. Any other field, a `size(..)` included, and a duplicate are returned as
/// [`syn::Error`] values naming `unknown`, so the caller can surface them via the [`crate::Ctxt`]
/// accumulator immediately
impl TryFrom<&syn::MetaList> for UnknownXcoder {
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
                err!(UnknownFieldInUnknown(meta.path));
                2;
                enc: symbol::pnm_parse_maybestr_encoder => err!(DuplicateEncoderInUnknown),
                dec: symbol::pnm_parse_maybestr_decoder => err!(DuplicateDecoderInUnknown),
            }
        })?;
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Ok(UnknownXcoder { enc, dec })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Error,
        ast::{attr::MainContainer, symbol},
    };

    use quote::quote;

    #[test]
    /// Verifies every wrong `unknown(..)` on a container is the one error reported, and names
    /// `unknown`
    ///
    /// Writes each wrong form beside an otherwise complete key + len framing: a duplicate `enc`
    /// and `dec`, both non-list forms and a list with no `dec`. A message borrowed from
    /// `keylen(..)` or a field would name the wrong attribute
    fn wrong_unknown_names_unknown() {
        // --------------------------------------------------
        // (name, container `unknown` attributes, the one error)
        // --------------------------------------------------
        let cases = [
            (
                "duplicate enc",
                quote! { unknown(dec = frame, enc = frame_enc, enc = frame_enc) },
                Error::DuplicateEncoderInUnknown,
            ),
            (
                "duplicate dec",
                quote! { unknown(dec = frame, dec = frame) },
                Error::DuplicateDecoderInUnknown,
            ),
            (
                "name-value unknown",
                quote! { unknown = frame },
                Error::ExpectedAsList(symbol::UNKNOWN_FRAMES.to_string()),
            ),
            (
                "bare unknown",
                quote! { unknown },
                Error::ExpectedAsList(symbol::UNKNOWN_FRAMES.to_string()),
            ),
            (
                "missing dec",
                quote! { unknown(enc = frame_enc) },
                Error::MissingDecInCodec(symbol::UNKNOWN_FRAMES.to_string()),
            ),
        ];
        // --------------------------------------------------
        // `unknown` opening a code span, as `unknown` or `unknown(..)`
        // --------------------------------------------------
        let named = format!("`{}", symbol::UNKNOWN_FRAMES);
        for (name, unknown, expected) in cases {
            let item: syn::DeriveInput = syn::parse_quote! {
                #[klv(
                    key(dec = decb::u8),
                    len(dec = decb::u8_as_usize),
                    #unknown,
                    allow_unimplemented_encode,
                )]
                struct Reading {
                    #[klv(key = 0x01, dec = decb::u8)]
                    id: u8,
                }
            };
            let Err(messages) = MainContainer::parsed(&item) else {
                panic!("{name}: reports an error");
            };
            assert_eq!(messages, [expected.to_string()], "{name}: the one error");
            assert!(
                messages.iter().all(|message| message.contains(&named)),
                "{name}: the error names `unknown`: {messages:?}"
            );
        }
    }

    #[test]
    /// Verifies an `unknown(..)` with no `enc` parses when no `#[klv(unknown)]` field keeps its
    /// frames, `allow_unimplemented_encode` unset
    ///
    /// The encoder only re-emits the frames that field keeps, so without the field it is never
    /// called
    fn enc_is_required_only_beside_the_field() {
        let item: syn::DeriveInput = syn::parse_quote! {
            #[klv(
                key(dec = decb::u8, enc = encb::u8),
                len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
                unknown(dec = frame),
            )]
            struct Reading {
                #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
                id: u8,
            }
        };
        assert!(
            MainContainer::parsed(&item).is_ok(),
            "the container parses, no error reported"
        );
    }
}
