//! The container length-prefix shape and the fixed-prefix field length kinds
//!
//! This module emits no tokens. It classifies the container length prefix and
//! names the ways a field's value length is written under a fixed-width prefix,
//! so emission modules can choose a concrete write shape
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::size::SizeSpec;

/// Whether the container's length prefix is fixed-width or variable-width
#[derive(Debug, Clone, Copy)]
pub(super) enum ContainerLengthPrefix {
    /// A fixed `width`-byte length slot declared by `len(size(exact = N))`
    Fixed {
        /// The fixed byte width of the reserved length slot
        width: usize,
    },
    /// A variable-width length from `len(size(var))`, omitted size, hint-only size, or BER
    Variable,
}

/// [`ContainerLengthPrefix`] implementation
impl ContainerLengthPrefix {
    /// Resolves the length-prefix shape from parsed `len(size(..))`
    ///
    /// Converts exact size declarations into fixed-width length slots and
    /// treats all other declarations as variable-width. Hint-only sizes stay
    /// variable because they cannot reserve an exact number of bytes for a
    /// later back-fill operation
    ///
    /// # Arguments
    ///
    /// * `size` - Optional parsed length-size declaration from container attrs
    ///
    /// # Returns
    ///
    /// `ContainerLengthPrefix::Fixed` when an exact byte width is declared, or
    /// `ContainerLengthPrefix::Variable` for every non-exact length form
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let prefix = ContainerLengthPrefix::from_size(size);
    /// assert!(matches!(prefix, ContainerLengthPrefix::Fixed { .. } | ContainerLengthPrefix::Variable));
    /// ```
    pub(super) fn from_size(size: Option<SizeSpec>) -> Self {
        // --------------------------------------------------
        // exact sizes become fixed-width length prefixes
        // --------------------------------------------------
        match size.and_then(SizeSpec::exact_width) {
            Some(width) => Self::Fixed { width },
            None => Self::Variable,
        }
    }
}

/// A field value width known at compile time
#[derive(Debug, Clone, Copy)]
pub(super) struct ValueWidth {
    /// Exact encoded byte width of the field value
    pub(super) bytes: usize,

    /// The span that declared the width: the field's `size(..)`, else its value encoder
    pub(super) declared_at: proc_macro2::Span,
}

/// How one field's value length is written under a fixed-width length prefix
pub(super) enum FieldLengthPlan {
    /// Value byte width is known at compile time
    KnownValueWidth(ValueWidth),
    /// Value width is unknown, so the container's fixed length slot is reserved and back-filled
    ReservedLengthSlot,
}
