//! The fixed-width length slot
//!
//! [`FixedLengthSlot`] owns everything about a `len(size(exact = W))` slot: the largest length
//! its `W` bytes can encode, the length write checked against `W` in debug builds, and the
//! complete write for a KLV unit whose body length is unknown until after the body is encoded:
//! write the prefix, reserve the slot, write the value, then back-fill the slot or roll back the
//! whole unit if the value is too large
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::types;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{ToTokens, quote, quote_spanned};

/// A fixed-width length slot: the encoder writing the length, and the byte width it must fill
#[derive(Debug, Clone, Copy)]
pub(super) struct FixedLengthSlot<'a> {
    /// Encoder expression writing the length
    pub(super) len_encoder: &'a types::XcoderType,

    /// Exact byte width of the encoded length
    pub(super) width: usize,
}

/// [`FixedLengthSlot`] implementation
impl FixedLengthSlot<'_> {
    /// The largest length the slot encodes
    ///
    /// # Returns
    ///
    /// `256^width - 1`, saturating at `usize::MAX` when the slot is as wide as the host `usize`
    /// or wider
    pub(super) fn max_len(self) -> usize {
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
    pub(super) fn fits(self, n: usize) -> bool {
        n <= self.max_len()
    }

    /// Writes `len` with the length encoder, checking in debug builds that it filled the slot
    ///
    /// # Arguments
    ///
    /// * `len` - The length expression written into the slot
    /// * `span` - Span of the emitted write
    ///
    /// # Returns
    ///
    /// The length write followed by its `debug_assert_eq!` on the bytes written
    pub(super) fn write_len(
        self,
        len: &impl ToTokens,
        span: proc_macro2::Span,
    ) -> proc_macro2::TokenStream {
        let Self { len_encoder, width } = self;
        quote_spanned! { span =>
            let __tail = out.len();
            #len_encoder(#len, out);
            debug_assert_eq!(
                out.len() - __tail,
                #width,
                "tinyklv: length encoder wrote a different byte count than its declared fixed `size`",
            );
        }
    }

    /// Writes a prefix, reserves the slot, writes the body, then back-fills the slot
    ///
    /// When the slot is too narrow to represent the body, the generated code logs the error and
    /// rolls back the entire KLV unit
    ///
    /// # Arguments
    ///
    /// * `prefix_write` - Generated statements that write the field key or sentinel
    /// * `value_write` - Generated statements that append the body bytes to `out`
    ///
    /// # Returns
    ///
    /// A block containing the complete back-filled write
    pub(super) fn write_backfilled(
        self,
        prefix_write: proc_macro2::TokenStream,
        value_write: proc_macro2::TokenStream,
    ) -> proc_macro2::TokenStream {
        let width = self.width;
        let len_write = self.write_len(&quote! { __body_len }, proc_macro2::Span::call_site());
        // --------------------------------------------------
        // a slot narrower than usize rolls back an overflowing body
        // --------------------------------------------------
        if let Some(max_body_len) = self.max_len().checked_add(1) {
            let logger = crate::expand::logger::error_logger();
            return quote! {
                {
                    let __field_start = out.len();
                    #prefix_write
                    let __len_pos = out.len();
                    out.resize(__len_pos + #width, 0u8);
                    let __body_start = out.len();
                    #value_write
                    let __body_len = out.len() - __body_start;
                    if __body_len >= #max_body_len {
                        #logger(
                            "tinyklv: KLV item body of {} bytes exceeds its {}-byte length prefix; omitting the item",
                            __body_len,
                            #width,
                        );
                        out.truncate(__field_start);
                    } else {
                        #len_write
                        out.copy_within(__tail.., __len_pos);
                        out.truncate(__tail);
                    }
                }
            };
        }
        // --------------------------------------------------
        // a usize-wide slot back-fills directly
        // --------------------------------------------------
        quote! {
            {
                #prefix_write
                let __len_pos = out.len();
                out.resize(__len_pos + #width, 0u8);
                let __body_start = out.len();
                #value_write
                let __body_len = out.len() - __body_start;
                #len_write
                out.copy_within(__tail.., __len_pos);
                out.truncate(__tail);
            }
        }
    }
}
