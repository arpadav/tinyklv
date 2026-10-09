//! The state of a container's codec attributes: `key(..)`, `len(..)`, the fused `keylen(..)` and
//! `unknown(..)`
//!
//! Defines [`Codec`], the encoder and decoder of a `keylen(..)` or `unknown(..)` list, and
//! [`CodecFields`], what each codec list's parsed fields report. [`CodecSlot`] names which of the
//! four attributes an occurrence belongs to, and [`CodecAttr`] is the raw state of one of them on
//! a container: absent, present but unusable, or parsed into its slot's [`CodecFields`], with the
//! rules by which each occurrence of the attribute updates that state
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    Ctxt,
    ast::{
        symbol::{self, Symbol},
        types::XcoderType,
    },
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::borrow::Cow;

/// The parsed fields of one codec attribute list, held by a [`CodecAttr`]
///
/// Each slot parses into its own type, so a `keylen(..)` holds no `size(..)`
pub(crate) trait CodecFields:
    for<'a> TryFrom<&'a syn::MetaList, Error = syn::Error>
{
    /// Whether the list set an `enc`
    fn has_enc(&self) -> bool;

    /// Whether the list set a `dec`
    fn has_dec(&self) -> bool;
}

#[derive(Debug, Clone)]
/// The encoder and decoder of a `keylen(..)` or `unknown(..)` list, its encoder an `E`
///
/// Both are optional because `allow_unimplemented_encode` or `allow_unimplemented_decode` may be
/// set on the container, in which case a missing encoder or decoder is not an error
pub(crate) struct Codec<E> {
    /// The encoder, or `None` when unimplemented encoding is allowed
    pub(crate) enc: Option<E>,

    /// The decoder, or `None` when unimplemented decoding is allowed
    pub(crate) dec: Option<XcoderType>,
}
/// [`Codec`] implementation of [`CodecFields`]
impl<E> CodecFields for Codec<E>
where
    Self: for<'a> TryFrom<&'a syn::MetaList, Error = syn::Error>,
{
    fn has_enc(&self) -> bool {
        self.enc.is_some()
    }

    fn has_dec(&self) -> bool {
        self.dec.is_some()
    }
}

/// Which of the container's codec attributes an occurrence belongs to
#[derive(Debug, Clone, Copy)]
pub(super) enum CodecSlot {
    /// `key(..)`
    Key,
    /// `len(..)`
    Length,
    /// `keylen(..)`
    Keylen,
    /// `unknown(..)`
    Unknown,
}
/// [`CodecSlot`] implementation
impl CodecSlot {
    /// Every slot, the domain of [`Self::from_symbol`]
    const ALL: [Self; 4] = [Self::Key, Self::Length, Self::Keylen, Self::Unknown];

    /// The slot an attribute name belongs to
    ///
    /// # Arguments
    ///
    /// * `name` - The attribute's path, as a [`Symbol`]
    ///
    /// # Returns
    ///
    /// The slot whose [`Self::symbol`] is `name`, or `None` for any other attribute
    pub(super) fn from_symbol(name: Symbol) -> Option<Self> {
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
            Self::Unknown => symbol::UNKNOWN_FRAMES,
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
            Self::Unknown => err!(DuplicateUnknown),
        }
    }
}

/// The raw state of a container's `key(..)`, `len(..)`, fused `keylen(..)` or `unknown(..)`
/// attribute
///
/// Each present state keeps the attribute's path, which spans the diagnostics that blame the
/// attribute itself. An [`Self::Invalid`] attribute still counts as present, so its own error is
/// not joined by a missing-framing or conflict error that only its absence would explain
#[derive(Debug, Default)]
pub(crate) enum CodecAttr<T> {
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
/// [`CodecAttr`] implementation
impl<T: CodecFields> CodecAttr<T> {
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
    pub(super) fn on_meta(&mut self, cx: &Ctxt, meta: &syn::Meta, slot: CodecSlot) {
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
    fn on_list(&mut self, cx: &Ctxt, list: &syn::MetaList, slot: CodecSlot) {
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
    fn on_non_list(&mut self, cx: &Ctxt, path: &syn::Path, slot: CodecSlot) {
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
    /// * `allow_encode` - Whether a missing `enc` is allowed
    /// * `allow_decode` - Whether a missing `dec` is allowed
    pub(crate) fn report_missing(&self, cx: &Ctxt, allow_encode: bool, allow_decode: bool) {
        let Self::Parsed(path, fields) = self else {
            return;
        };
        if !allow_encode && !fields.has_enc() {
            cx.error_spanned_by(path, err!(MissingEncInCodec(path)));
        }
        if !allow_decode && !fields.has_dec() {
            cx.error_spanned_by(path, err!(MissingDecInCodec(path)));
        }
    }
}
