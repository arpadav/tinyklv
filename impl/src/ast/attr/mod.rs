//! Top-level parsed AST types for a `#[derive(Klv)]`-annotated struct
//!
//! Defines [`MainContainer`] (the fully parsed and validated view of an
//! annotated struct) and [`MainField`] (a single named field within that
//! struct). These types are the bridge between the attribute-parsing pass
//! (`container` and `field` sub-modules) and the code-generation pass
//! (`expand`)
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod container;
mod field;
pub(crate) mod size;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::Ctxt;
use container::{ContainerParsed, default::DefaultXcoder};

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
/// parsed fields, and references back to the original [`syn`] generics and
/// derive input. The `expand` pass consumes this type to emit the final
/// token stream
pub(crate) struct MainContainer<'a> {
    /// The struct name without generic parameters
    pub ident: syn::Ident,

    /// Validated container-level `#[klv(..)]` attributes, including key/len
    /// xcoders and all optional settings
    pub attrs: container::ContainerParsed,

    /// Parsed representation of every field in the struct, in declaration order
    pub data: Vec<MainField<'a>>,

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
    pub attrs: Option<field::FieldParsed>,

    /// The declared Rust type of the field
    pub ty: &'a syn::Type,

    /// The original [`syn::Field`], retained for span anchoring
    pub _original: &'a syn::Field,
}

/// [`MainContainer`] implementation
impl<'a> MainContainer<'a> {
    /// Parses a `#[derive(Klv)]` input into a [`MainContainer`]
    ///
    /// Invokes the container attribute parser, determines the struct variant
    /// (named-field structs are supported; tuple structs and unit structs emit
    /// errors), parses all fields, resolves container-level `default(..)` xcoders
    /// and the `trait_fallback` flag, and validates that `key` and `len` xcoders
    /// are present. Returns `None` and records diagnostics in `cx` if any
    /// required attributes are absent or the struct variant is unsupported
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
        let attrs = container::Container::from_ast(cx, item);

        let data = match &item.data {
            syn::Data::Struct(data) => match &data.fields {
                syn::Fields::Named(fields) => Some(fields_from_ast(
                    cx,
                    &fields.named,
                    &attrs.defaults,
                    attrs.allow_unimplemented_encode.is_some(),
                    attrs.allow_unimplemented_decode.is_some(),
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

        Some(MainContainer {
            ident: item.ident.clone(),
            attrs: ContainerParsed::from_cont(cx, &item.ident, attrs)?,
            data,
            generics: &item.generics,
            _original: item,
        })
    }
}

/// Parses a punctuated list of [`syn::Field`]s into [`MainField`] entries
///
/// Filters out unnamed fields (those without an ident), then for each named
/// field invokes [`field::Field::from_ast`] to parse and validate its
/// `#[klv(..)]` attributes, applying container-level `default(..)` xcoders
/// and the `trait_fallback` flag as needed
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
/// A [`Vec<MainField>`] in declaration order, one entry per named field
fn fields_from_ast<'a>(
    cx: &Ctxt,
    fields: &'a Punctuated<syn::Field, Token![,]>,
    container_defaults: &HashMap<syn::Type, DefaultXcoder>,
    aue: bool,
    aud: bool,
    fi: bool,
) -> Vec<MainField<'a>> {
    fields
        .iter()
        .filter_map(|field| field.ident.as_ref().map(|name| (field, name)))
        .map(|(field, name)| MainField {
            name: name.clone(),
            attrs: match field::Field::from_ast(cx, field, name, container_defaults, aue, aud, fi) {
                Some(x) => field::FieldParsed::from_field(cx, field, &x),
                None => None,
            },
            ty: &field.ty,
            _original: field,
        })
        .collect()
}
