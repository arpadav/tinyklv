//! The fixed-prefix field length kinds
//!
//! It names the ways a field's value length is written under a fixed-width length prefix, so
//! emission modules can choose a concrete write shape, and writes a value of known width
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote_spanned;

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

/// [`ValueWidth`] implementation
impl ValueWidth<'_> {
    /// Writes a prefix, then the value, checking in debug builds that it is [`Self::bytes`] long
    ///
    /// # Arguments
    ///
    /// * `prefix_write` - Generated statements that write what precedes the value
    /// * `value_write` - Builds the statements appending the value bytes to the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// A block writing the prefix and the value into `out`
    pub(super) fn write_after(
        self,
        prefix_write: proc_macro2::TokenStream,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let bytes = self.bytes;
        let value_write = value_write(&quote_spanned! { span => out });
        quote_spanned! { span =>
            {
                #prefix_write
                let __value_start = out.len();
                #value_write
                ::core::debug_assert_eq!(
                    out.len() - __value_start,
                    #bytes,
                    "tinyklv: fixed-width value encoder wrote a different byte count than its detected width",
                );
            }
        }
    }
}

/// How one field's value length is written under a fixed-width length prefix
pub(super) enum FieldLengthPlan<'a> {
    /// Value byte width is known at compile time
    KnownValueWidth(ValueWidth<'a>),
    /// Value width is unknown, so the container's fixed length slot is reserved and back-filled
    ReservedLengthSlot,
}
