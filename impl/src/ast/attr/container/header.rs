//! Parsing of the container-level `key(..)` and `len(..)` sub-attributes, and the state of the
//! three frame header attributes, the fused `keylen(..)` included
//!
//! Defines [`HeaderXcoder`], which holds the optional encoder and decoder of a `key(..)` or
//! `len(..)`, and the byte count of its `size(..)`. Both xcoders are optional because
//! `allow_unimplemented_encode` / `allow_unimplemented_decode` may suppress
//! the missing-xcoder error. [`TryFrom<&syn::MetaList>`] drives the
//! `parse_nested_meta` parse for `enc = ..`, `dec = ..` and `size(..)`
//!
//! Defines [`HeaderAttrs`], the [`CodecAttr`] state of each of the three frame header attributes
//! on a container
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    AllowUnimplemented,
    codec::{CodecAttr, CodecFields},
    keylen::KeylenXcoder,
};
use crate::{
    Ctxt,
    ast::{
        attr::size::SizeBytes,
        symbol,
        types::{HeaderEncoder, XcoderType},
    },
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;

#[derive(Debug, Clone)]
/// The encoder, decoder and size of a KLV key or length prefix
///
/// `enc` and `dec` are optional because `allow_unimplemented_encode` or
/// `allow_unimplemented_decode` may be set on the container, in which case
/// a missing encoder or decoder is not an error
pub(crate) struct HeaderXcoder {
    /// The encoder, or `None` when unimplemented encoding is allowed
    pub(crate) enc: Option<XcoderType>,

    /// The decoder, or `None` when unimplemented decoding is allowed
    pub(crate) dec: Option<XcoderType>,

    /// The declared byte count of the key or length prefix ([`SizeBytes::Unset`] ⇒
    /// variable-width); see [`SizeBytes`]
    pub(crate) size: SizeBytes,
}
/// [`HeaderXcoder`] implementation of [`TryFrom`] for [`syn::MetaList`]
///
/// Parses the `enc = ..`, `dec = ..` and `size(..)` sub-fields from a `key(..)` or `len(..)`
/// attribute list into a [`HeaderXcoder`]. Unknown fields and duplicate
/// encoder/decoder entries are returned as [`syn::Error`] values rather than
/// stored internally, so the caller can surface them via the [`crate::Ctxt`]
/// accumulator immediately
impl TryFrom<&syn::MetaList> for HeaderXcoder {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaList) -> syn::Result<Self> {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut enc: Option<HeaderEncoder> = None;
        let mut dec: Option<XcoderType> = None;
        let mut size: Option<SizeBytes> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        input.parse_nested_meta(|meta| {
            tk_syn_macros::handle_unique_nested_meta_values! {
                meta;
                err!(UnknownHeaderField(meta.path));
                3;
                enc: symbol::pnm_parse_maybestr_header_encoder => err!(DuplicateEncoderInHeader),
                dec: symbol::pnm_parse_maybestr_decoder => err!(DuplicateDecoderInHeader),
                size: symbol::parse_pnm_header_size => err!(DuplicateSizeInHeader),
            }
        })?;
        // --------------------------------------------------
        // return (size is validated during its own parse, and a list without `size(..)` declares
        // no byte count)
        // --------------------------------------------------
        Ok(HeaderXcoder {
            enc: enc.map(|HeaderEncoder(enc)| enc),
            dec,
            size: size.unwrap_or_default(),
        })
    }
}
/// [`HeaderXcoder`] implementation of [`CodecFields`]
impl CodecFields for HeaderXcoder {
    fn has_enc(&self) -> bool {
        self.enc.is_some()
    }

    fn has_dec(&self) -> bool {
        self.dec.is_some()
    }
}

/// The raw states of a container's `key(..)`, `len(..)` and fused `keylen(..)` attributes
#[derive(Debug, Default)]
pub(super) struct HeaderAttrs {
    /// `key(..)`
    pub(super) key: CodecAttr<HeaderXcoder>,

    /// `len(..)`
    pub(super) len: CodecAttr<HeaderXcoder>,

    /// `keylen(..)`
    pub(super) keylen: CodecAttr<KeylenXcoder>,
}
/// [`HeaderAttrs`] implementation
impl HeaderAttrs {
    /// Reports every parsed attribute's missing `enc` and `dec`, in slot order
    /// ([`CodecAttr::report_missing`])
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context the errors are reported to
    /// * `allow` - Whether `allow_unimplemented_encode` and `allow_unimplemented_decode` are set
    pub(super) fn report_missing(&self, cx: &Ctxt, allow: AllowUnimplemented) {
        self.key.report_missing(cx, allow);
        self.len.report_missing(cx, allow);
        self.keylen.report_missing(cx, allow);
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Ctxt, Error,
        ast::{attr::MainContainer, symbol},
    };

    use quote::quote;

    #[test]
    /// Verifies every wrong occurrence of a frame header attribute is blamed on that attribute
    ///
    /// For each of `key(..)`, `len(..)` and `keylen(..)`, writes a duplicate list and both
    /// non-list forms (`x = ..` and a bare `x`) beside an otherwise complete framing, and checks
    /// that the only error reported is the one naming that attribute. A slot wired to a sibling's
    /// name or duplicate message would report a valid-looking error about the wrong attribute
    fn wrong_header_form_names_its_own_slot() {
        let key_list = quote! { key(dec = decb::u8) };
        let len_list = quote! { len(dec = decb::u8_as_usize) };
        let keylen_list = quote! { keylen(dec = header) };
        // --------------------------------------------------
        // (name, container header attributes, the one error)
        // --------------------------------------------------
        let cases = [
            (
                "duplicate key",
                quote! { #key_list, #key_list, #len_list },
                Error::DuplicateKey,
            ),
            (
                "name-value key",
                quote! { key = decb::u8, #len_list },
                Error::ExpectedAsList(symbol::KEY.to_string()),
            ),
            (
                "bare key",
                quote! { key, #len_list },
                Error::ExpectedAsList(symbol::KEY.to_string()),
            ),
            (
                "duplicate len",
                quote! { #key_list, #len_list, #len_list },
                Error::DuplicateLength,
            ),
            (
                "name-value len",
                quote! { #key_list, len = decb::u8_as_usize },
                Error::ExpectedAsList(symbol::LENGTH.to_string()),
            ),
            (
                "bare len",
                quote! { #key_list, len },
                Error::ExpectedAsList(symbol::LENGTH.to_string()),
            ),
            (
                "duplicate keylen",
                quote! { #keylen_list, #keylen_list },
                Error::DuplicateKeylen,
            ),
            (
                "name-value keylen",
                quote! { keylen = header },
                Error::ExpectedAsList(symbol::KEYLEN.to_string()),
            ),
            (
                "bare keylen",
                quote! { keylen },
                Error::ExpectedAsList(symbol::KEYLEN.to_string()),
            ),
        ];
        for (name, header, expected) in cases {
            let item: syn::DeriveInput = syn::parse_quote! {
                #[klv(#header, allow_unimplemented_encode)]
                struct Reading {
                    #[klv(key = 0x01, dec = decb::u8)]
                    id: u8,
                }
            };
            let cx = Ctxt::new();
            let _ = MainContainer::from_ast(&cx, &item);
            let Err(errors) = cx.check() else {
                panic!("{name}: reports an error");
            };
            let messages: Vec<String> = errors.into_iter().map(|err| err.to_string()).collect();
            assert_eq!(
                messages,
                [expected.to_string()],
                "{name}: the one error names its slot"
            );
        }
    }

    #[test]
    /// Verifies a `&` or `*` sigil on any frame header encoder is the one error reported
    ///
    /// The derive passes each header encoder its key and length by value, so a sigil would be
    /// silently ignored; every slot and both sigils must be rejected
    fn sigil_on_header_encoder_is_rejected() {
        let key_list = quote! { key(dec = decb::u8, enc = encb::u8) };
        let len_list = quote! { len(dec = decb::u8_as_usize, enc = encb::u8_from_usize) };
        // --------------------------------------------------
        // (name, container header attributes)
        // --------------------------------------------------
        let cases = [
            (
                "& key",
                quote! { key(dec = decb::u8, enc = &encb::u8), #len_list },
            ),
            (
                "* key",
                quote! { key(dec = decb::u8, enc = *encb::u8), #len_list },
            ),
            (
                "& len",
                quote! { #key_list, len(dec = decb::u8_as_usize, enc = &encb::u8_from_usize) },
            ),
            (
                "* len",
                quote! { #key_list, len(dec = decb::u8_as_usize, enc = *encb::u8_from_usize) },
            ),
            (
                "& keylen",
                quote! { keylen(dec = header, enc = &header_enc) },
            ),
            (
                "* keylen",
                quote! { keylen(dec = header, enc = *header_enc) },
            ),
            (
                "quoted & keylen",
                quote! { keylen(dec = header, enc = "&header_enc") },
            ),
        ];
        for (name, header) in cases {
            let item: syn::DeriveInput = syn::parse_quote! {
                #[klv(#header)]
                struct Reading {
                    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
                    id: u8,
                }
            };
            let cx = Ctxt::new();
            let _ = MainContainer::from_ast(&cx, &item);
            let Err(errors) = cx.check() else {
                panic!("{name}: reports an error");
            };
            let messages: Vec<String> = errors.into_iter().map(|err| err.to_string()).collect();
            assert_eq!(
                messages,
                [Error::SigilOnHeaderEncoder.to_string()],
                "{name}: the one error is the sigil"
            );
        }
    }
}
