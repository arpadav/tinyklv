//! Length-prefix decisions for encoded fields and frames
//!
//! This module emits no tokens. It classifies the container length prefix and
//! each field's value length so emission modules can choose a concrete write
//! shape without duplicating width policy
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::fixed_width;
use crate::{
    ast::{attr::size::SizeSpec, types},
    expand::helpers,
};

/// Whether the container's length prefix is fixed-width or variable-width
#[derive(Clone, Copy)]
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

    /// Returns `true` when values must be staged before writing the length
    ///
    /// Variable container lengths cannot reserve an exact slot, so generated
    /// encoders must know the value length before emitting the prefix. Fixed
    /// prefixes can be back-filled and therefore return `false`
    ///
    /// # Returns
    ///
    /// `true` for variable-width prefixes and `false` for fixed-width prefixes
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// assert!(ContainerLengthPrefix::Variable.is_variable());
    /// ```
    pub(super) fn is_variable(self) -> bool {
        // --------------------------------------------------
        // classify variable-width container length prefixes
        // --------------------------------------------------
        matches!(self, Self::Variable)
    }
}

/// How one field's value length is written
pub(super) enum FieldLengthPlan {
    /// Value byte width is known at compile time
    KnownValueWidth {
        /// Exact encoded byte width of the field value
        value_width: usize,
    },
    /// Value width is unknown, but the container length slot has fixed width
    ReservedLengthSlot {
        /// Exact byte width reserved for the field length slot
        len_width: usize,
    },
    /// Container length is variable-width, so stage the value first
    StagedValue,
}

/// Chooses the field length plan from field metadata and container prefix
///
/// Prefers plans that can write a field length immediately when an exact field
/// size is declared or a primitive type and built-in encoder agree on width. If
/// the container has a fixed length prefix but field width is unknown, the plan
/// reserves that fixed slot and back-fills it after encoding. Variable
/// container lengths stage the value first because no fixed slot exists
///
/// # Arguments
///
/// * `fallback_enc` - Whether the field uses the fallback `EncodeValue` encoder
/// * `value_encoder` - Parsed value encoder used to infer fixed encoder width
/// * `ty` - Parsed field type used to infer fixed primitive width
/// * `len_prefix` - Container length-prefix strategy shared by field items
/// * `size` - Optional exact or hinted field value-size declaration
///
/// # Returns
///
/// The concrete field length plan that downstream generators should emit for
/// the field item write
///
/// # Example
///
/// ```rust,ignore
/// let plan = field_length_plan(false, &value_encoder, &ty, len_prefix, size);
/// assert!(matches!(plan, FieldLengthPlan::KnownValueWidth { .. } | FieldLengthPlan::ReservedLengthSlot { .. } | FieldLengthPlan::StagedValue));
/// ```
pub(super) fn field_length_plan(
    fallback_enc: bool,
    value_encoder: &types::SiguledXcoder,
    ty: &syn::Type,
    len_prefix: ContainerLengthPrefix,
    size: Option<SizeSpec>,
) -> FieldLengthPlan {
    // --------------------------------------------------
    // variable container lengths require staged field values
    // --------------------------------------------------
    let ContainerLengthPrefix::Fixed { width: len_width } = len_prefix else {
        return FieldLengthPlan::StagedValue;
    };
    // --------------------------------------------------
    // explicit exact field sizes can be written immediately
    // --------------------------------------------------
    if let Some(value_width) = size.and_then(SizeSpec::exact_width) {
        return FieldLengthPlan::KnownValueWidth { value_width };
    }
    // --------------------------------------------------
    // prove fixed width from primitive type and built-in encoder
    // --------------------------------------------------
    if !fallback_enc {
        let inner = helpers::unwrap_option_type(ty).unwrap_or(ty);
        if let (Some(type_width), Some(encoder_width)) = (
            fixed_width::fixed_value_type_width(inner),
            fixed_width::builtin_fixed_encoder_width(value_encoder),
        ) && type_width == encoder_width
        {
            return FieldLengthPlan::KnownValueWidth {
                value_width: type_width,
            };
        }
    }
    // --------------------------------------------------
    // otherwise reserve the container's fixed length slot
    // --------------------------------------------------
    FieldLengthPlan::ReservedLengthSlot { len_width }
}
