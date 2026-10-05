//! Parsing of the container-level `key(..)`, `len(..)`, and fused `keylen(..)` sub-attributes
//!
//! Defines [`HeaderXcoder`], which holds the optional encoder and decoder of one of
//! these sub-attributes. Both are optional because
//! `allow_unimplemented_encode` / `allow_unimplemented_decode` may suppress
//! the missing-xcoder error. [`TryFrom<&syn::MetaList>`] drives the
//! `parse_nested_meta` parse for `enc = ..`, `dec = ..` and `size(..)`
//!
//! Defines [`HeaderSlot`], which of the three sub-attributes an occurrence belongs to, and
//! [`HeaderXcoderAttr`], the raw state of each of the three sub-attributes on
//! a container: absent, present but unusable, or parsed into a [`HeaderXcoder`], and the
//! rules by which each occurrence of the attribute updates that state
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    Ctxt,
    ast::{
        attr::size::SizeSpec,
        symbol::{self, Symbol},
        types::{HeaderEncoder, XcoderType},
    },
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::borrow::Cow;

#[derive(Debug, Clone)]
/// The encoder and decoder for a KLV key, a length, or a fused key+length frame header
///
/// `enc` and `dec` are optional because `allow_unimplemented_encode` or
/// `allow_unimplemented_decode` may be set on the container, in which case
/// a missing encoder or decoder is not an error
pub(crate) struct HeaderXcoder {
    /// The encoder, or `None` when unimplemented encoding is allowed
    pub(crate) enc: Option<XcoderType>,

    /// The decoder, or `None` when unimplemented decoding is allowed
    pub(crate) dec: Option<XcoderType>,

    /// The declared size shape of the key or length prefix (absent ⇒ variable-width); see [`SizeSpec`]
    pub(crate) size: Option<SizeSpec>,
}
/// [`HeaderXcoder`] implementation of [`TryFrom`] for [`syn::MetaList`]
///
/// Parses the `enc = ..`, `dec = ..` and `size(..)` sub-fields from a `key(..)`,
/// `len(..)` or `keylen(..)` attribute list into a [`HeaderXcoder`]. Unknown fields and duplicate
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
        let mut size: Option<SizeSpec> = None;
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
                size: symbol::parse_pnm_size => err!(DuplicateSizeInHeader),
            }
        })?;
        // --------------------------------------------------
        // return (size is validated during its own parse)
        // --------------------------------------------------
        Ok(HeaderXcoder {
            enc: enc.map(|HeaderEncoder(enc)| enc),
            dec,
            size,
        })
    }
}

/// Which of the container's frame header attributes an occurrence belongs to
#[derive(Debug, Clone, Copy)]
pub(super) enum HeaderSlot {
    /// `key(..)`
    Key,
    /// `len(..)`
    Length,
    /// `keylen(..)`
    Keylen,
}
/// [`HeaderSlot`] implementation
impl HeaderSlot {
    /// The attribute's name
    ///
    /// # Returns
    ///
    /// The [`Symbol`] the attribute is written as
    fn symbol(self) -> Symbol {
        match self {
            Self::Key => symbol::KEY,
            Self::Length => symbol::LENGTH,
            Self::Keylen => symbol::KEYLEN,
        }
    }

    /// The error a second list occurrence of the attribute is reported with
    ///
    /// # Returns
    ///
    /// The duplicate-attribute message
    fn duplicate_error(self) -> Cow<'static, str> {
        match self {
            Self::Key => err!(DuplicateKey),
            Self::Length => err!(DuplicateLength),
            Self::Keylen => err!(DuplicateKeylen),
        }
    }
}

/// The raw state of a container's `key(..)`, `len(..)` or fused `keylen(..)` attribute
///
/// Each present state keeps the attribute's path, which spans the diagnostics that blame the
/// attribute itself. An [`Self::Invalid`] attribute still counts as present, so its own error is
/// not joined by a missing-framing or conflict error that only its absence would explain
#[derive(Debug)]
pub(super) enum HeaderXcoderAttr {
    /// The attribute did not appear
    Absent,
    /// The attribute appeared but is unusable (it failed to parse, or was written as a
    /// name-value or a bare path); its error is already reported. A later list occurrence is
    /// parsed in its place and reports no duplicate error
    Invalid(syn::Path),
    /// The attribute parsed into its xcoder, boxed as in `ContainerMode`
    Parsed(syn::Path, Box<HeaderXcoder>),
}
/// [`HeaderXcoderAttr`] implementation
impl HeaderXcoderAttr {
    /// Applies one `name(..)` list occurrence of the attribute
    ///
    /// A list after a [`Self::Parsed`] one is reported as the slot's duplicate error and leaves
    /// the state alone. Otherwise the list is parsed in place of the current state:
    /// [`Self::Parsed`], or [`Self::Invalid`] once the parse error is reported
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context the errors are reported to
    /// * `list` - The attribute list, e.g. `keylen(dec = ..)`
    /// * `slot` - Which attribute this is, naming its errors
    pub(super) fn on_list(&mut self, cx: &Ctxt, list: &syn::MetaList, slot: HeaderSlot) {
        if matches!(self, Self::Parsed(..)) {
            cx.error_spanned_by(&list.path, slot.duplicate_error());
            return;
        }
        let path = list.path.clone();
        *self = match HeaderXcoder::try_from(list) {
            Ok(xcoder) => Self::Parsed(path, Box::new(xcoder)),
            Err(err) => {
                cx.syn_error(err);
                Self::Invalid(path)
            }
        };
    }

    /// Applies one non-list occurrence of the attribute (`keylen = ..` or a bare `keylen`)
    ///
    /// Reports the wrong form. A first occurrence makes the attribute present as
    /// [`Self::Invalid`], so no missing-framing error follows; a later one leaves the earlier
    /// state alone and reports no duplicate
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context the wrong form is reported to
    /// * `path` - The attribute's path, spanning the error and later diagnostics
    /// * `slot` - Which attribute this is, naming the error
    pub(super) fn on_non_list(&mut self, cx: &Ctxt, path: &syn::Path, slot: HeaderSlot) {
        cx.error_spanned_by(path, err!(ExpectedAsList(slot.symbol())));
        if !self.is_present() {
            *self = Self::Invalid(path.clone());
        }
    }

    /// Whether the attribute appeared, usable or not
    ///
    /// # Returns
    ///
    /// `false` only for [`Self::Absent`]
    pub(super) fn is_present(&self) -> bool {
        !matches!(self, Self::Absent)
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
