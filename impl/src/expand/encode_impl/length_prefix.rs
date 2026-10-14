//! The container's length prefix: a fixed-width slot, or a variable-width length written after the
//! value is staged
//!
//! [`LengthPrefix`](crate::expand::encode_impl::length_prefix::LengthPrefix) is the `len(..)`
//! encoder with the width its `size(..)` declares, for each field and for the sentinel frame alike
//!
//! [`FixedLengthSlot`](crate::expand::encode_impl::length_prefix::FixedLengthSlot) owns
//! everything about a `len(size(exact = W))` slot: the largest length its `W` bytes can encode, the
//! write for a value of known width, whose length is written directly, the write for a value of
//! expected width, whose length is written directly and back-filled when the value takes another,
//! and the complete write for a KLV unit whose body length is unknown until after the body is
//! encoded: write the prefix, reserve the slot, write the value, then back-fill the slot, the
//! length checked against `W` in debug builds, or roll back the whole unit if the value is too
//! large
//!
//! [`VariableLength`](crate::expand::encode_impl::length_prefix::VariableLength) writes the
//! length of a staged value
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{frame_body::ValueWidth, staging::Staging};
use crate::ast::{attr::size::SizeBytes, types};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{ToTokens, quote, quote_spanned};

/// The container's length prefix: the `len(..)` encoder and the width its `size(..)` declares
#[derive(Debug, Clone, Copy)]
pub(super) enum LengthPrefix<'a> {
    /// A fixed-width slot declared by `len(size(exact = N))`
    Fixed(FixedLengthSlot<'a>),
    /// A variable-width length from an omitted, empty or reserve-only `len(size(..))`
    Variable(VariableLength<'a>),
}

/// [`LengthPrefix`] implementation
impl<'a> LengthPrefix<'a> {
    /// Resolves the length prefix from the `len(..)` encoder and its `size(..)`
    ///
    /// Converts exact size declarations into fixed-width length slots and
    /// treats all other declarations as variable-width. Reserve-only sizes stay
    /// variable because they cannot reserve an exact number of bytes for a
    /// later back-fill operation
    ///
    /// # Arguments
    ///
    /// * `len_encoder` - Encoder expression writing the length
    /// * `size` - The parsed length-size declaration from container attrs
    ///
    /// # Returns
    ///
    /// `LengthPrefix::Fixed` when an exact byte width is declared, or
    /// `LengthPrefix::Variable` for every non-exact length form
    pub(super) fn new(len_encoder: &'a types::XcoderType, size: &SizeBytes) -> Self {
        // --------------------------------------------------
        // exact sizes become fixed-width length slots
        // --------------------------------------------------
        match size.exact_width() {
            Some(width) => Self::Fixed(FixedLengthSlot { len_encoder, width }),
            None => Self::Variable(VariableLength(len_encoder)),
        }
    }

    /// A length write that never runs, for a struct none of whose frames writes a length: the
    /// length encoder is type-checked, a wrong signature reported as a written length's is, and is
    /// not left unused
    ///
    /// # Returns
    ///
    /// `if false { #len_encoder(0usize, out); }`
    pub(super) fn unwritten(self) -> proc_macro2::TokenStream {
        let (Self::Fixed(FixedLengthSlot { len_encoder, .. })
        | Self::Variable(VariableLength(len_encoder))) = self;
        quote! {
            if false {
                #len_encoder(0usize, out);
            }
        }
    }
}

/// A fixed-width length slot: the encoder writing the length, and the byte width it must fill
#[derive(Debug, Clone, Copy)]
pub(super) struct FixedLengthSlot<'a> {
    /// Encoder expression writing the length
    len_encoder: &'a types::XcoderType,

    /// Exact byte width of the encoded length
    width: usize,
}

/// [`FixedLengthSlot`] implementation
impl FixedLengthSlot<'_> {
    /// The exact byte width of the encoded length
    ///
    /// # Returns
    ///
    /// The `W` of `len(size(exact = W))`
    pub(super) fn width(self) -> usize {
        self.width
    }

    /// The largest length the slot encodes
    ///
    /// # Returns
    ///
    /// `256^width - 1`, saturating at `usize::MAX` when the slot is as wide as the host `usize`
    /// or wider
    fn max_len(self) -> usize {
        u32::try_from(self.width)
            .ok()
            .and_then(|w| 256usize.checked_pow(w))
            .map_or(usize::MAX, |b| b - 1)
    }

    /// Whether the slot encodes a length of `n`
    ///
    /// # Arguments
    ///
    /// * `n` - The length to encode
    ///
    /// # Returns
    ///
    /// `true` when `n <= self.max_len()`
    fn fits(self, n: usize) -> bool {
        n <= self.max_len()
    }

    /// Writes `len` with the length encoder, checking in debug builds that it fills the slot it is
    /// moved into
    ///
    /// The statements bind `__tail` to `out.len()` before the length is written, so `__tail` is
    /// where the written length starts. [`Self::write_filled`] reads it to move the length into
    /// the slot
    ///
    /// # Arguments
    ///
    /// * `len` - The length expression written into the slot
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// The `__tail` binding, the length write, then its `debug_assert_eq!` on the bytes written
    fn write_len(self, len: &impl ToTokens, span: proc_macro2::Span) -> proc_macro2::TokenStream {
        let Self { len_encoder, width } = self;
        quote_spanned! { span =>
            let __tail = out.len();
            #len_encoder(#len, out);
            ::core::debug_assert_eq!(
                out.len() - __tail,
                #width,
                "tinyklv: length encoder wrote a different byte count than its declared fixed `size`",
            );
        }
    }

    /// Writes a prefix, the length, then a value whose width is known
    ///
    /// # Arguments
    ///
    /// * `prefix_write` - Generated statements that write the field key
    /// * `value` - The value's known width
    /// * `value_write` - Builds the statements appending the value bytes to the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// A block writing the complete frame into `out`, the length as its encoder writes it, which
    /// may take more than the slot's width, checking in debug builds that the value encoder wrote
    /// `value.bytes` bytes, or the [`Self::too_narrow`] error
    pub(super) fn write_known(
        self,
        prefix_write: proc_macro2::TokenStream,
        value: ValueWidth<'_>,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        if let Some(too_narrow) = self.too_narrow(value) {
            return too_narrow;
        }
        let Self { len_encoder, .. } = self;
        let bytes = value.bytes;
        let len_write = quote_spanned! { span => #len_encoder(#bytes, out); };
        value.write_after(quote! { #prefix_write #len_write }, value_write, span)
    }

    /// Writes a prefix, the length, then a value whose width is expected, back-filling the slot
    /// from the bytes written when the value takes another width
    ///
    /// Only a user's encoder named as a built-in one writes another width, so the bytes written are
    /// compared with the width only when `foreign` holds, a constant `false` for the crate's own
    /// encoders
    ///
    /// # Arguments
    ///
    /// * `prefix_write` - Generated statements that write the field key and bind the value encoder
    ///   `foreign` reads
    /// * `value` - The value's expected width
    /// * `foreign` - Whether the value encoder may write another width, a `bool` expression
    /// * `value_write` - Builds the statements appending the value bytes to the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// The [`Self::write_filled`] block, or the [`Self::too_narrow`] error
    pub(super) fn write_expected(
        self,
        prefix_write: proc_macro2::TokenStream,
        value: ValueWidth<'_>,
        foreign: &proc_macro2::TokenStream,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        if let Some(too_narrow) = self.too_narrow(value) {
            return too_narrow;
        }
        let Self { len_encoder, .. } = self;
        let bytes = value.bytes;
        let len_write = quote_spanned! { span => #len_encoder(#bytes, out); };
        let mismatch = quote! { #foreign && __body_len != #bytes };
        self.write_filled(prefix_write, len_write, Some(mismatch), value_write, span)
    }

    /// Writes a prefix, reserves the slot, writes the body, then back-fills the slot
    ///
    /// # Arguments
    ///
    /// * `prefix_write` - Generated statements that write the field key or sentinel
    /// * `value_write` - Builds the statements appending the body bytes to the buffer it is given
    /// * `span` - Span of the `out` handed to `value_write`
    ///
    /// # Returns
    ///
    /// The [`Self::write_filled`] block
    pub(super) fn write_backfilled(
        self,
        prefix_write: proc_macro2::TokenStream,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let width = self.width;
        let reserve = quote! { out.resize(__len_pos + #width, 0u8); };
        self.write_filled(prefix_write, reserve, None, value_write, span)
    }

    /// The compile error for a value wider than the slot encodes, at [`ValueWidth::declared_at`]
    /// naming [`ValueWidth::field`], emitted in place of the write so every other field still
    /// expands
    ///
    /// # Arguments
    ///
    /// * `value` - The value's width
    ///
    /// # Returns
    ///
    /// The `compile_error!`, or `None` when the slot encodes `value.bytes`
    fn too_narrow(self, value: ValueWidth<'_>) -> Option<proc_macro2::TokenStream> {
        let ValueWidth {
            field,
            bytes,
            declared_at,
        } = value;
        (!self.fits(bytes)).then(|| {
            let too_narrow = crate::Error::LenPrefixTooNarrow {
                field: field.to_string(),
                value_bytes: bytes,
                width: self.width,
                max_len: self.max_len(),
            };
            syn::Error::new(declared_at, too_narrow).to_compile_error()
        })
    }

    /// Writes a prefix, the slot, the body, then fills the slot from the bytes written
    ///
    /// The slot is filled when `mismatch` holds, on a cold path, or always without one. The length
    /// is written at the tail with [`Self::write_len`], copied from its `__tail` into the slot, and
    /// the tail is truncated. When the slot is too narrow to represent the body, the generated code
    /// logs the error and rolls back the entire KLV unit
    ///
    /// # Arguments
    ///
    /// * `prefix_write` - Generated statements that write the field key or sentinel
    /// * `slot_write` - Generated statements writing the slot's bytes at `__len_pos`
    /// * `mismatch` - The `bool` expression, reading `__body_len`, under which the slot is filled,
    ///   or `None` to always fill it
    /// * `value_write` - Builds the statements appending the body bytes to the buffer it is given
    /// * `span` - Span of the `out` handed to `value_write`
    ///
    /// # Returns
    ///
    /// A block containing the complete filled write
    fn write_filled(
        self,
        prefix_write: proc_macro2::TokenStream,
        slot_write: proc_macro2::TokenStream,
        mismatch: Option<proc_macro2::TokenStream>,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let width = self.width;
        let value_write = value_write(&quote_spanned! { span => out });
        let len_write = self.write_len(&quote! { __body_len }, proc_macro2::Span::call_site());
        let fill = quote! {
            #len_write
            out.copy_within(__tail.., __len_pos);
            out.truncate(__tail);
        };
        // --------------------------------------------------
        // a slot narrower than usize rolls back an overflowing body, compared as `u64`: the bound
        // can exceed a 32-bit target's usize
        // --------------------------------------------------
        let max_body_len = self
            .max_len()
            .checked_add(1)
            .and_then(|bound| u64::try_from(bound).ok());
        let (field_start, fill) = match max_body_len {
            Some(max_body_len) => {
                let logger = crate::expand::logger::error_logger();
                let fill = quote! {
                    if __body_len as u64 >= #max_body_len {
                        #logger(
                            "tinyklv: KLV item body of {} bytes exceeds its {}-byte length prefix; omitting the item",
                            __body_len,
                            #width,
                        );
                        out.truncate(__field_start);
                    } else {
                        #fill
                    }
                };
                (Some(quote! { let __field_start = out.len(); }), fill)
            }
            None => (None, fill),
        };
        // --------------------------------------------------
        // a slot written with the expected width is filled only on a mismatch
        // --------------------------------------------------
        let fill = match mismatch {
            Some(mismatch) => quote! {
                if #mismatch {
                    ::core::hint::cold_path();
                    #fill
                }
            },
            None => fill,
        };
        quote! {
            {
                #field_start
                #prefix_write
                let __len_pos = out.len();
                #slot_write
                let __body_start = out.len();
                #value_write
                let __body_len = out.len() - __body_start;
                #fill
            }
        }
    }
}

/// A variable-width length: the encoder writing a staged value's length
#[derive(Debug, Clone, Copy)]
pub(super) struct VariableLength<'a>(&'a types::XcoderType);

/// [`VariableLength`] implementation
impl VariableLength<'_> {
    /// Stages a value, then writes the prefix, the staged length, and the staged value
    ///
    /// The length argument is located at the derive, as a fixed-width slot's is, so a length
    /// encoder of the wrong signature is one error, labelled at the encoder, however many frames it
    /// writes
    ///
    /// # Arguments
    ///
    /// * `staging` - The buffer the value is staged in
    /// * `prefix_write` - Generated statements writing what precedes the length, the field key or
    ///   sentinel, which may read the staged buffer
    /// * `value_write` - Builds the statements encoding the value into the buffer it is given
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// The staged write from [`Staging::write`]
    pub(super) fn write_staged(
        self,
        staging: Staging,
        prefix_write: proc_macro2::TokenStream,
        value_write: impl FnOnce(&proc_macro2::TokenStream) -> proc_macro2::TokenStream,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let Self(len_encoder) = self;
        let buffer = staging.buffer();
        let header = quote! { #prefix_write #len_encoder(#buffer.len(), out); };
        staging.write(header, value_write, span)
    }
}

#[cfg(test)]
mod tests {
    use super::FixedLengthSlot;
    use crate::ast::types::XcoderType;

    use quote::quote;

    #[test]
    /// Verifies a back-filled body is bounded as a `u64`: a four-byte slot's bound, `2^32`,
    /// overflows a 32-bit target's `usize`, where a `usize` literal would wrap to 0 and roll back
    /// every body
    fn a_four_byte_slot_bounds_its_body_as_a_u64() {
        let len_encoder: XcoderType = syn::parse_quote! { encb::be_u32_from_usize };
        let slot = FixedLengthSlot {
            len_encoder: &len_encoder,
            width: 4,
        };
        let write = slot.write_backfilled(
            quote! {},
            |out| quote! { body(#out); },
            proc_macro2::Span::call_site(),
        );
        let bound = quote! { if __body_len as u64 >= 4294967296u64 }.to_string();
        assert!(write.to_string().contains(&bound), "{write}");
    }
}
