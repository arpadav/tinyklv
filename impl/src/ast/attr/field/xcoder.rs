//! Raw parsed accumulator for a single field's `#[klv(..)]` settings
//!
//! Defines [`FieldXcoder`], which accumulates every recognized sub-attribute
//! from one `#[klv(..)]` annotation on a struct field: the key literal or `oneof(..)`, the
//! value's [`ValueCodec`] (encoder, decoder and size shape), the `latebind`
//! post-decode step, field-level `default`, `break_on`, the skip, and the `unknown` flag of the
//! field that keeps the frames whose key no field declares. The [`From<&syn::MetaList>`]
//! impl drives the actual `parse_nested_meta` parse; duplicate-field errors
//! are stored in `FieldXcoder::errors` so the caller can forward them to
//! the [`crate::Ctxt`] accumulator
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::oneof::{KeySpec, Oneof};
use crate::ast::attr::{size::SizeSpec, skip_if::SkipIf};
use crate::ast::symbol;
use crate::ast::types::{DefaultValue, LatebindXcoder, SiguledXcoder, XcoderType};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use syn::spanned::Spanned;
use tk_syn_macros::handle_unique_nested_meta_values;

#[derive(Debug, Default)]
pub(crate) struct FieldXcoder {
    /// The key for the field
    pub key: Option<syn::Lit>,
    /// The arms of a `key = oneof(..)`, written in place of a key literal
    pub oneof: Option<Oneof>,
    /// The encoder, decoder and size of the field's value
    pub codec: ValueCodec,
    /// Post-decode conversion or in-place mutation (`latebind = path` or
    /// `latebind = &mut path`). `None` means no post-decode step
    pub latebind: Option<LatebindXcoder>,
    /// syn errors
    pub errors: Option<syn::Error>,
    /// Fallback value used when the field's key is absent from the input
    ///
    /// See [`DefaultValue`] for the two accepted forms
    pub default: Option<DefaultValue>,
    /// The `fn(&value) -> BreakType` consulted right after the field's value decodes
    pub break_on: Option<syn::Path>,
    /// When the field's frame is left out of the encode; see [`SkipIf`]
    pub skip_if: Option<SkipIf>,
    /// The span of the `unknown` flag, set on the field that keeps the unknown frames
    pub unknown: Option<proc_macro2::Span>,
    /// The span of the first setting the list names other than `unknown`, which a keyed field
    /// has and the field that keeps the unknown frames may not
    pub keyed_setting: Option<proc_macro2::Span>,
    /// The spans of the list's `enc`, `dec` and `size`, which a `key = oneof(..)` field may not
    /// have
    pub codec_settings: Vec<proc_macro2::Span>,
}

#[derive(Debug, Clone, Default)]
/// The codec of the value a key's frames carry: a field's own, or one `oneof(..)` arm's
pub(crate) struct ValueCodec {
    /// The encoder, with optional dispatch sigil
    pub enc: Option<SiguledXcoder>,
    /// The decoder
    pub dec: Option<XcoderType>,
    /// The declared size shape of the value; see [`SizeSpec`]
    pub size: Option<SizeSpec>,
    /// `true` if `enc` was synthesized from the `trait_fallback` container flag
    /// rather than supplied by the user or a container `default(..)` match
    pub fallback_enc: bool,
    /// `true` if `dec` was synthesized from the `trait_fallback` container flag
    /// rather than supplied by the user or a container `default(..)` match
    pub fallback_dec: bool,
}
/// [`ValueCodec`] implementation
impl ValueCodec {
    /// The span of the size's `implicit` keyword, if written: the value has no length prefix
    pub(crate) fn implicit(&self) -> Option<proc_macro2::Span> {
        self.size.as_ref().and_then(SizeSpec::implicit)
    }
}
/// [`FieldXcoder`] implementation of [`From`] for [`syn::MetaList`]
///
/// Parses the nested meta arguments from a field-level `#[klv(key = .. | oneof(..), enc =
/// .., dec = .., size(..), latebind = .., default = .., break_on = .., skip_if_default |
/// skip_if = ..)]` attribute list, or the bare `#[klv(unknown)]`, into a [`FieldXcoder`]
///
/// Duplicate-field errors and unknown-field errors are stored in
/// `FieldXcoder::errors` rather than returned directly, so that the caller
/// ([`super::super::Field::from_ast`]) can forward all diagnostics to the
/// [`crate::Ctxt`] accumulator in one pass before aborting
impl From<&syn::MetaList> for FieldXcoder {
    fn from(input: &syn::MetaList) -> Self {
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut key: Option<KeySpec> = None;
        let mut enc: Option<SiguledXcoder> = None;
        let mut dec: Option<XcoderType> = None;
        let mut size: Option<SizeSpec> = None;
        let mut latebind: Option<LatebindXcoder> = None;
        let mut default: Option<DefaultValue> = None;
        let mut break_on: Option<syn::Path> = None;
        let mut skip_if_default: Option<proc_macro2::Span> = None;
        let mut skip_if: Option<syn::Path> = None;
        let mut unknown: Option<proc_macro2::Span> = None;
        let mut keyed_setting: Option<proc_macro2::Span> = None;
        let mut codec_settings: Vec<proc_macro2::Span> = Vec::new();
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        let mut errors = input
            .parse_nested_meta(|meta| {
                if meta.path != symbol::UNKNOWN_FRAMES {
                    keyed_setting.get_or_insert_with(|| meta.path.span());
                }
                if meta.path == symbol::ENCODER
                    || meta.path == symbol::DECODER
                    || meta.path == symbol::SIZE
                {
                    codec_settings.push(meta.path.span());
                }
                handle_unique_nested_meta_values! {
                    meta;
                    err!(UnknownFieldField(meta.path));
                    10;
                    key: symbol::parse_pnm_key                      => err!(DuplicateKeyInField),
                    enc: symbol::pnm_parse_maybestr_encoder         => err!(DuplicateEncoderInField),
                    dec: symbol::pnm_parse_maybestr_decoder         => err!(DuplicateDecoderInField),
                    size: symbol::parse_pnm_size                    => err!(DuplicateSizeInField),
                    latebind: symbol::pnm_parse_maybestr_latebind   => err!(DuplicateLatebindInField),
                    default: symbol::parse_pnm_default_value        => err!(DuplicateDefaultInField),
                    break_on: symbol::pnm_parse_maybestr_break_on   => err!(DuplicateBreakOnInField),
                    skip_if_default: symbol::parse_pnm_skip_if_default => err!(SkipIfConflict),
                    skip_if: symbol::pnm_parse_maybestr_skip_if     => err!(SkipIfConflict),
                    unknown: symbol::parse_pnm_unknown              => err!(DuplicateUnknownInField),
                }
            })
            .err();
        // --------------------------------------------------
        // the two skip forms contradict
        // --------------------------------------------------
        let skip_if = match SkipIf::from_items(skip_if_default, skip_if) {
            Ok(skip_if) => skip_if,
            Err(e) => {
                match errors {
                    None => errors = Some(e),
                    Some(ref mut errors) => errors.combine(e),
                }
                None
            }
        };
        // --------------------------------------------------
        // a key literal, or the arms of a `oneof(..)`
        // --------------------------------------------------
        let (key, oneof) = match key {
            Some(KeySpec::Literal(key)) => (Some(key), None),
            Some(KeySpec::Oneof(oneof)) => (None, Some(oneof)),
            None => (None, None),
        };
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        FieldXcoder {
            key,
            oneof,
            codec: ValueCodec {
                enc,
                dec,
                size,
                ..ValueCodec::default()
            },
            latebind,
            errors,
            default,
            break_on,
            skip_if,
            unknown,
            keyed_setting,
            codec_settings,
        }
    }
}
