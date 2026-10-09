//! Top-level parsed AST types for a `#[derive(Klv)]`-annotated struct
//!
//! Defines [`MainContainer`] (the fully parsed and validated view of an annotated struct),
//! [`MainField`] (a single named field within that struct) and [`UnknownFrames`] (the codec of the
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

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub(crate) use field::FieldParsed;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::Ctxt;
use container::{
    BreakOnSpec, CodecAttr, ContainerMode, ContainerParsed, DefaultXcoder, UnknownXcoder,
};
use size::SizeSpec;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::collections::HashMap;
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
    /// declaration order, read through [`Self::klv_fields`] and [`Self::plain_fields`]
    data: Vec<MainField<'a>>,

    /// The frames whose key no field declares, when the container sets `unknown(..)`
    pub unknown: Option<UnknownFrames<'a>>,

    /// Generic parameters from the original struct definition
    pub generics: &'a syn::Generics,

    /// The original [`syn::DeriveInput`], retained for span anchoring and
    /// visibility extraction
    pub _original: &'a syn::DeriveInput,
}

/// Parsed representation of one named field in a `#[derive(Klv)]` struct
///
/// `attrs` is `Some` only when the field carries a `#[klv(..)]` annotation;
/// fields without one are skipped by the encode/decode codegen and filled via
/// [`Default::default`] during struct construction. `ty` and `_original` are
/// references into the original [`syn::DeriveInput`] to avoid cloning
pub(crate) struct MainField<'a> {
    /// The field identifier
    pub name: syn::Ident,

    /// The validated KLV field attributes, or `None` if the field has no
    /// `#[klv(..)]` annotation
    pub attrs: Option<FieldParsed>,

    /// The declared Rust type of the field
    pub ty: &'a syn::Type,

    /// The original [`syn::Field`], retained for span anchoring
    pub _original: &'a syn::Field,
}

/// The frames whose key no field declares: the container's `unknown(..)` codec and the field
/// that keeps them, built once per container
///
/// The codec's decoder reads every such frame but two: a keylen `Sliced` one no field keeps, whose
/// bytes go with its `take`, and the container's `break_on` key's, which ends the loop first. The
/// codec's encoder re-emits a kept frame, so it is needed only beside [`Self::keep`]
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
/// neither one of [`MainContainer::klv_fields`] nor one of [`MainContainer::plain_fields`]
#[derive(Debug)]
pub(crate) struct UnknownField<'a> {
    /// The field identifier
    pub name: syn::Ident,

    /// The declared Rust type of the field
    pub ty: &'a syn::Type,

    /// The span of the field's `unknown` flag
    pub at: proc_macro2::Span,
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
    /// required only beside the field. A key + len `break_on = <fn>` is an error unless the
    /// derive reads every frame's length ([`Self::reads_every_len`]). Returns `None` and records
    /// diagnostics in `cx` if any required attributes are absent or the struct variant is
    /// unsupported
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
        let allow_encode = attrs.allow_unimplemented_encode.is_some();
        let allow_decode = attrs.allow_unimplemented_decode.is_some();
        // --------------------------------------------------
        // only a named struct carries klv fields; any other form is reported
        // --------------------------------------------------
        let (data, keep) = match &item.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(fields) => Some(fields_from_ast(
                    cx,
                    &fields.named,
                    &attrs.defaults,
                    allow_encode,
                    allow_decode,
                    attrs.trait_fallback.is_some(),
                )),
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
        // `Sliced` one no field keeps and the `break_on` key's, which ends the loop first, and
        // its encoder re-emits the ones the `unknown` field keeps; an unusable codec already
        // reported its error
        // --------------------------------------------------
        codec.report_missing(cx, allow_encode || keep.is_none(), allow_decode);
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
        match &container.attrs.mode {
            // --------------------------------------------------
            // the `keylen` codec decides each frame's framing, so no field is `implicit`, after
            // its default fills it
            // --------------------------------------------------
            ContainerMode::Fused { .. } => {
                for (_, attrs) in container.klv_fields() {
                    if let Some(span) = attrs.size.as_ref().and_then(SizeSpec::implicit) {
                        cx.syn_error(syn::Error::new(span, err!(ImplicitInKeylen)));
                    }
                }
            }
            // --------------------------------------------------
            // a `break_on` fn is passed each frame's length, which the derive must read
            // --------------------------------------------------
            ContainerMode::Separate {
                break_on: Some(BreakOnSpec::Func(path)),
                ..
            } if !container.reads_every_len() => {
                cx.error_spanned_by(path, err!(BreakOnFnWithoutLength));
            }
            ContainerMode::Separate { .. } => (),
        }
        Some(container)
    }

    /// The struct's keyed fields, in declaration order
    ///
    /// # Returns
    ///
    /// Every keyed field, paired with its parsed attributes
    pub(crate) fn klv_fields(&self) -> impl Iterator<Item = (&MainField<'a>, &FieldParsed)> {
        self.data
            .iter()
            .filter_map(|field| field.attrs.as_ref().map(|attrs| (field, attrs)))
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

    /// Whether the derive reads the length of every key + len frame, which a `break_on = <fn>`
    /// is passed and which decides where the length is read
    ///
    /// # Returns
    ///
    /// `false` when a `size(implicit)` field's frame has no length, or when `unknown(..)` is set,
    /// whose decoder reads an unknown key's length itself
    pub(crate) fn reads_every_len(&self) -> bool {
        self.unknown.is_none()
            && !self
                .klv_fields()
                .any(|(_, attrs)| attrs.size.as_ref().and_then(SizeSpec::implicit).is_some())
    }

    /// Whether the struct has a field to generate for, keyed or the one that keeps
    /// [`Self::unknown`] frames, and every keyed one passes `has`
    ///
    /// Gates each generated half: a decode or encode impl needs a codec on every keyed field, and
    /// a struct without a field has nothing to generate. The frames' codec is the container's
    /// `unknown(..)`, which each half checks itself
    ///
    /// # Arguments
    ///
    /// * `has` - The per-field check, e.g. that the field carries an encoder
    ///
    /// # Returns
    ///
    /// `true` when a keyed or `#[klv(unknown)]` field exists and `has` holds for every keyed one
    pub(crate) fn every_keyed_field(&self, has: impl FnMut(&FieldParsed) -> bool) -> bool {
        let mut attrs = self.klv_fields().map(|(_, attrs)| attrs).peekable();
        (self.kept().is_some() || attrs.peek().is_some()) && attrs.all(has)
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
/// * `cx` - Error accumulation context for recording per-field diagnostics
/// * `fields` - The comma-punctuated named field list from the struct definition
/// * `container_defaults` - Per-type default xcoders from the container annotation
/// * `aue` - Whether missing encoders are allowed (`allow_unimplemented_encode`)
/// * `aud` - Whether missing decoders are allowed (`allow_unimplemented_decode`)
/// * `fi` - Whether to inject trait-impl fallback placeholders (`trait_fallback`)
///
/// # Returns
///
/// A [`Vec<MainField>`] in declaration order, one entry per named field but the
/// `#[klv(unknown)]` one, and that one, if any
fn fields_from_ast<'a>(
    cx: &Ctxt,
    fields: &'a Punctuated<syn::Field, Token![,]>,
    container_defaults: &HashMap<syn::Type, DefaultXcoder>,
    aue: bool,
    aud: bool,
    fi: bool,
) -> (Vec<MainField<'a>>, Option<UnknownField<'a>>) {
    let mut data = Vec::with_capacity(fields.len());
    let mut unknown = None;
    for (field, name) in fields
        .iter()
        .filter_map(|field| field.ident.as_ref().map(|name| (field, name)))
    {
        let attrs = match field::Field::from_ast(cx, field, name, container_defaults, aue, aud, fi)
        {
            Some(field::Field::Keyed(x)) => FieldParsed::from_field(cx, field, &x),
            // --------------------------------------------------
            // the field that keeps the unknown frames is held apart from the keyed ones
            // --------------------------------------------------
            Some(field::Field::Unknown(at)) => {
                match unknown {
                    Some(_) => cx.syn_error(syn::Error::new(at, err!(MultipleUnknownFields))),
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
            _original: field,
        });
    }
    (data, unknown)
}
