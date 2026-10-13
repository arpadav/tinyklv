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
use super::EncodeCtx;

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
    /// optional scratch buffer declaration, and one generated KLV item write for each
    /// attributed field and each retained unknown frame ([`Self::klv_items`]). The generated impl
    /// writes directly into the caller's output buffer and does not emit any frame sentinel bytes
    ///
    /// # Returns
    ///
    /// A token stream containing one `EncodeValue` trait implementation for the
    /// container
    pub(super) fn encode_value_impl(&self) -> proc_macro2::TokenStream {
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
        let scratch_decl = self.framing.declare_scratch(self.input);
        let klv_items = self.klv_items();
        // --------------------------------------------------
        // assemble the trait impl
        // --------------------------------------------------
        quote! {
            #[automatically_derived]
            impl #impl_generics ::tinyklv::traits::EncodeValue for #name #ty_generics #where_clause {
                fn encode_value(&self, out: &mut ::std::vec::Vec<u8>) {
                    #reserve
                    #scratch_decl
                    #klv_items
                }
            }
        }
    }
}
