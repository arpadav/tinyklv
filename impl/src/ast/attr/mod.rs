// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod container;
mod field;

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

/// A source data structure annotated with `#[derive(Klv)]` parsed into an internal representation.
pub(crate) struct MainContainer<'a> {
    /// The struct or enum name (without generics).
    pub ident: syn::Ident,
    /// Attributes on the structure, parsed for `tinyklv`.
    pub attrs: container::ContainerParsed,
    /// The contents of the struct
    pub data: Vec<MainField<'a>>,
    /// Any generics on the struct
    pub generics: &'a syn::Generics,
    /// Original input.
    pub _original: &'a syn::DeriveInput,
}

/// A field of a struct.
pub(crate) struct MainField<'a> {
    pub name: syn::Ident,
    pub attrs: Option<field::FieldParsed>,
    pub ty: &'a syn::Type,
    pub _original: &'a syn::Field,
}

impl<'a> MainContainer<'a> {
    /// Convert the raw [`syn`] ast into a parsed container object, collecting errors in `cx`.
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
