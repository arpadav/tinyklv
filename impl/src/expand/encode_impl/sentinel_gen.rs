//! `EncodeFrame` impl emission
//!
//! Generates the sentinel-framed encoder for containers that declare a
//! sentinel. Fixed-width frame lengths back-fill the shared fixed-width length slot;
//! variable-width frame lengths stage the whole body first. The frame length is written with the
//! sentinel's own `len(..)`, else the container's
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{
    EncodeCtx,
    framing::MAX_RESERVE_BYTES,
    length_prefix::{KlvUnit, LengthPrefix},
    staging::Staging,
};

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
    /// The generated `encode_frame` writes the sentinel, the frame length with the packet's length
    /// encoder ([`ContainerMode::packet`](crate::ast::attr::container::ContainerMode::packet)),
    /// and the encoded value body. A fixed-width frame length reserves a length slot and
    /// back-fills it after writing the value, while a variable-width one encodes the body into a
    /// fresh buffer so the final length is known before emission
    ///
    /// # Returns
    ///
    /// A token stream containing the generated `EncodeFrame` implementation, or `None` when the
    /// container declares no sentinel, or the packet's length encoder is missing
    /// (`allow_unimplemented_encode`)
    pub(super) fn encode_frame_impl(&self) -> Option<proc_macro2::TokenStream> {
        // --------------------------------------------------
        // the sentinel, and the length encoder that writes the frame length
        // --------------------------------------------------
        let (sentinel, len) = self.input.attrs.mode.packet()?;
        let len = LengthPrefix::new(len.enc.as_ref()?, &len.size);
        let call_site = proc_macro2::Span::call_site();
        // --------------------------------------------------
        // the body is the derive's own `EncodeValue`, never another `encode_value` in scope
        // --------------------------------------------------
        let value_write = |out: &proc_macro2::TokenStream| {
            quote! { <Self as ::tinyklv::traits::EncodeValue>::encode_value(self, #out); }
        };
        // --------------------------------------------------
        // select fixed-slot or staged variable-length frame writing
        // --------------------------------------------------
        let frame_write = match len {
            LengthPrefix::Fixed(len) => {
                // --------------------------------------------------
                // reserve the frame's capacity, unless it is too large, then write the sentinel
                // --------------------------------------------------
                let width = len.width();
                let reserve = self.reserve_bytes.and_then(|bytes| {
                    let frame = width.checked_mul(2)?.checked_add(bytes)?;
                    (frame <= MAX_RESERVE_BYTES).then(|| {
                        quote! { out.reserve(#bytes + 2 * #width + #sentinel.len()); }
                    })
                });
                let sentinel_write = quote! {
                    #reserve
                    out.extend_from_slice(#sentinel);
                };
                // --------------------------------------------------
                // delegate fixed-slot back-fill bookkeeping
                // --------------------------------------------------
                len.write_backfilled(KlvUnit::Packet, sentinel_write, value_write, call_site)
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
                len.write_staged(staging, sentinel_write, value_write, call_site)
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
                fn encode_frame(&self, out: &mut ::std::vec::Vec<::core::primitive::u8>) {
                    #frame_write
                }
            }
        })
    }
}
