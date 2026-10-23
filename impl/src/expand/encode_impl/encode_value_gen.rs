//! `EncodeValue` impl emission
//!
//! Generates the value-body encoder for a container: reserve capacity, declare
//! the scratch buffer only when the framing stages values, then emit one
//! complete KLV item write per attributed field and per retained unknown frame
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{EncodeCtx, frame_body::FrameBody, framing::EncodeFraming, staging::Staging};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// [`EncodeCtx`] implementation of the `EncodeValue` impl
impl EncodeCtx<'_> {
    /// Generates the `EncodeValue` trait implementation for a container
    ///
    /// Builds the derived value encoder body, including the encode reserve unless it is too large
    /// ([`EncodeFraming::value_reserve_bytes`](super::framing::EncodeFraming::value_reserve_bytes)),
    /// optional scratch buffer declaration, a length write that never runs when no frame writes a
    /// length ([`LengthPrefix::unwritten`](super::length_prefix::LengthPrefix::unwritten)), and
    /// one generated KLV item write for each
    /// attributed field and each retained unknown frame ([`Self::klv_items`]). The generated impl
    /// writes directly into the caller's output buffer and does not emit any frame sentinel bytes
    ///
    /// # Returns
    ///
    /// A token stream containing one `EncodeValue` trait implementation for the
    /// container, or `None` when a key has no value encoder
    pub(super) fn encode_value_impl(&self) -> Option<proc_macro2::TokenStream> {
        // --------------------------------------------------
        // split the container type
        // --------------------------------------------------
        let name = &self.input.ident;
        let (impl_generics, ty_generics, where_clause) = self.input.generics.split_for_impl();
        // --------------------------------------------------
        // build reserve, scratch and frame emission
        // --------------------------------------------------
        let reserve = self
            .reserve_bytes
            .map(|bytes| quote! { out.reserve(#bytes); });
        // --------------------------------------------------
        // the scratch buffer, when a frame stages its value. A retained unknown frame is staged
        // only under a fused header, which always declares the buffer
        // --------------------------------------------------
        let staged = matches!(self.framing, EncodeFraming::Fused { .. })
            || self
                .input
                .klv_keys()
                .any(|key| FrameBody::of(self.framing, &key).stages());
        let scratch_decl = staged.then(|| Staging::Scratch.declare());
        let klv_items = self.klv_items()?;
        // --------------------------------------------------
        // a length encoder no frame calls, every one `size(implicit)` and no packet's, is still
        // type-checked
        // --------------------------------------------------
        let unwritten_len = match self.framing {
            EncodeFraming::Separate { len, .. }
                if !self.input.attrs.mode.len_frames_packet()
                    && self
                        .input
                        .klv_keys()
                        .all(|key| key.codec.implicit().is_some()) =>
            {
                Some(len.unwritten())
            }
            _ => None,
        };
        // --------------------------------------------------
        // assemble the trait impl
        // --------------------------------------------------
        Some(quote! {
            #[automatically_derived]
            impl #impl_generics ::tinyklv::traits::EncodeValue for #name #ty_generics #where_clause {
                fn encode_value(&self, out: &mut ::std::vec::Vec<::core::primitive::u8>) {
                    #reserve
                    #scratch_decl
                    #unwritten_len
                    #klv_items
                }
            }
        })
    }
}
