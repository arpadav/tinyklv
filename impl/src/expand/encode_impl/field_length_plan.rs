//! The fixed-prefix field length kinds
//!
//! This module emits no tokens. It names the ways a field's value length is written under a
//! fixed-width length prefix, so emission modules can choose a concrete write shape
//!
//! Author: aav

/// A field value width known at compile time
#[derive(Debug, Clone, Copy)]
pub(super) struct ValueWidth<'a> {
    /// The field whose value has this width
    pub(super) field: &'a syn::Ident,

    /// Exact encoded byte width of the field value
    pub(super) bytes: usize,

    /// The span that declared the width: the field's `size(..)`, else its value encoder
    pub(super) declared_at: proc_macro2::Span,
}

/// How one field's value length is written under a fixed-width length prefix
pub(super) enum FieldLengthPlan<'a> {
    /// Value byte width is known at compile time
    KnownValueWidth(ValueWidth<'a>),
    /// Value width is unknown, so the container's fixed length slot is reserved and back-filled
    ReservedLengthSlot,
}
