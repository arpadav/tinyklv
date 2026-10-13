//! Parsing of a field's `key = oneof(..)`
//!
//! Defines [`KeySpec`], what a field's `key = ..` names: one key literal, or the [`Oneof`] whose
//! keys all route to the one field, each to a variant of the field's enum. [`Oneof`]'s
//! [`TryFrom<&syn::MetaList>`] parses its `K => Enum::Variant(Type)` arms, each a [`OneofArm`]
//! that parses itself, since an arm is no `name = value` meta. [`OneofKey`] is an arm once its
//! codec is filled from its type
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::xcoder::ValueCodec;
use crate::ast::symbol;

// --------------------------------------------------
// external
// --------------------------------------------------
use syn::{Token, punctuated::Punctuated};

#[derive(Debug, Clone)]
/// What a field's `key = ..` names
pub(crate) enum KeySpec {
    /// `key = K`, the one key of the field's frames
    Literal(syn::Lit),
    /// `key = oneof(..)`, one key per variant of the field's enum
    Oneof(Oneof),
}
/// [`KeySpec`] implementation of [`syn::parse::Parse`]
///
/// A `oneof(..)` list is a [`Oneof`]; anything else is parsed as a key literal
impl syn::parse::Parse for KeySpec {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let oneof = input.peek2(syn::token::Paren)
            && input
                .cursor()
                .ident()
                .is_some_and(|(ident, _)| ident == symbol::ONEOF.0);
        if !oneof {
            return input.parse().map(Self::Literal);
        }
        let list: syn::MetaList = input.parse()?;
        Oneof::try_from(&list).map(Self::Oneof)
    }
}

#[derive(Debug, Clone)]
/// The arms of a field's `key = oneof(..)`
pub(crate) struct Oneof {
    /// Each `K => Enum::Variant(Type)`, in the order written
    pub arms: Vec<OneofArm>,
}
/// [`Oneof`] implementation of [`TryFrom`] for [`syn::MetaList`]
///
/// Parses the comma-separated arms of a `oneof(..)` list. A malformed arm is returned as a
/// [`syn::Error`], so the caller can surface it via the [`crate::Ctxt`] accumulator
impl TryFrom<&syn::MetaList> for Oneof {
    type Error = syn::Error;
    fn try_from(input: &syn::MetaList) -> syn::Result<Self> {
        let arms = input.parse_args_with(Punctuated::<OneofArm, Token![,]>::parse_terminated)?;
        Ok(Self {
            arms: arms.into_iter().collect(),
        })
    }
}

#[derive(Debug, Clone)]
/// One arm of a `key = oneof(..)`, `K => Enum::Variant(Type)`: the frames of key `K` carry a
/// `Type`, which the field holds as `Enum::Variant`
pub(crate) struct OneofArm {
    /// The key of the arm's frames
    pub key: syn::Lit,
    /// The tuple variant of the field's enum the value is wrapped in
    pub variant: syn::Path,
    /// The type of the variant's one field, whose codec the arm's frames use
    pub ty: syn::Type,
}
/// [`OneofArm`] implementation of [`syn::parse::Parse`]
impl syn::parse::Parse for OneofArm {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let key = input.parse()?;
        input.parse::<Token![=>]>()?;
        let variant = input.parse()?;
        let ty;
        syn::parenthesized!(ty in input);
        Ok(Self {
            key,
            variant,
            ty: ty.parse()?,
        })
    }
}

#[derive(Debug)]
/// One arm of a `key = oneof(..)`, its codec filled from its type
pub(crate) struct OneofKey {
    /// The arm as written
    pub arm: OneofArm,
    /// The codec of the arm's value
    pub codec: ValueCodec,
}
