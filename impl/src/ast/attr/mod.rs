//! Top-level parsed AST types for a `#[derive(Klv)]`-annotated struct
//!
//! Defines [`MainContainer`] (the fully parsed and validated view of an annotated struct),
//! [`MainField`] (a single named field within that struct), [`FieldKey`] (one key of a keyed
//! field, with the codec of the value its frames carry) and [`UnknownFrames`] (the codec of the
//! frames whose key no field declares, and the [`UnknownField`] that keeps them). These types are
//! the bridge between the attribute-parsing pass (`container` and `field` sub-modules) and the
//! code-generation pass (`expand`)
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod container;
mod field;
pub(crate) mod size;
pub(crate) mod skip_if;
mod unique;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use field::{FieldKeys, FieldParsed, KeySpec, ValueCodec, ValueXcoder};

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::Ctxt;
use container::{AllowUnimplemented, CodecAttr, ContainerMode, ContainerParsed, UnknownXcoder};
use field::{CodecFill, OneofKey};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::ToTokens;
use std::collections::{HashMap, hash_map::Entry};
use syn::{Token, punctuated::Punctuated};

/// Validated representation of a `#[derive(Klv)]`-annotated struct
///
/// Produced by [`MainContainer::from_ast`] after the full attribute-parsing
/// pass. Holds the validated container attributes (key/length xcoders, stream
/// type, sentinel, break condition, defaults, and flags), the ordered list of
/// parsed fields but the `#[klv(unknown)]` one, the frames whose key no field
/// declares (the `unknown(..)` codec and that field), and references back to
/// the original [`syn`] generics and derive input. The `expand` pass consumes
/// this type to emit the final token stream
pub(crate) struct MainContainer<'a> {
    /// The struct name without generic parameters
    pub ident: syn::Ident,

    /// Validated container-level `#[klv(..)]` attributes, including key/len
    /// xcoders and all optional settings but `unknown(..)`, held in [`Self::unknown`]
    pub attrs: container::ContainerParsed,

    /// Parsed representation of every field in the struct but the `#[klv(unknown)]` one, in
    /// declaration order, read through [`Self::keyed_fields`] and [`Self::plain_fields`]
    data: Vec<MainField<'a>>,

    /// The frames whose key no field declares, when the container sets `unknown(..)`
    pub unknown: Option<UnknownFrames<'a>>,

    /// Generic parameters from the original struct definition
    pub generics: &'a syn::Generics,

    /// The original [`syn::DeriveInput`], whose visibility the partial struct takes
    pub _original: &'a syn::DeriveInput,
}

/// Parsed representation of one named field in a `#[derive(Klv)]` struct
///
/// `attrs` is `Some` only when the field carries a `#[klv(..)]` annotation;
/// fields without one are skipped by the encode/decode codegen and filled via
/// [`Default::default`] during struct construction. `ty` is a reference into the original
/// [`syn::DeriveInput`] to avoid cloning
pub(crate) struct MainField<'a> {
    /// The field identifier
    pub name: syn::Ident,

    /// The validated KLV field attributes, or `None` if the field has no
    /// `#[klv(..)]` annotation
    pub attrs: Option<FieldParsed>,

    /// The declared Rust type of the field
    pub ty: &'a syn::Type,
}

/// One key of a keyed field, with the codec of the value its frames carry: the field's own key,
/// or one arm of its `key = oneof(..)`
#[derive(Clone, Copy)]
pub(crate) struct FieldKey<'a> {
    /// The field the key's frames decode into
    pub field: &'a MainField<'a>,

    /// The field's validated KLV attributes
    pub attrs: &'a FieldParsed,

    /// The key literal
    pub key: &'a syn::Lit,

    /// The codec of the value the key's frames carry
    pub codec: &'a ValueCodec,

    /// The type of that value: the field's value type, or the arm's
    pub ty: &'a syn::Type,

    /// The variant of the field's enum an arm's value is wrapped in, `None` for the field's own
    /// key
    pub variant: Option<&'a syn::Path>,
}

/// The frames whose key no field declares: the container's `unknown(..)` codec and the field
/// that keeps them, built once per container
///
/// The codec's decoder reads every such frame but a keylen `Sliced` one no field keeps, whose bytes
/// go with its `take`, and one the container's `break_on` ends or skips first. The codec's encoder
/// re-emits a kept frame, so it is needed only beside [`Self::keep`]
#[derive(Debug)]
pub(crate) struct UnknownFrames<'a> {
    /// The `unknown(dec = .., enc = ..)` codec
    pub codec: UnknownXcoder,

    /// The field each decoded frame is pushed into, or `None` when the frames are dropped
    pub keep: Option<UnknownField<'a>>,
}

/// The `#[klv(unknown)]` field of a `#[derive(Klv)]` struct, a `Vec<(Key, U)>` of the frames
/// whose key no field declares
///
/// It has no key, and the container's `unknown(dec = .., enc = ..)` is its codec, so it is
/// neither one of [`MainContainer::keyed_fields`] nor one of [`MainContainer::plain_fields`]
#[derive(Debug)]
pub(crate) struct UnknownField<'a> {
    /// The field identifier
    pub name: syn::Ident,

    /// The declared Rust type of the field
    pub ty: &'a syn::Type,

    /// The span of the field's `unknown` flag
    at: proc_macro2::Span,
}

/// [`MainContainer`] implementation
impl<'a> MainContainer<'a> {
    /// Parses a `#[derive(Klv)]` input into a [`MainContainer`]
    ///
    /// Invokes the container attribute parser, determines the struct variant
    /// (named-field structs are supported; tuple structs and unit structs emit
    /// errors), parses all fields, resolves container-level `default(..)` xcoders
    /// and the `trait_fallback` flag, and resolves exactly one framing mode, `key` +
    /// `len` or a fused `keylen` ([`ContainerParsed::from_cont`]). A `size(implicit)` field is an
    /// error under `keylen`, whose codec decides each frame's framing. The container's
    /// `unknown(..)` and the `#[klv(unknown)]` field that keeps its frames become one
    /// [`UnknownFrames`]: the field is an error without the codec, and the codec's `enc` is
    /// required only beside the field. A key two fields or arms share is an error. Returns `None`
    /// and records diagnostics in `cx` if any required
    /// attributes are absent or the struct variant is unsupported
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording parse diagnostics
    /// * `item` - The raw derive input from the proc-macro harness
    ///
    /// # Returns
    ///
    /// `Some(MainContainer)` on success, or `None` if any required attribute
    /// is missing or the struct form is not supported (errors in `cx`)
    pub fn from_ast(cx: &Ctxt, item: &'a syn::DeriveInput) -> Option<MainContainer<'a>> {
        // --------------------------------------------------
        // parse the container attributes
        // --------------------------------------------------
        let attrs = container::Container::from_ast(cx, item);
        let allow = attrs.allow_unimplemented();
        // --------------------------------------------------
        // only a named struct carries klv fields; any other form is reported
        // --------------------------------------------------
        let (data, keep) = match &item.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(fields) => {
                    Some(fields_from_ast(&CodecFill::of(cx, &attrs), &fields.named))
                }
                syn::Fields::Unnamed(fields) => {
                    cx.error_spanned_by(fields, err!(UnsupportedUnnamedStructs));
                    None
                }
                syn::Fields::Unit => {
                    cx.error_spanned_by(&data.fields, err!(UnsupportedUnitStructs));
                    None
                }
            },
            syn::Data::Enum(_) => {
                cx.error_spanned_by(item, err!(UnsupportedContainer("enum")));
                return None;
            }
            syn::Data::Union(_) => {
                cx.error_spanned_by(item, err!(UnsupportedContainer("union")));
                return None;
            }
        }?;
        // --------------------------------------------------
        // validate the container attributes
        // --------------------------------------------------
        let (attrs, codec) = ContainerParsed::from_cont(cx, &item.ident, attrs)?;
        // --------------------------------------------------
        // the frames whose key no field declares: `unknown(..)` decodes each one but a keylen
        // `Sliced` one no field keeps and one `break_on` ends or skips first, and its encoder
        // re-emits the ones the `unknown` field keeps; an unusable codec already reported its
        // error
        // --------------------------------------------------
        codec.report_missing(
            cx,
            AllowUnimplemented {
                encode: allow.encode || keep.is_none(),
                ..allow
            },
        );
        let unknown = match (codec, keep) {
            (CodecAttr::Parsed(_, codec), keep) => Some(UnknownFrames {
                codec: *codec,
                keep,
            }),
            (CodecAttr::Absent, Some(field)) => {
                cx.syn_error(syn::Error::new(field.at, err!(UnknownFieldWithoutCodec)));
                None
            }
            (CodecAttr::Absent, None) | (CodecAttr::Invalid(_), _) => None,
        };
        // --------------------------------------------------
        // build the container
        // --------------------------------------------------
        let container = MainContainer {
            ident: item.ident.clone(),
            attrs,
            data,
            unknown,
            generics: &item.generics,
            _original: item,
        };
        container.report_shared_keys(cx);
        // --------------------------------------------------
        // the `keylen` codec decides each frame's framing, so no field is `implicit`, after its
        // default fills it
        // --------------------------------------------------
        if let ContainerMode::Fused { .. } = container.attrs.mode {
            for key in container.klv_keys() {
                if let Some(span) = key.codec.implicit() {
                    cx.syn_error(syn::Error::new(span, err!(ImplicitInKeylen)));
                }
            }
        }
        Some(container)
    }

    /// Reports each key a field or `oneof(..)` arm shares with an earlier one, at the later key:
    /// its frames decode into the earlier one, so the later never decodes
    ///
    /// # Arguments
    ///
    /// * `cx` - Error accumulation context for recording the errors
    fn report_shared_keys(&self, cx: &Ctxt) {
        let mut fields: HashMap<String, &syn::Ident> = HashMap::new();
        for key in self.klv_keys() {
            match fields.entry(key_value(key.key)) {
                Entry::Occupied(first) => {
                    cx.error_spanned_by(key.key, err!(SharedKey(key.key, first.get())));
                }
                Entry::Vacant(slot) => {
                    slot.insert(&key.field.name);
                }
            }
        }
    }

    /// The struct's keyed fields, in declaration order
    ///
    /// # Returns
    ///
    /// Every keyed field, paired with its parsed attributes
    pub(crate) fn keyed_fields(&self) -> impl Iterator<Item = (&MainField<'a>, &FieldParsed)> {
        self.data
            .iter()
            .filter_map(|field| field.attrs.as_ref().map(|attrs| (field, attrs)))
    }

    /// The keys of the struct's keyed fields, in declaration order, a `key = oneof(..)` field's in
    /// the order of its arms
    ///
    /// # Returns
    ///
    /// Every key, paired with its field and the codec of its value
    pub(crate) fn klv_keys(&self) -> impl Iterator<Item = FieldKey<'_>> {
        self.keyed_fields()
            .flat_map(|(field, attrs)| FieldKey::of(field, attrs))
    }

    /// The struct's fields without `#[klv(..)]`, in declaration order
    ///
    /// # Returns
    ///
    /// Every field the codegen skips, which decode fills with its type's `Default`
    pub(crate) fn plain_fields(&self) -> impl Iterator<Item = &MainField<'a>> {
        self.data.iter().filter(|field| field.attrs.is_none())
    }

    /// The `#[klv(unknown)]` field that keeps the [`Self::unknown`] frames
    ///
    /// # Returns
    ///
    /// The field, or `None` when the struct sets no `unknown(..)` or drops its frames
    pub(crate) fn kept(&self) -> Option<&UnknownField<'a>> {
        self.unknown.as_ref()?.keep.as_ref()
    }

    /// Whether the derive reads the length of every key + len frame, which decides where the
    /// length is read
    ///
    /// # Returns
    ///
    /// `false` when a `size(implicit)` field's frame has no length, or when `unknown(..)` is set,
    /// whose decoder reads an unknown key's length itself
    pub(crate) fn reads_every_len(&self) -> bool {
        self.unknown.is_none() && !self.klv_keys().any(|key| key.codec.implicit().is_some())
    }

    /// Whether a decode or encode impl is generated, given the check `has` of each key's value
    /// codec: the struct has something to generate for, a keyed field, the one that keeps
    /// [`Self::unknown`] frames or no field at all, and every key's value codec passes `has`
    ///
    /// Gates each generated half: a decode or encode impl needs a codec on every key, and a
    /// struct whose every field is without `#[klv(..)]` has nothing to generate. A struct with no
    /// field decodes by skipping every frame, and encodes to nothing. The frames' codec is the
    /// container's `unknown(..)`, which each half checks itself
    ///
    /// # Arguments
    ///
    /// * `has` - The per-key check, e.g. that the value carries an encoder
    ///
    /// # Returns
    ///
    /// `true` when a keyed or `#[klv(unknown)]` field exists, or no field does, and `has` holds
    /// for every key
    pub(crate) fn generates_with(&self, mut has: impl FnMut(&ValueCodec) -> bool) -> bool {
        let mut fields = self.keyed_fields().peekable();
        (self.kept().is_some() || fields.peek().is_some() || self.data.is_empty())
            && fields
                .flat_map(|(field, attrs)| FieldKey::of(field, attrs))
                .all(|key| has(key.codec))
    }
}

/// [`MainField`] implementation
impl<'a> MainField<'a> {
    /// The type one value of this field has
    ///
    /// # Returns
    ///
    /// `T` for an `Option<T>` field, else the field's declared type
    pub(crate) fn value_ty(&self) -> &'a syn::Type {
        crate::ast::types::unwrap_option_type(self.ty).unwrap_or(self.ty)
    }
}

/// [`FieldKey`] implementation
impl<'a> FieldKey<'a> {
    /// The keys of a keyed field
    ///
    /// # Arguments
    ///
    /// * `field` - The field
    /// * `attrs` - Its validated KLV attributes
    ///
    /// # Returns
    ///
    /// The field's own key, or each arm of its `key = oneof(..)`
    pub(crate) fn of(
        field: &'a MainField<'a>,
        attrs: &'a FieldParsed,
    ) -> impl Iterator<Item = Self> {
        let (own, arms) = match &attrs.keys {
            FieldKeys::One { key, codec } => (
                Some(Self {
                    field,
                    attrs,
                    key,
                    codec,
                    ty: field.value_ty(),
                    variant: None,
                }),
                [].as_slice(),
            ),
            FieldKeys::Oneof(arms) => (None, arms.as_slice()),
        };
        own.into_iter()
            .chain(arms.iter().map(move |arm| Self::arm(field, attrs, arm)))
    }

    /// The key of one arm of a field's `key = oneof(..)`
    ///
    /// # Arguments
    ///
    /// * `field` - The field
    /// * `attrs` - Its validated KLV attributes
    /// * `arm` - One of its arms, the codec filled
    ///
    /// # Returns
    ///
    /// The arm's key, whose value is the arm's type, wrapped in the arm's variant
    pub(crate) fn arm(
        field: &'a MainField<'a>,
        attrs: &'a FieldParsed,
        OneofKey { arm, codec }: &'a OneofKey,
    ) -> Self {
        Self {
            field,
            attrs,
            key: &arm.key,
            codec,
            ty: &arm.ty,
            variant: Some(&arm.variant),
        }
    }
}

/// The value a key literal is matched as, alike for literals written apart: `0x10`, `16` and
/// `16u8`, or `b'A'` and `65`
///
/// # Arguments
///
/// * `lit` - The key literal
///
/// # Returns
///
/// An integer's or a byte's base-10 digits, else the literal as written
fn key_value(lit: &syn::Lit) -> String {
    match lit {
        syn::Lit::Int(int) => int.base10_digits().to_owned(),
        syn::Lit::Byte(byte) => byte.value().to_string(),
        other => other.to_token_stream().to_string(),
    }
}

/// Parses a punctuated list of [`syn::Field`]s into [`MainField`] entries and the
/// [`UnknownField`]
///
/// Filters out unnamed fields (those without an ident), then for each named
/// field invokes [`field::Field::from_ast`] to parse and validate its
/// `#[klv(..)]` attributes, applying container-level `default(..)` xcoders
/// and the `trait_fallback` flag as needed. A second `#[klv(unknown)]` field is an error
///
/// # Arguments
///
/// * `fill` - The container's `default(..)` xcoders and flags, and the error accumulation
///   context for recording per-field diagnostics
/// * `fields` - The comma-punctuated named field list from the struct definition
///
/// # Returns
///
/// A [`Vec<MainField>`] in declaration order, one entry per named field but the
/// `#[klv(unknown)]` one, and that one, if any
fn fields_from_ast<'a>(
    fill: &CodecFill,
    fields: &'a Punctuated<syn::Field, Token![,]>,
) -> (Vec<MainField<'a>>, Option<UnknownField<'a>>) {
    let mut data = Vec::with_capacity(fields.len());
    let mut unknown = None;
    for (field, name) in fields
        .iter()
        .filter_map(|field| field.ident.as_ref().map(|name| (field, name)))
    {
        let attrs = match field::Field::from_ast(fill, field, name) {
            Some(field::Field::Keyed(attrs)) => Some(*attrs),
            // --------------------------------------------------
            // the field that keeps the unknown frames is held apart from the keyed ones
            // --------------------------------------------------
            Some(field::Field::Unknown(at)) => {
                match unknown {
                    Some(_) => fill
                        .cx
                        .syn_error(syn::Error::new(at, err!(MultipleUnknownFields))),
                    None => {
                        unknown = Some(UnknownField {
                            name: name.clone(),
                            ty: &field.ty,
                            at,
                        });
                    }
                }
                continue;
            }
            None => None,
        };
        data.push(MainField {
            name: name.clone(),
            attrs,
            ty: &field.ty,
        });
    }
    (data, unknown)
}
