//! Parsing of field-level `#[klv(..)]` attributes
//!
//! Defines [`Field`] (the raw parsed form of a single struct field's KLV
//! annotations) and [`FieldParsed`] (the validated form that guarantees a
//! `key` literal is present). Container-level `default(..)` xcoders are
//! applied here when a field does not carry its own explicit encoder/decoder
//! The `trait_fallback` opt-in is also resolved at this stage, injecting
//! placeholder paths that the expand phase replaces with fully-qualified
//! `<T as EncodeValue<..>>::encode_value` / `<T as DecodeValue<..>>::decode_value` calls
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod xcoder;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::container::default::DefaultXcoder;
use xcoder::FieldXcoder;

// --------------------------------------------------
// external
// --------------------------------------------------
use crate::Ctxt;
use crate::ast::types::{
    DefaultValue, LatebindXcoder, SiguledXcoder, XcoderLike, XcoderSigil, XcoderType,
};
use crate::symbol;
use quote::ToTokens;
use std::collections::HashMap;

#[derive(Debug)]
/// Raw parsed form of a single struct field's `#[klv(..)]` annotations
///
/// Holds a [`FieldXcoder`] that may have been enriched with container-level
/// `default(..)` xcoders or `trait_fallback` placeholders. Fields with no
/// `#[klv(..)]` attribute at all produce `None` from [`Field::from_ast`]
pub(crate) struct Field {
    /// The combined key, encoder, decoder, and auxiliary settings for this field
    pub contents: FieldXcoder,
}
/// [`Field`] implementation
impl Field {
    /// Parses the `#[klv(..)]` attributes from a single struct field
    ///
    /// Scans every attribute on `field` for the top-level `klv` attribute name,
    /// collects all [`FieldXcoder`] entries, merges duplicates (emitting errors
    /// to `cx` for true conflicts), applies container-level `default(..)` xcoders
    /// for fields whose type matches, and injects `trait_fallback` placeholders
    /// when the opt-in flag is set and no explicit xcoder was found. Returns
    /// `None` when the field carries no `#[klv(..)]` attribute at all
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording diagnostics
    /// * `field` - The syn field whose attributes are parsed
    /// * `name` - The field ident, used as a span anchor for error messages
    /// * `container_defaults` - Per-type default xcoders from the container annotation
    /// * `allow_unimplemented_encode` - When `true`, missing encoders do not produce errors
    /// * `allow_unimplemented_decode` - When `true`, missing decoders do not produce errors
    /// * `trait_fallback` - When `true`, inject trait-impl placeholders for missing xcoders
    ///
    /// # Returns
    ///
    /// `Some(Field)` if the field carried at least one `#[klv(..)]` attribute, or
    /// `None` if no KLV annotation was found
    pub fn from_ast(
        cx: &Ctxt,
        field: &syn::Field,
        name: &syn::Ident,
        container_defaults: &HashMap<syn::Type, DefaultXcoder>,
        allow_unimplemented_encode: bool,
        allow_unimplemented_decode: bool,
        trait_fallback: bool,
    ) -> Option<Self> {
        // --------------------------------------------------
        // return if no attrs on field
        // --------------------------------------------------
        if field.attrs.is_empty() {
            return None;
        }

        // --------------------------------------------------
        // loop through attrs
        // --------------------------------------------------
        let mut all_field_xcoders = Vec::new();
        for attr in &field.attrs {
            if attr.path() != symbol::KLV_ATTR {
                continue;
            }
            // --------------------------------------------------
            // parse out klv attributes
            // --------------------------------------------------
            match attr.meta {
                syn::Meta::List(ref contents) => {
                    all_field_xcoders.push(FieldXcoder::from(contents));
                }
                _ => {
                    cx.error_spanned_by(attr, err!(MalformedField));
                }
            }
        }

        // --------------------------------------------------
        // return if no attrs == symbol::KLV_ATTR
        // --------------------------------------------------
        if all_field_xcoders.is_empty() {
            return None;
        }

        // --------------------------------------------------
        // add all `syn_error`'s to `cx`
        // --------------------------------------------------
        all_field_xcoders
            .iter()
            .filter_map(|f| f.errors.clone())
            .for_each(|e| cx.syn_error(e));

        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut field_xcoder = FieldXcoder::default();

        // --------------------------------------------------
        // get all klv attr keys
        // --------------------------------------------------
        let keys = all_field_xcoders
            .iter()
            .filter_map(|f| f.key.clone())
            .collect::<Vec<_>>();
        field_xcoder.key = match keys.len() {
            0 => {
                cx.error_spanned_by(field, err!(MissingKeyInField));
                None
            }
            1 => Some(keys[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateKey));
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr encoders
        // --------------------------------------------------
        let mut keep_enc_none = false;
        let encoders = all_field_xcoders
            .iter()
            .filter_map(|f| f.enc.clone())
            .collect::<Vec<_>>();
        field_xcoder.enc = match encoders.len() {
            0 => None,
            1 => Some(encoders[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateEncoderInField));
                keep_enc_none = true;
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr decoders
        // --------------------------------------------------
        let mut keep_dec_none = false;
        let decoders = all_field_xcoders
            .iter()
            .filter_map(|f| f.dec.clone())
            .collect::<Vec<_>>();
        field_xcoder.dec = match decoders.len() {
            0 => None,
            1 => Some(decoders[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateDecoderInField));
                keep_dec_none = true;
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr var
        // --------------------------------------------------
        let vars = all_field_xcoders
            .iter()
            .filter_map(|f| f.varlen.clone())
            .collect::<Vec<_>>();
        field_xcoder.varlen = match vars.len() {
            0 => None,
            1 => Some(vars[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateVariableLengthInField));
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr latebind
        // --------------------------------------------------
        let latebinds = all_field_xcoders
            .iter()
            .filter_map(|f| f.latebind.clone())
            .collect::<Vec<_>>();
        field_xcoder.latebind = match latebinds.len() {
            0 => None,
            1 => Some(latebinds[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateLatebindInField));
                None
            }
        };

        // --------------------------------------------------
        // get all klv attr default's
        // --------------------------------------------------
        let defaults = all_field_xcoders
            .iter()
            .filter_map(|f| f.default.clone())
            .collect::<Vec<_>>();
        field_xcoder.default = match defaults.len() {
            0 => None,
            1 => Some(defaults[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateDefaultInField));
                None
            }
        };

        // --------------------------------------------------
        // set defaults, if no enc/dec was found
        // --------------------------------------------------
        let typ_maybe_unwrapped =
            crate::expand::helpers::unwrap_option_type(&field.ty).unwrap_or(&field.ty);
        if let Some(default) = container_defaults.get(typ_maybe_unwrapped) {
            if let (Some(default_enc), false) = (&default.enc, keep_enc_none) {
                field_xcoder.enc = Some(default_enc.clone());
            }
            if let (Some(default_dec), false) = (&default.dec, keep_dec_none) {
                field_xcoder.dec = Some(default_dec.clone());
                field_xcoder.varlen.clone_from(&default.var);
            }
        }

        // --------------------------------------------------
        // fallback to trait impls (opt-in via `trait_fallback`)
        //
        // only engages when the user set the container-level flag AND no
        // explicit xcoder / container default filled the slot. `allow_unimplemented_*`
        // takes precedence - if either is set, fallback is skipped on that side
        // --------------------------------------------------
        if trait_fallback
            && !keep_enc_none
            && !allow_unimplemented_encode
            && field_xcoder.enc.is_none()
        {
            // placeholder path - replaced at emit time in `encode_impl.rs`
            // by a fully-qualified `<T as EncodeValue<Vec<u8>>>::encode_value`
            // call. `syn::Path` cannot represent the qualified form directly
            // (qself lives on `TypePath`/`ExprPath`), so a marker flag plus
            // a never-emitted placeholder keeps the type signature clean
            let placeholder: syn::Path = syn::parse_quote! { __tinyklv_fallback_enc };
            field_xcoder.enc = Some(SiguledXcoder {
                sigil: XcoderSigil::None,
                inner: XcoderLike::Path(placeholder),
            });
            field_xcoder.fallback_enc = true;
        }
        if trait_fallback
            && !keep_dec_none
            && !allow_unimplemented_decode
            && field_xcoder.dec.is_none()
        {
            let varlen_set = field_xcoder.varlen.as_ref().is_some_and(|v| v.value);
            if varlen_set {
                cx.error_spanned_by(
                    name.clone(),
                    err!(VarlenFallbackRequiresExplicitDec(name, typ_maybe_unwrapped)),
                );
            } else {
                // placeholder - see encode comment above. `decode_impl.rs`
                // branches on `fallback_dec` and emits the real qualified path
                let placeholder: syn::Path = syn::parse_quote! { __tinyklv_fallback_dec };
                field_xcoder.dec = Some(XcoderLike::Path(placeholder));
                field_xcoder.fallback_dec = true;
            }
        }
        // --------------------------------------------------
        // unimplemented encode error - only fires if fallback did not fill
        // --------------------------------------------------
        if !allow_unimplemented_encode && field_xcoder.enc.is_none() {
            cx.error_spanned_by(
                name.clone(),
                err!(UnimplementedEncode(name, typ_maybe_unwrapped)),
            );
        }
        // --------------------------------------------------
        // unimplemented decode error - only fires if fallback did not fill
        // --------------------------------------------------
        if !allow_unimplemented_decode && field_xcoder.dec.is_none() {
            cx.error_spanned_by(
                name.clone(),
                err!(UnimplementedDecode(name, typ_maybe_unwrapped)),
            );
        }
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Some(Field {
            contents: field_xcoder,
        })
    }
}

/// Validated form of a single field's KLV attributes, with a guaranteed key literal
///
/// Produced by [`FieldParsed::from_field`] once the presence of `key` has been
/// confirmed. Flags `fallback_enc` and `fallback_dec` signal that the encoder/decoder
/// path stored in `enc`/`dec` is a placeholder to be replaced by a fully-qualified
/// trait call during expansion, rather than a user-supplied function path
pub(crate) struct FieldParsed {
    /// The key literal used to identify this field in the KLV stream
    pub key: syn::Lit,

    /// The encoder xcoder, if one was resolved (explicit, default, or fallback)
    pub enc: Option<SiguledXcoder>,

    /// The decoder xcoder, if one was resolved (explicit, default, or fallback)
    pub dec: Option<XcoderType>,

    /// Whether the decoder expects a length argument (variable-length fields)
    pub var: Option<syn::LitBool>,

    /// Optional post-decode transformation applied via `.map(..)`
    pub latebind: Option<LatebindXcoder>,

    /// Field-level default value emitted when no value is decoded for this field
    pub default: Option<DefaultValue>,

    /// `true` when `enc` holds a trait-fallback placeholder rather than a real path
    pub fallback_enc: bool,

    /// `true` when `dec` holds a trait-fallback placeholder rather than a real path
    pub fallback_dec: bool,
}
/// [`FieldParsed`] implementation
impl FieldParsed {
    /// Converts a raw [`Field`] into a validated [`FieldParsed`]
    ///
    /// Checks that a key literal is present in `f.contents`, records a
    /// missing-key error in `cx` if absent, and projects all remaining
    /// xcoder settings into the returned struct
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording the missing-key diagnostic
    /// * `sf` - The syn field, used as a span anchor for error messages
    /// * `f` - The raw parsed field to validate and project
    ///
    /// # Returns
    ///
    /// `Some(FieldParsed)` when a key is present, or `None` if the key was absent
    pub fn from_field(cx: &Ctxt, sf: &syn::Field, f: &Field) -> Option<FieldParsed> {
        // --------------------------------------------------
        // check for required fields
        // --------------------------------------------------
        let Some(key) = &f.contents.key else {
            cx.error_spanned_by(sf.ident.clone(), err!(MissingKeyInField));
            return None;
        };
        // --------------------------------------------------
        // return parsed field
        // --------------------------------------------------
        Some(FieldParsed {
            key: key.clone(),
            enc: f.contents.enc.clone(),
            dec: f.contents.dec.clone(),
            var: f.contents.varlen.clone(),
            latebind: f.contents.latebind.clone(),
            default: f.contents.default.clone(),
            fallback_enc: f.contents.fallback_enc,
            fallback_dec: f.contents.fallback_dec,
        })
    }
}
