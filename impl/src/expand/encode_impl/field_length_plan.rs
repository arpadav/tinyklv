//! The field length kinds
//!
//! It names the ways a field's value length is written under a fixed-width length prefix or a
//! `keylen` header, so emission modules can choose a concrete write shape, writes a value of known
//! width, and checks in debug builds that a value is its known width
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::{Span, TokenStream};
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
        prefix_write: TokenStream,
        value_write: impl FnOnce(&TokenStream) -> TokenStream,
        span: Span,
    ) -> TokenStream {
        let value_write = value_write(&quote_spanned! { span => out });
        let check = self.check(&quote_spanned! { span => out.len() - __value_start }, span);
        quote_spanned! { span =>
            {
                #prefix_write
                let __value_start = out.len();
                #value_write
                #check
            }
        }
    }

    /// Checks in debug builds that the value encoder wrote [`Self::bytes`] bytes
    ///
    /// # Arguments
    ///
    /// * `written` - The byte count the value encoder wrote
    /// * `span` - Span of the emitted check
    ///
    /// # Returns
    ///
    /// The `debug_assert_eq!`, its message naming [`Self::field`]
    pub(super) fn check(self, written: &TokenStream, span: Span) -> TokenStream {
        let Self { field, bytes, .. } = self;
        let message = format!(
            "tinyklv: the value encoder of `{field}` wrote a different byte count than its fixed width"
        );
        quote_spanned! { span =>
            ::core::debug_assert_eq!(#written, #bytes, #message);
        }
    }
}

/// How one field's value length is written under a fixed-width length prefix or a `keylen` header
#[derive(Debug, Clone, Copy)]
pub(super) enum FieldLengthPlan<'a> {
    /// Value byte width is known at compile time, so the length is written before the value
    KnownValueWidth(ValueWidth<'a>),
    /// Value width is unknown until the value is written: the container's fixed length slot is
    /// reserved and back-filled, a `keylen` header is written after the value and rotated in
    /// front of it
    UnknownValueWidth,
}
