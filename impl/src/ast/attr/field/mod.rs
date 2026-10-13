//! Parsing of field-level `#[klv(..)]` attributes
//!
//! Defines [`Field`] (the raw parsed form of a single struct field's KLV
//! annotations) and [`FieldParsed`] (the validated form that guarantees a
//! `key` is present). A container-level `default(..)` fills whichever of
//! the encoder, decoder, size and skip a field of its type leaves unset; `lengthed` goes with the
//! decoder. A skipped field that sets no `default` of its own decodes a missing frame to its
//! type's `Default::default()`.
//! The `trait_fallback` opt-in is also resolved at this stage, injecting
//! placeholder paths that the expand phase replaces with fully-qualified
//! `<T as EncodeValue>::encode_value` / `<T as DecodeValue<..>>::decode_value` calls.
//! A `key = oneof(..)` field takes no encoder, decoder or size: each arm's codec is filled the
//! same way from its arm's type, and the skip from the field's.
//! A `#[klv(unknown)]` field, which keeps the frames whose key no field declares, takes no other
//! setting and is not keyed: none of the above applies to it
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod oneof;
mod xcoder;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use oneof::{KeySpec, OneofKey};
pub(crate) use xcoder::ValueCodec;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    Ctxt,
    ast::{
        attr::{container::DefaultXcoder, size::SizeSpec, skip_if::SkipIf},
        types::{
            DefaultValue, LatebindXcoder, SiguledXcoder, XcoderLike, XcoderSigil, is_option,
            unwrap_option_type,
        },
    },
    symbol,
};
use xcoder::FieldXcoder;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::collections::HashMap;

#[derive(Debug)]
/// Raw parsed form of a single struct field's `#[klv(..)]` annotations
///
/// Fields with no `#[klv(..)]` attribute at all produce `None` from [`Field::from_ast`]
pub(crate) enum Field {
    /// A keyed field's combined key, encoder, decoder, and auxiliary settings, which may have
    /// been enriched with container-level `default(..)` xcoders or `trait_fallback` placeholders
    Keyed(Box<FieldXcoder>),
    /// A `key = oneof(..)` field, each arm's codec filled from its type
    Oneof(Box<FieldParsed>),
    /// The `#[klv(unknown)]` field, with the span of its `unknown` flag
    Unknown(proc_macro2::Span),
}
/// [`Field`] implementation
impl Field {
    /// Parses the `#[klv(..)]` attributes from a single struct field
    ///
    /// Scans every attribute on `field` for the top-level `klv` attribute name,
    /// collects all [`FieldXcoder`] entries, merges duplicates (emitting errors
    /// to `cx` for true conflicts), applies container-level `default(..)` xcoders
    /// for fields whose type matches, and injects `trait_fallback` placeholders
    /// when the opt-in flag is set and no explicit xcoder was found. A field flagged `unknown` is
    /// [`Field::Unknown`] before any of that, and any other setting beside the flag is an error.
    /// A `key = oneof(..)` field is [`Field::Oneof`], whose arms each get their codec that way
    /// from their type, and an `enc`, `dec` or `size(..)` beside it is an error.
    /// Returns `None` when the field carries no `#[klv(..)]` attribute at all, or one that failed
    /// to parse, whose error is the field's only one
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
    /// `Some(Field)` if the field carried at least one `#[klv(..)]` attribute and each parsed, or
    /// `None` if no KLV annotation was found or one failed to parse
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
        // a list that failed to parse stopped at its error, so the settings it holds are
        // incomplete and nothing more is checked on the field
        // --------------------------------------------------
        if all_field_xcoders.iter().any(|f| f.errors.is_some()) {
            return None;
        }
        // --------------------------------------------------
        // the field that keeps the unknown frames is not keyed: the container's `unknown(..)` is
        // its codec, so it takes no other setting
        // --------------------------------------------------
        let mut unknowns = all_field_xcoders.iter().filter_map(|f| f.unknown);
        if let Some(at) = unknowns.next() {
            if let Some(again) = unknowns.next() {
                cx.syn_error(syn::Error::new(again, err!(DuplicateUnknownInField)));
            }
            if let Some(setting) = all_field_xcoders.iter().find_map(|f| f.keyed_setting) {
                cx.syn_error(syn::Error::new(setting, err!(UnknownFieldWithSetting)));
            }
            return Some(Field::Unknown(at));
        }
        // --------------------------------------------------
        // init
        // --------------------------------------------------
        let mut field_xcoder = FieldXcoder::default();
        // --------------------------------------------------
        // get all klv attr keys, a `oneof(..)` counted as one, which leaves the key literal unset
        // --------------------------------------------------
        let keys = all_field_xcoders
            .iter()
            .filter_map(|f| f.key.clone())
            .collect::<Vec<_>>();
        let oneofs = all_field_xcoders
            .iter()
            .filter_map(|f| f.oneof.as_ref())
            .collect::<Vec<_>>();
        field_xcoder.key = match keys.len() + oneofs.len() {
            0 => {
                cx.error_spanned_by(field, err!(MissingKeyInField));
                None
            }
            1 => keys.first().cloned(),
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
            .filter_map(|f| f.codec.enc.clone())
            .collect::<Vec<_>>();
        field_xcoder.codec.enc = match encoders.len() {
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
            .filter_map(|f| f.codec.dec.clone())
            .collect::<Vec<_>>();
        field_xcoder.codec.dec = match decoders.len() {
            0 => None,
            1 => Some(decoders[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateDecoderInField));
                keep_dec_none = true;
                None
            }
        };
        // --------------------------------------------------
        // get all klv attr size
        // --------------------------------------------------
        let sizes = all_field_xcoders
            .iter()
            .filter_map(|f| f.codec.size.clone())
            .collect::<Vec<_>>();
        field_xcoder.codec.size = match sizes.len() {
            0 => None,
            1 => Some(sizes[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateSizeInField));
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
        // get all klv attr break_on's
        // --------------------------------------------------
        let break_ons = all_field_xcoders
            .iter()
            .filter_map(|f| f.break_on.clone())
            .collect::<Vec<_>>();
        field_xcoder.break_on = match break_ons.len() {
            0 => None,
            1 => Some(break_ons[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(DuplicateBreakOnInField));
                None
            }
        };
        // --------------------------------------------------
        // get all klv attr skips
        // --------------------------------------------------
        let skips = all_field_xcoders
            .iter()
            .filter_map(|f| f.skip_if.clone())
            .collect::<Vec<_>>();
        field_xcoder.skip_if = match skips.len() {
            0 => None,
            1 => Some(skips[0].clone()),
            _ => {
                cx.error_spanned_by(field, err!(SkipIfConflict));
                None
            }
        };
        // --------------------------------------------------
        // a container default of the field's type fills the skip the field left unset
        // --------------------------------------------------
        let typ_maybe_unwrapped = unwrap_option_type(&field.ty).unwrap_or(&field.ty);
        if field_xcoder.skip_if.is_none() {
            field_xcoder.skip_if = container_defaults
                .get(typ_maybe_unwrapped)
                .and_then(|default| default.skip_if.clone());
        }
        // --------------------------------------------------
        // a skipped field's missing frame decodes to its type's default, unless the field sets
        // its own; an `Option` field's already decodes to `None`
        // --------------------------------------------------
        if field_xcoder.skip_if.is_some() && field_xcoder.default.is_none() && !is_option(&field.ty)
        {
            field_xcoder.default = Some(DefaultValue::Call);
        }
        // --------------------------------------------------
        // each value codec is filled from its type's container default, else its trait impls
        // --------------------------------------------------
        let fill = CodecFill {
            cx,
            name,
            defaults: container_defaults,
            allow_encode: allow_unimplemented_encode,
            allow_decode: allow_unimplemented_decode,
            trait_fallback,
        };
        // --------------------------------------------------
        // a `oneof(..)` arm's value takes its own type's codec, so an `enc`, `dec` or `size` on
        // the field is an error
        // --------------------------------------------------
        if let ([], [oneof]) = (keys.as_slice(), oneofs.as_slice()) {
            all_field_xcoders
                .iter()
                .flat_map(|f| &f.codec_settings)
                .for_each(|&at| cx.syn_error(syn::Error::new(at, err!(OneofFieldWithCodec))));
            let arms = oneof
                .arms
                .iter()
                .map(|arm| OneofKey {
                    codec: fill.codec(ValueCodec::default(), false, false, &arm.ty, &arm.ty),
                    arm: arm.clone(),
                })
                .collect();
            return Some(Field::Oneof(Box::new(FieldParsed {
                keys: FieldKeys::Oneof(arms),
                latebind: field_xcoder.latebind,
                default: field_xcoder.default,
                break_on: field_xcoder.break_on,
                skip_if: field_xcoder.skip_if,
            })));
        }
        // --------------------------------------------------
        // the field's own value codec, filled from its type
        // --------------------------------------------------
        field_xcoder.codec = fill.codec(
            std::mem::take(&mut field_xcoder.codec),
            keep_enc_none,
            keep_dec_none,
            typ_maybe_unwrapped,
            name,
        );
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Some(Field::Keyed(Box::new(field_xcoder)))
    }
}

/// What fills a value codec where the field leaves it unset: the container `default(typ = ..)`
/// of the value's type, else, under `trait_fallback`, that type's trait impls
struct CodecFill<'a> {
    /// Error accumulation context for recording diagnostics
    cx: &'a Ctxt,

    /// The field ident, which an error names
    name: &'a syn::Ident,

    /// Per-type default xcoders from the container annotation
    defaults: &'a HashMap<syn::Type, DefaultXcoder>,

    /// When `true`, missing encoders do not produce errors
    allow_encode: bool,

    /// When `true`, missing decoders do not produce errors
    allow_decode: bool,

    /// When `true`, inject trait-impl placeholders for missing xcoders
    trait_fallback: bool,
}
/// [`CodecFill`] implementation
impl CodecFill<'_> {
    /// Fills the codec of a value of type `ty`
    ///
    /// The container default of `ty` fills each of enc, dec, `implicit` and the size's byte count
    /// `own` left unset; `lengthed` comes with whichever dec is used. A missing encoder or
    /// decoder is reported at `at`, unless `allow_unimplemented_*` or `trait_fallback` covers it
    ///
    /// # Arguments
    ///
    /// * `own` - The codec the field wrote
    /// * `keep_enc_none` - Leaves the encoder unset, a duplicate already reported
    /// * `keep_dec_none` - Leaves the decoder unset, a duplicate already reported
    /// * `ty` - The value's type
    /// * `at` - Where an error points
    ///
    /// # Returns
    ///
    /// The filled codec
    fn codec(
        &self,
        mut own: ValueCodec,
        keep_enc_none: bool,
        keep_dec_none: bool,
        ty: &syn::Type,
        at: &impl ToTokens,
    ) -> ValueCodec {
        let Self {
            cx,
            name,
            defaults,
            allow_encode,
            allow_decode,
            trait_fallback,
        } = *self;
        // --------------------------------------------------
        // a container default fills each of enc, dec, `implicit` and the size's byte count the
        // field left unset; `lengthed` comes with whichever dec the field uses
        // --------------------------------------------------
        let default = defaults.get(ty);
        let default_dec = own.dec.is_none()
            && !keep_dec_none
            && default.is_some_and(|default| default.dec.is_some());
        if let Some(default) = default {
            if own.enc.is_none() && !keep_enc_none {
                own.enc.clone_from(&default.enc);
            }
            if default_dec {
                own.dec.clone_from(&default.dec);
            }
        }
        // --------------------------------------------------
        // the size is filled, and checked, with or without a default
        // --------------------------------------------------
        let default_size = default.and_then(|default| default.size.as_ref());
        match SizeSpec::filled(own.size.take(), default_size, default_dec) {
            Ok(size) => own.size = size,
            Err(err) => cx.syn_error(err),
        }
        // --------------------------------------------------
        // fallback to trait impls (opt-in via `trait_fallback`)
        //
        // only engages when the user set the container-level flag AND no
        // explicit xcoder / container default filled the slot. `allow_unimplemented_*`
        // takes precedence - if either is set, fallback is skipped on that side
        // --------------------------------------------------
        if trait_fallback && !keep_enc_none && !allow_encode && own.enc.is_none() {
            // placeholder path - replaced at emit time in `encode_impl::klv_item_gen`
            // by a fully-qualified `<T as EncodeValue>::encode_value`
            // call. `syn::Path` cannot represent the qualified form directly
            // (qself lives on `TypePath`/`ExprPath`), so a marker flag plus
            // a never-emitted placeholder keeps the type signature clean
            let placeholder: syn::Path = syn::parse_quote! { __tinyklv_fallback_enc };
            own.enc = Some(SiguledXcoder {
                sigil: XcoderSigil::None,
                inner: XcoderLike::Path(placeholder),
            });
            own.fallback_enc = true;
        }
        if trait_fallback && !keep_dec_none && !allow_decode && own.dec.is_none() {
            let takes_len = own.size.as_ref().is_some_and(SizeSpec::takes_len);
            if takes_len {
                cx.error_spanned_by(at, err!(SizeLengthedFallbackRequiresExplicitDec(name, ty)));
            } else {
                // placeholder - see encode comment above. `decode_impl::key_match_gen`
                // branches on `fallback_dec` and emits the real qualified path
                let placeholder: syn::Path = syn::parse_quote! { __tinyklv_fallback_dec };
                own.dec = Some(XcoderLike::Path(placeholder));
                own.fallback_dec = true;
            }
        }
        // --------------------------------------------------
        // unimplemented encode error - only fires if fallback did not fill
        // --------------------------------------------------
        if !allow_encode && own.enc.is_none() {
            cx.error_spanned_by(at, err!(UnimplementedEncode(name, ty)));
        }
        // --------------------------------------------------
        // unimplemented decode error - only fires if fallback did not fill
        // --------------------------------------------------
        if !allow_decode && own.dec.is_none() {
            cx.error_spanned_by(at, err!(UnimplementedDecode(name, ty)));
        }
        own
    }
}

#[derive(Debug)]
/// Validated form of a single field's KLV attributes, with a guaranteed key
///
/// Produced by [`FieldParsed::from_field`] once the presence of `key` has been
/// confirmed, or by [`Field::from_ast`] for a `key = oneof(..)` field. Flags `fallback_enc` and
/// `fallback_dec` of a [`ValueCodec`] signal that the encoder/decoder path stored in `enc`/`dec`
/// is a placeholder to be replaced by a fully-qualified trait call during expansion, rather than
/// a user-supplied function path
pub(crate) struct FieldParsed {
    /// The keys whose frames decode into this field, each with the codec of the value they carry
    pub keys: FieldKeys,

    /// Optional post-decode transformation applied via `.map(..)`
    pub latebind: Option<LatebindXcoder>,

    /// Field-level default value emitted when no value is decoded for this field
    pub default: Option<DefaultValue>,

    /// The `fn(&value) -> BreakType` consulted right after the field's value decodes, if any
    pub break_on: Option<syn::Path>,

    /// When the field's frame is left out of the encode, its own or its default's
    pub skip_if: Option<SkipIf>,
}

#[derive(Debug)]
/// The keys of a field's frames
pub(crate) enum FieldKeys {
    /// `key = K`, whose frames carry the field's value
    One {
        /// The key literal used to identify this field in the KLV stream
        key: syn::Lit,
        /// The codec of the field's value
        codec: Box<ValueCodec>,
    },
    /// `key = oneof(..)`, one key per arm, whose frames carry the arm's type
    Oneof(Vec<OneofKey>),
}
/// [`FieldParsed`] implementation
impl FieldParsed {
    /// Converts a keyed field's raw settings ([`Field::Keyed`]) into a validated [`FieldParsed`]
    ///
    /// Checks that a key literal is present in `f`, records a
    /// missing-key error in `cx` if absent, and projects all remaining
    /// xcoder settings into the returned struct
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording the missing-key diagnostic
    /// * `sf` - The syn field, used as a span anchor for error messages
    /// * `f` - The raw parsed settings to validate and project
    ///
    /// # Returns
    ///
    /// `Some(FieldParsed)` when a key is present, or `None` if the key was absent
    pub fn from_field(cx: &Ctxt, sf: &syn::Field, f: &FieldXcoder) -> Option<FieldParsed> {
        // --------------------------------------------------
        // check for required fields
        // --------------------------------------------------
        let Some(key) = &f.key else {
            cx.error_spanned_by(sf.ident.clone(), err!(MissingKeyInField));
            return None;
        };
        // --------------------------------------------------
        // return parsed field
        // --------------------------------------------------
        Some(FieldParsed {
            keys: FieldKeys::One {
                key: key.clone(),
                codec: Box::new(f.codec.clone()),
            },
            latebind: f.latebind.clone(),
            default: f.default.clone(),
            break_on: f.break_on.clone(),
            skip_if: f.skip_if.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        Ctxt,
        ast::attr::{MainContainer, size::SizeSpec},
    };

    use quote::ToTokens;

    #[test]
    /// Verifies a container `default(typ = ..)` fills only what a field of its type leaves unset,
    /// with `lengthed` coming from whichever side's `dec` the field uses
    ///
    /// The default is `lengthed` with `exact = 4`. A field with its own `dec` takes the default's
    /// `exact` but not its `lengthed`; a field without one takes the default's `dec` and
    /// `lengthed`; a field's own `exact` stays
    fn a_default_fills_what_the_field_leaves_unset() {
        let item: syn::DeriveInput = syn::parse_quote! {
            #[klv(
                key(dec = decb::u8),
                len(dec = decb::u8_as_usize),
                default(typ = Name, dec = name_lengthed, size(lengthed, exact = 4)),
                allow_unimplemented_encode,
            )]
            struct Reading {
                #[klv(key = 0x01, dec = name_fixed)]
                own_dec: Name,
                #[klv(key = 0x02)]
                no_dec: Name,
                #[klv(key = 0x03, size(exact = 8))]
                own_exact: Name,
            }
        };
        let cx = Ctxt::new();
        let container = MainContainer::from_ast(&cx, &item);
        cx.check().expect("the input parses");
        let container = container.expect("the input is a container");
        let filled: Vec<String> = container
            .klv_keys()
            .map(|key| {
                let size = key.codec.size.as_ref();
                format!(
                    "{}: dec = {}, lengthed = {}, exact = {:?}",
                    key.field.name,
                    key.codec.dec.to_token_stream(),
                    size.is_some_and(SizeSpec::takes_len),
                    size.and_then(SizeSpec::exact_width),
                )
            })
            .collect();
        assert_eq!(
            filled,
            [
                "own_dec: dec = name_fixed, lengthed = false, exact = Some(4)",
                "no_dec: dec = name_lengthed, lengthed = true, exact = Some(4)",
                "own_exact: dec = name_lengthed, lengthed = true, exact = Some(8)",
            ],
        );
    }
}
