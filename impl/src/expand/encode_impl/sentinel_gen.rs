//! `EncodeFrame` impl emission
//!
//! Generates the sentinel-framed encoder for containers that declare a
//! sentinel. Fixed-width frame lengths back-fill the shared fixed-width length slot;
//! variable-width frame lengths stage the whole body first
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{EncodeCtx, framing::EncodeFraming, length_prefix::LengthPrefix, staging::Staging};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Extra reserve space for a variable-width frame length prefix
const VAR_LEN_PREFIX_BYTES: usize = 8;

/// [`EncodeCtx`] implementation of the `EncodeFrame` impl
impl EncodeCtx<'_> {
    /// Generates the `EncodeFrame` impl of a container that declares a sentinel
    ///
    /// The generated `encode_frame` writes the sentinel, the frame length with the separate length
    /// encoder, and the encoded value body. A fixed-width frame length reserves a length slot and
    /// back-fills it after writing the value, while a variable-width one encodes the body into a
    /// fresh buffer so the final length is known before emission
    ///
    /// # Returns
    ///
    /// A token stream containing the generated `EncodeFrame` implementation, or `None` when the
    /// container declares no sentinel
    pub(super) fn encode_frame_impl(&self) -> Option<proc_macro2::TokenStream> {
        // --------------------------------------------------
        // the sentinel, and the separate length encoder that writes the frame length
        // --------------------------------------------------
        let EncodeFraming::Separate {
            len,
            sentinel: Some(sentinel),
            ..
        } = self.framing
        else {
            return None;
        };
        let call_site = proc_macro2::Span::call_site();
        // --------------------------------------------------
        // select fixed-slot or staged variable-length frame writing
        // --------------------------------------------------
        let frame_write = match len {
            LengthPrefix::Fixed(len) => {
                // --------------------------------------------------
                // reserve the frame's capacity, then write the sentinel
                // --------------------------------------------------
                let width = len.width();
                let reserve_bytes = self.reserve_bytes;
                let sentinel_write = quote! {
                    out.reserve(#reserve_bytes + 2 * #width + #sentinel.len());
                    out.extend_from_slice(#sentinel);
                };
                // --------------------------------------------------
                // delegate fixed-slot back-fill bookkeeping
                // --------------------------------------------------
                len.write_backfilled(
                    sentinel_write,
                    |out| quote! { self.encode_value(#out); },
                    call_site,
                )
            }
            LengthPrefix::Variable(len) => {
                // --------------------------------------------------
                // stage the body in a fresh buffer, sized into the reserve
                // --------------------------------------------------
                let staging = Staging::Fresh;
                let buffer = staging.buffer();
                let sentinel_write = quote! {
                    out.reserve(#buffer.len() + #sentinel.len() + #VAR_LEN_PREFIX_BYTES);
                    out.extend_from_slice(#sentinel);
                };
                len.write_staged(
                    staging,
                    sentinel_write,
                    |body| quote! { self.encode_value(#body); },
                    call_site,
                )
            }
        };
        // --------------------------------------------------
        // assemble the trait impl
        // --------------------------------------------------
        let name = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        Some(quote! {
            #[automatically_derived]
            impl #impl_generics ::tinyklv::traits::EncodeFrame for #name #ty_generics #where_clause {
                fn encode_frame(&self, out: &mut ::std::vec::Vec<u8>) {
                    #frame_write
                }
            }
        })
    }
}
