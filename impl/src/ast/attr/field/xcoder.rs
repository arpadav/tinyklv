//! Raw parsed accumulator for a single field's `#[klv(..)]` settings
//!
//! Defines [`FieldXcoder`], which accumulates every recognized sub-attribute
//! from one `#[klv(..)]` annotation on a struct field: the key literal, the
//! optional encoder and decoder, the value size shape, the `latebind`
//! post-decode step, and field-level `default`. The [`From<&syn::MetaList>`]
//! impl drives the actual `parse_nested_meta` parse; duplicate-field errors
//! are stored in `FieldXcoder::errors` so the caller can forward them to
//! the [`crate::Ctxt`] accumulator
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::size::SizeSpec;
use crate::ast::symbol;
use crate::ast::types::{DefaultValue, LatebindXcoder, SiguledXcoder, XcoderType};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use tk_syn_macros::handle_unique_nested_meta_values;

#[derive(Debug, Default)]
pub(crate) struct FieldXcoder {
    /// The key for the field
    pub key: Option<syn::Lit>,
    /// The encoder for the field, with optional dispatch sigil
    pub enc: Option<SiguledXcoder>,
    /// The decoder for the field
    pub dec: Option<XcoderType>,
    /// The declared size shape of the field's value; see [`SizeSpec`]
    pub size: Option<SizeSpec>,
    /// Post-decode conversion or in-place mutation (`latebind = path` or
    /// `latebind = &mut path`). `None` means no post-decode step
    pub latebind: Option<LatebindXcoder>,
    /// syn errors
    pub errors: Option<syn::Error>,
    /// Fallback value used when the field's key is absent from the input
    ///
    /// See [`DefaultValue`] for the two accepted forms
    pub default: Option<DefaultValue>,
    /// `true` if `enc` was synthesized from the `trait_fallback` container flag
    /// rather than supplied by the user or a container `default(..)` match
    pub fallback_enc: bool,
    /// `true` if `dec` was synthesized from the `trait_fallback` container flag
    /// rather than supplied by the user or a container `default(..)` match
    pub fallback_dec: bool,
}
/// [`FieldXcoder`] implementation of [`From`] for [`syn::MetaList`]
///
/// Parses the nested meta arguments from a field-level `#[klv(key = .., enc =
/// .., dec = .., size(..), latebind = .., default = ..)]` attribute list
/// into a [`FieldXcoder`]
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
        let mut key: Option<syn::Lit> = None;
        let mut enc: Option<SiguledXcoder> = None;
        let mut dec: Option<XcoderType> = None;
        let mut size: Option<SizeSpec> = None;
        let mut latebind: Option<LatebindXcoder> = None;
        let mut default: Option<DefaultValue> = None;
        // --------------------------------------------------
        // parse nested meta
        // --------------------------------------------------
        let errors = input
            .parse_nested_meta(|meta| {
                handle_unique_nested_meta_values! {
                    meta;
                    err!(UnknownFieldField(meta.path));
                    6;
                    key: symbol::parse_pnm_key                      => err!(DuplicateKeyInField),
                    enc: symbol::pnm_parse_maybestr_encoder         => err!(DuplicateEncoderInField),
                    dec: symbol::pnm_parse_maybestr_decoder         => err!(DuplicateDecoderInField),
                    size: symbol::parse_pnm_size                    => err!(DuplicateSizeInField),
                    latebind: symbol::pnm_parse_maybestr_latebind   => err!(DuplicateLatebindInField),
                    default: symbol::parse_pnm_default_value        => err!(DuplicateDefaultInField),
                }
            })
            .err();
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        FieldXcoder {
            key,
            enc,
            dec,
            size,
            latebind,
            errors,
            default,
            fallback_enc: false,
            fallback_dec: false,
        }
    }
}
