//! Parsing of field-level `#[klv(..)]` attributes
//!
//! Defines [`Field`] (what a struct field's KLV annotations make it, each annotation list
//! parsed raw first) and [`FieldParsed`] (the validated form of a keyed field, with exactly one
//! `key`). A container-level `default(..)` fills whichever of
//! the encoder, decoder, size and skip a field of its type leaves unset; `lengthed` goes with the
//! decoder. A skipped field that sets no `default` of its own decodes a missing frame to its
//! type's `Default::default()`.
//! The `trait_fallback` opt-in is also resolved at this stage, choosing the value type's trait
//! impl, which the expand phase calls as a fully-qualified
//! `<T as EncodeValue>::encode_value` / `<T as DecodeValue<..>>::decode_value`.
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
pub(crate) use xcoder::{ValueCodec, ValueXcoder};

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::{
    Ctxt,
    ast::{
        attr::{
            container::{AllowUnimplemented, Container, DefaultXcoder},
            size::SizeSpec,
            skip_if::SkipIf,
            unique::Unique,
        },
        types::{DefaultValue, LatebindXcoder, is_option, ungrouped, unwrap_option_type},
    },
    symbol,
};
use xcoder::{FieldXcoder, WrittenCodec};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::collections::HashMap;

#[derive(Debug)]
/// What a single struct field's `#[klv(..)]` annotations make it, once checked
///
/// Fields with no `#[klv(..)]` attribute at all produce `None` from [`Field::from_ast`]
pub(crate) enum Field {
    /// A keyed field: its key or `key = oneof(..)` arms, each value codec enriched with
    /// container-level `default(..)` xcoders or `trait_fallback` impls, and its auxiliary
    /// settings
    Keyed(Box<FieldParsed>),
    /// The `#[klv(unknown)]` field, with the span of its `unknown` flag
    Unknown(proc_macro2::Span),
}
/// [`Field`] implementation
impl Field {
    /// Parses the `#[klv(..)]` attributes from a single struct field
    ///
    /// Scans every attribute on `field` for the top-level `klv` attribute name,
    /// collects all [`FieldXcoder`] entries, merges each setting across them (one written in more
    /// than one list is an error at the field; a duplicate size is dropped and any other setting
    /// keeps its first, so nothing more is reported on it), applies container-level `default(..)`
    /// xcoders for fields whose type matches, and chooses the `trait_fallback` impls
    /// when the opt-in flag is set and no explicit xcoder was found. A field flagged `unknown` is
    /// [`Field::Unknown`] before any of that, and any other setting beside the flag is an error.
    /// A `key = oneof(..)` field's arms each get their codec that way from their type, and an
    /// `enc`, `dec` or `size(..)` beside it is an error.
    /// Returns `None` when the field carries no `#[klv(..)]` attribute at all, or one that failed
    /// to parse, whose error is the field's only one, or no key or more than one, which is
    /// reported once and leaves the value codec unchecked
    ///
    /// # Arguments
    ///
    /// * `fill` - The container's `default(..)` xcoders and flags, and the error accumulation
    ///   context for recording diagnostics
    /// * `field` - The syn field whose attributes are parsed
    /// * `name` - The field ident, used as a span anchor for error messages
    ///
    /// # Returns
    ///
    /// `Some(Field)` if the field carried at least one `#[klv(..)]` attribute, each parsed, and it
    /// is the `unknown` field or has one key, or `None` otherwise
    pub fn from_ast(fill: &CodecFill, field: &syn::Field, name: &syn::Ident) -> Option<Self> {
        let cx = fill.cx;
        // --------------------------------------------------
        // return if no attrs on field
        // --------------------------------------------------
        if field.attrs.is_empty() {
            return None;
        }
        // --------------------------------------------------
        // loop through attrs
        // --------------------------------------------------
        let mut parsed = Vec::new();
        for attr in &field.attrs {
            if attr.path() != symbol::KLV_ATTR {
                continue;
            }
            // --------------------------------------------------
            // parse out klv attributes
            // --------------------------------------------------
            match attr.meta {
                syn::Meta::List(ref contents) => {
                    parsed.push(FieldXcoder::try_from(contents));
                }
                _ => {
                    cx.error_spanned_by(attr, err!(MalformedField));
                }
            }
        }
        // --------------------------------------------------
        // return if no attrs == symbol::KLV_ATTR
        // --------------------------------------------------
        if parsed.is_empty() {
            return None;
        }
        // --------------------------------------------------
        // add all `syn_error`'s to `cx`
        // --------------------------------------------------
        let count = parsed.len();
        let mut lists: Vec<FieldXcoder> = parsed
            .into_iter()
            .filter_map(|f| f.map_err(|e| cx.syn_error(e)).ok())
            .collect();
        // --------------------------------------------------
        // a list that failed to parse stopped at its error, so the settings it holds are
        // incomplete and nothing more is checked on the field
        // --------------------------------------------------
        if lists.len() < count {
            return None;
        }
        // --------------------------------------------------
        // the field that keeps the unknown frames is not keyed: the container's `unknown(..)` is
        // its codec, so it takes no other setting
        // --------------------------------------------------
        let unknown = Unique::of(lists.iter().filter_map(|f| f.unknown), |&again| {
            cx.syn_error(syn::Error::new(again, err!(DuplicateUnknownInField)));
        });
        if let Some(at) = unknown.first() {
            if let Some(setting) = lists.iter().find_map(|f| f.keyed_setting) {
                cx.syn_error(syn::Error::new(setting, err!(UnknownFieldWithSetting)));
            }
            return Some(Field::Unknown(at));
        }
        // --------------------------------------------------
        // get all klv attr keys, a `oneof(..)` counted as one
        // --------------------------------------------------
        let keys = lists.iter_mut().filter_map(|f| f.key.take());
        let key = match Unique::of(keys, |_| {
            cx.error_spanned_by(field, err!(DuplicateKeyInField));
        }) {
            Unique::Unset => {
                cx.error_spanned_by(name, err!(MissingKeyInField));
                None
            }
            Unique::One(key) => Some(key),
            Unique::Duplicate(_) => None,
        };
        // --------------------------------------------------
        // get all klv attr encoders
        // --------------------------------------------------
        let encoders = lists.iter_mut().filter_map(|f| f.codec.enc.take());
        let enc = Unique::of(encoders, |_| {
            cx.error_spanned_by(field, err!(DuplicateEncoderInField));
        })
        .first();
        // --------------------------------------------------
        // get all klv attr decoders
        // --------------------------------------------------
        let decoders = lists.iter_mut().filter_map(|f| f.codec.dec.take());
        let dec = Unique::of(decoders, |_| {
            cx.error_spanned_by(field, err!(DuplicateDecoderInField));
        })
        .first();
        // --------------------------------------------------
        // get all klv attr size
        // --------------------------------------------------
        let sizes = lists.iter_mut().filter_map(|f| f.codec.size.take());
        let size = Unique::of(sizes, |_| {
            cx.error_spanned_by(field, err!(DuplicateSizeInField));
        })
        .once();
        // --------------------------------------------------
        // get all klv attr latebind
        // --------------------------------------------------
        let latebinds = lists.iter_mut().filter_map(|f| f.latebind.take());
        let latebind = Unique::of(latebinds, |_| {
            cx.error_spanned_by(field, err!(DuplicateLatebindInField));
        })
        .first();
        // --------------------------------------------------
        // get all klv attr default's
        // --------------------------------------------------
        let defaults = lists.iter_mut().filter_map(|f| f.default.take());
        let mut default = Unique::of(defaults, |_| {
            cx.error_spanned_by(field, err!(DuplicateDefaultInField));
        })
        .first();
        // --------------------------------------------------
        // get all klv attr break_on's
        // --------------------------------------------------
        let break_ons = lists.iter_mut().filter_map(|f| f.break_on.take());
        let break_on = Unique::of(break_ons, |_| {
            cx.error_spanned_by(field, err!(DuplicateBreakOnInField));
        })
        .first();
        // --------------------------------------------------
        // get all klv attr skips
        // --------------------------------------------------
        let skips = lists.iter_mut().filter_map(|f| f.skip_if.take());
        let skip_if = Unique::of(skips, |_| {
            cx.error_spanned_by(field, err!(SkipIfConflict));
        })
        .first();
        // --------------------------------------------------
        // a container default of the field's type fills the skip the field left unset
        // --------------------------------------------------
        let typ_maybe_unwrapped = unwrap_option_type(&field.ty).unwrap_or(&field.ty);
        let skip_if = skip_if.or_else(|| {
            fill.defaults
                .get(&ungrouped(typ_maybe_unwrapped))
                .and_then(|default| default.skip_if.clone())
        });
        // --------------------------------------------------
        // a skipped field's missing frame decodes to its type's default, unless the field sets
        // its own; an `Option` field's already decodes to `None`
        // --------------------------------------------------
        if skip_if.is_some() && default.is_none() && !is_option(&field.ty) {
            default = Some(DefaultValue::Call);
        }
        // --------------------------------------------------
        // a field without exactly one key, already reported, is checked no further: which
        // frames its value codec would read is unknown
        // --------------------------------------------------
        let key = key?;
        // --------------------------------------------------
        // each value codec is filled from its type's container default, else its trait impls
        // --------------------------------------------------
        let keys = match key {
            // --------------------------------------------------
            // a `oneof(..)` arm's value takes its own type's codec, so an `enc`, `dec` or `size`
            // on the field is an error
            // --------------------------------------------------
            KeySpec::Oneof(oneof) => {
                lists
                    .iter()
                    .flat_map(|f| &f.codec_settings)
                    .for_each(|&at| cx.syn_error(syn::Error::new(at, err!(OneofFieldWithCodec))));
                let arms = oneof
                    .arms
                    .into_iter()
                    .map(|arm| OneofKey {
                        codec: fill.codec(
                            name,
                            WrittenCodec::default(),
                            &arm.ty,
                            Some(&arm.variant),
                        ),
                        arm,
                    })
                    .collect();
                FieldKeys::Oneof(arms)
            }
            // --------------------------------------------------
            // the field's own value codec, filled from its type
            // --------------------------------------------------
            KeySpec::Literal(key) => {
                let own = WrittenCodec { enc, dec, size };
                FieldKeys::One {
                    key,
                    codec: Box::new(fill.codec(name, own, typ_maybe_unwrapped, None)),
                }
            }
        };
        // --------------------------------------------------
        // return
        // --------------------------------------------------
        Some(Field::Keyed(Box::new(FieldParsed {
            keys,
            latebind,
            default,
            break_on,
            skip_if,
        })))
    }
}

/// What fills a value codec where the field leaves it unset: the container `default(typ = ..)`
/// of the value's type, else, under `trait_fallback`, that type's trait impls
pub(super) struct CodecFill<'a> {
    /// Error accumulation context for recording diagnostics
    pub(super) cx: &'a Ctxt,

    /// Per-type default xcoders from the container annotation
    defaults: &'a HashMap<syn::Type, DefaultXcoder>,

    /// Whether missing encoders and decoders produce no errors
    allow: AllowUnimplemented,

    /// When `true`, choose the trait impls for missing xcoders
    trait_fallback: bool,
}
/// [`CodecFill`] implementation
impl<'a> CodecFill<'a> {
    /// The fill of a container's fields
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording diagnostics
    /// * `container` - The container attributes: its `default(..)` xcoders and flags
    ///
    /// # Returns
    ///
    /// The fill
    pub(super) fn of(cx: &'a Ctxt, container: &'a Container) -> Self {
        Self {
            cx,
            defaults: &container.defaults,
            allow: container.allow_unimplemented(),
            trait_fallback: container.trait_fallback.is_some(),
        }
    }

    /// Fills the codec of a value of type `ty`
    ///
    /// The container default of `ty` fills each of enc, dec, `implicit` and the size's byte count
    /// `own` left unset; `lengthed` comes with whichever dec is used. A missing encoder or
    /// decoder is reported at the field's name, or at an arm's type, unless
    /// `allow_unimplemented_*` or `trait_fallback` covers it, or an error on that side is already
    /// reported
    ///
    /// # Arguments
    ///
    /// * `name` - The field ident, which an error names
    /// * `own` - The codec the field wrote
    /// * `ty` - The value's type
    /// * `arm` - The variant of a `oneof(..)` arm's value, `None` for the field's own
    ///
    /// # Returns
    ///
    /// The filled codec
    fn codec(
        &self,
        name: &syn::Ident,
        own: WrittenCodec,
        ty: &syn::Type,
        arm: Option<&syn::Path>,
    ) -> ValueCodec {
        let Self {
            cx,
            defaults,
            allow,
            trait_fallback,
        } = *self;
        let at = match arm {
            Some(_) => ty.to_token_stream(),
            None => name.to_token_stream(),
        };
        // --------------------------------------------------
        // a container default fills each of enc, dec, `implicit` and the size's byte count the
        // field left unset; `lengthed` comes with whichever dec the field uses
        // --------------------------------------------------
        let WrittenCodec { enc, dec, size } = own;
        let default = defaults.get(&ungrouped(ty));
        let default_dec = dec.is_none() && default.is_some_and(|default| default.dec.is_some());
        let mut enc = enc
            .or_else(|| default.and_then(|default| default.enc.clone()))
            .map(ValueXcoder::Explicit);
        let mut dec = dec
            .or_else(|| default.and_then(|default| default.dec.clone()))
            .map(ValueXcoder::Explicit);
        // --------------------------------------------------
        // the size is filled, and checked, with or without a default
        // --------------------------------------------------
        let default_size = default.and_then(|default| default.size.as_ref());
        let size = match SizeSpec::filled(size, default_size, default_dec) {
            Ok(size) => size,
            Err(err) => {
                cx.syn_error(err);
                None
            }
        };
        // --------------------------------------------------
        // a side whose error is already reported, one on the type's container default, gets no
        // missing-codec error
        // --------------------------------------------------
        let default_reported = default.is_some_and(|default| default.reported);
        let enc_reported = default_reported;
        let mut dec_reported = default_reported;
        // --------------------------------------------------
        // fallback to trait impls (opt-in via `trait_fallback`)
        //
        // only engages when the user set the container-level flag AND no
        // explicit xcoder / container default filled the slot. `allow_unimplemented_*`
        // takes precedence - if either is set, fallback is skipped on that side
        // --------------------------------------------------
        if trait_fallback && !allow.encode && enc.is_none() {
            enc = Some(ValueXcoder::TraitFallback);
        }
        if trait_fallback && !allow.decode && dec.is_none() {
            let takes_len = size.as_ref().is_some_and(SizeSpec::takes_len);
            if takes_len {
                cx.error_spanned_by(&at, err!(SizeLengthedFallbackRequiresExplicitDec(name, ty)));
                dec_reported = true;
            } else {
                dec = Some(ValueXcoder::TraitFallback);
            }
        }
        // --------------------------------------------------
        // unimplemented encode error - only fires if fallback did not fill and no error on the
        // encoder is reported
        // --------------------------------------------------
        if !allow.encode && !enc_reported && enc.is_none() {
            let unimplemented = match arm {
                Some(variant) => err!(UnimplementedArmEncode(name, ty, variant)),
                None => err!(UnimplementedEncode(name, ty)),
            };
            cx.error_spanned_by(&at, unimplemented);
        }
        // --------------------------------------------------
        // unimplemented decode error - only fires if fallback did not fill and no error on the
        // decoder is reported
        // --------------------------------------------------
        if !allow.decode && !dec_reported && dec.is_none() {
            let unimplemented = match arm {
                Some(variant) => err!(UnimplementedArmDecode(name, ty, variant)),
                None => err!(UnimplementedDecode(name, ty)),
            };
            cx.error_spanned_by(&at, unimplemented);
        }
        ValueCodec { enc, dec, size }
    }
}

#[derive(Debug)]
/// Validated form of a single field's KLV attributes, with a guaranteed key
///
/// Produced by [`Field::from_ast`] for a field with one key or one `key = oneof(..)`. Each
/// [`ValueCodec`]'s encoder and decoder is a written path, or the value type's trait impl, which
/// expansion calls as a fully-qualified trait method ([`ValueXcoder`])
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
#[cfg(test)]
mod tests {
    use crate::ast::attr::{MainContainer, ValueXcoder, size::SizeSpec};

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
        let container = MainContainer::parsed(&item).expect("the input parses");
        let filled: Vec<String> = container
            .klv_keys()
            .map(|key| {
                let size = key.codec.size.as_ref();
                let Some(ValueXcoder::Explicit(dec)) = &key.codec.dec else {
                    panic!("{}: a written dec", key.field.name);
                };
                format!(
                    "{}: dec = {}, lengthed = {}, exact = {:?}",
                    key.field.name,
                    dec.to_token_stream(),
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
