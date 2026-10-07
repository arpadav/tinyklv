//! Parsing of the container-level `key(..)` and `len(..)` sub-attributes, and the state of every
//! frame header attribute, the fused `keylen(..)` included
//!
//! Defines [`HeaderXcoder`], which holds the optional encoder and decoder of a `key(..)` or
//! `len(..)`, and the optional byte count of its `size(..)`. Both xcoders are optional because
//! `allow_unimplemented_encode` / `allow_unimplemented_decode` may suppress
//! the missing-xcoder error. [`TryFrom<&syn::MetaList>`] drives the
//! `parse_nested_meta` parse for `enc = ..`, `dec = ..` and `size(..)`
//!
//! Defines [`HeaderSlot`], which of the three sub-attributes an occurrence belongs to, and
//! [`HeaderXcoderAttr`], the raw state of each of the three sub-attributes on
//! a container: absent, present but unusable, or parsed into its slot's [`HeaderFields`]
//! ([`HeaderXcoder`] or [`KeylenXcoder`]), and the rules by which each occurrence of the
//! attribute updates that state. [`HeaderAttrs`] holds the three states, indexed by slot
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::keylen::KeylenXcoder;
use crate::{
    Ctxt,
    ast::{
        attr::size::SizeBytes,
        symbol::{self, Symbol},
        types::{HeaderEncoder, XcoderType},
    },
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::borrow::Cow;

/// The parsed fields of one frame header attribute list, held by a [`HeaderXcoderAttr`]
///
/// Each slot parses into its own type, so a `keylen(..)` holds no `size(..)`
pub(super) trait HeaderFields:
    for<'a> TryFrom<&'a syn::MetaList, Error = syn::Error>
{
    /// Whether the list set an `enc`
    fn has_enc(&self) -> bool;

    /// Whether the list set a `dec`
    fn has_dec(&self) -> bool;
}

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

    /// The declared byte count of the key or length prefix (absent ⇒ variable-width); see
    /// [`SizeBytes`]
    pub(crate) size: Option<SizeBytes>,
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
        // return (size is validated during its own parse)
        // --------------------------------------------------
        Ok(HeaderXcoder {
            enc: enc.map(|HeaderEncoder(enc)| enc),
            dec,
            size,
        })
    }
}
/// [`HeaderXcoder`] implementation of [`HeaderFields`]
impl HeaderFields for HeaderXcoder {
    fn has_enc(&self) -> bool {
        self.enc.is_some()
    }

    fn has_dec(&self) -> bool {
        self.dec.is_some()
    }
}

/// Which of the container's frame header attributes an occurrence belongs to
#[derive(Debug, Clone, Copy)]
enum HeaderSlot {
    /// `key(..)`
    Key,
    /// `len(..)`
    Length,
    /// `keylen(..)`
    Keylen,
}
/// [`HeaderSlot`] implementation
impl HeaderSlot {
    /// Every slot, the domain of [`Self::from_symbol`]
    const ALL: [Self; 3] = [Self::Key, Self::Length, Self::Keylen];

    /// The slot an attribute name belongs to
    ///
    /// # Arguments
    ///
    /// * `name` - The attribute's path, as a [`Symbol`]
    ///
    /// # Returns
    ///
    /// The slot whose [`Self::symbol`] is `name`, or `None` for any other attribute
    fn from_symbol(name: Symbol) -> Option<Self> {
        Self::ALL.into_iter().find(|slot| slot.symbol() == name)
    }

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
#[derive(Debug, Default)]
pub(super) enum HeaderXcoderAttr<T> {
    /// The attribute did not appear
    #[default]
    Absent,
    /// The attribute appeared but is unusable (it failed to parse, or was written as a
    /// name-value or a bare path); its error is already reported. A later list occurrence is
    /// parsed in its place and reports no duplicate error
    Invalid(syn::Path),
    /// The attribute parsed into its slot's fields, boxed as in `ContainerMode`
    Parsed(syn::Path, Box<T>),
}
/// [`HeaderXcoderAttr`] implementation
impl<T: HeaderFields> HeaderXcoderAttr<T> {
    /// Applies one occurrence of the attribute, in any form
    ///
    /// A list is parsed ([`Self::on_list`]); a name-value or a bare path is the wrong form
    /// ([`Self::on_non_list`])
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context the errors are reported to
    /// * `meta` - The occurrence, e.g. `keylen(dec = ..)`
    /// * `slot` - Which attribute this is, naming its errors
    fn on_meta(&mut self, cx: &Ctxt, meta: &syn::Meta, slot: HeaderSlot) {
        match meta {
            syn::Meta::List(list) => self.on_list(cx, list, slot),
            syn::Meta::NameValue(syn::MetaNameValue { path, .. }) | syn::Meta::Path(path) => {
                self.on_non_list(cx, path, slot);
            }
        }
    }

    /// Applies one `name(..)` list occurrence of the attribute
    ///
    /// A list after a [`Self::Parsed`] one is reported as the slot's duplicate error and leaves
    /// the state alone. Otherwise the list is parsed into the slot's fields in place of the
    /// current state: [`Self::Parsed`], or [`Self::Invalid`] once the error is reported
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context the errors are reported to
    /// * `list` - The attribute list, e.g. `keylen(dec = ..)`
    /// * `slot` - Which attribute this is, naming its errors
    fn on_list(&mut self, cx: &Ctxt, list: &syn::MetaList, slot: HeaderSlot) {
        if matches!(self, Self::Parsed(..)) {
            cx.error_spanned_by(&list.path, slot.duplicate_error());
            return;
        }
        let path = list.path.clone();
        *self = match T::try_from(list) {
            Ok(fields) => Self::Parsed(path, Box::new(fields)),
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
    fn on_non_list(&mut self, cx: &Ctxt, path: &syn::Path, slot: HeaderSlot) {
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

    /// Reports the missing `enc` and `dec` of a parsed attribute, spanned on its path, unless the
    /// container allows them
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context the errors are reported to
    /// * `allow_encode` - Whether `allow_unimplemented_encode` is set
    /// * `allow_decode` - Whether `allow_unimplemented_decode` is set
    fn report_missing(&self, cx: &Ctxt, allow_encode: bool, allow_decode: bool) {
        let Self::Parsed(path, fields) = self else {
            return;
        };
        if !allow_encode && !fields.has_enc() {
            cx.error_spanned_by(path, err!(MissingEncInHeader(path)));
        }
        if !allow_decode && !fields.has_dec() {
            cx.error_spanned_by(path, err!(MissingDecInHeader(path)));
        }
    }
}

/// The raw states of a container's `key(..)`, `len(..)` and fused `keylen(..)` attributes, each
/// indexed by its [`HeaderSlot`]
#[derive(Debug, Default)]
pub(super) struct HeaderAttrs {
    /// `key(..)`
    pub(super) key: HeaderXcoderAttr<HeaderXcoder>,

    /// `len(..)`
    pub(super) len: HeaderXcoderAttr<HeaderXcoder>,

    /// `keylen(..)`
    pub(super) keylen: HeaderXcoderAttr<KeylenXcoder>,
}
/// [`HeaderAttrs`] implementation
impl HeaderAttrs {
    /// Applies one occurrence of a frame header attribute, in any form, to the state of the slot
    /// its path names ([`HeaderSlot::from_symbol`])
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context the errors are reported to
    /// * `meta` - The occurrence, e.g. `keylen(dec = ..)`
    ///
    /// # Returns
    ///
    /// Whether `meta` is a `key`, `len` or `keylen` attribute
    #[must_use]
    pub(super) fn on_meta(&mut self, cx: &Ctxt, meta: &syn::Meta) -> bool {
        let Some(slot) = HeaderSlot::from_symbol(Symbol::from(meta.path())) else {
            return false;
        };
        // --------------------------------------------------
        // the occurrence is recorded on the attribute of its slot
        // --------------------------------------------------
        match slot {
            HeaderSlot::Key => self.key.on_meta(cx, meta, slot),
            HeaderSlot::Length => self.len.on_meta(cx, meta, slot),
            HeaderSlot::Keylen => self.keylen.on_meta(cx, meta, slot),
        }
        true
    }

    /// Reports every parsed attribute's missing `enc` and `dec`, in slot order
    /// ([`HeaderXcoderAttr::report_missing`])
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context the errors are reported to
    /// * `allow_encode` - Whether `allow_unimplemented_encode` is set
    /// * `allow_decode` - Whether `allow_unimplemented_decode` is set
    pub(super) fn report_missing(&self, cx: &Ctxt, allow_encode: bool, allow_decode: bool) {
        self.key.report_missing(cx, allow_encode, allow_decode);
        self.len.report_missing(cx, allow_encode, allow_decode);
        self.keylen.report_missing(cx, allow_encode, allow_decode);
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
