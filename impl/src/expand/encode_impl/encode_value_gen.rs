//! `EncodeValue` impl emission
//!
//! Generates the value-body encoder for a container: reserve capacity, declare
//! the scratch buffer only when variable-width lengths need it, then emit one
//! complete KLV item write per attributed field
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{framing::EncodeFraming, klv_item_gen};
use crate::ast::attr::MainContainer;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{format_ident, quote};

/// Generates the `EncodeValue` trait implementation for a container
///
/// Builds the derived value encoder body, including the reserve hint, optional
/// scratch buffer declaration, and one generated KLV item write for each
/// attributed field. The generated impl writes directly into the caller's
/// output buffer and does not emit any frame sentinel bytes
///
/// # Arguments
///
/// * `input` - Parsed container metadata and field descriptors for the derive
/// * `framing` - The frame header encoder(s) written around each field value
/// * `reserve_bytes` - The reserve hint from [`EncodeFraming::value_reserve_bytes`]
///
/// # Returns
///
/// A token stream containing one `EncodeValue` trait implementation for the
/// container described by `input`
pub(super) fn gen_encode_value_impl(
    input: &MainContainer,
    framing: EncodeFraming<'_>,
    reserve_bytes: usize,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // split the container type
    // --------------------------------------------------
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    // --------------------------------------------------
    // build scratch and field item emission
    // --------------------------------------------------
    let scratch = format_ident!("__scratch");
    let scratch_decl = gen_scratch_decl(framing, &scratch);
    let klv_items = klv_item_gen::gen_klv_items(input, framing, &scratch);
    // --------------------------------------------------
    // assemble the trait impl
    // --------------------------------------------------
    quote! {
        #[automatically_derived]
        impl #impl_generics ::tinyklv::traits::EncodeValue for #name #ty_generics #where_clause {
            fn encode_value(&self, out: &mut ::std::vec::Vec<u8>) {
                out.reserve(#reserve_bytes);
                #scratch_decl
                #klv_items
            }
        }
    }
}

/// Emits the per-value scratch buffer declaration when field values need staging
///
/// A framing that stages values ([`EncodeFraming::stages_values`]) encodes each
/// field value into a scratch buffer before its frame header is written. Fixed-width
/// plans can write directly to the caller output, so this helper emits no declaration
/// for that case and keeps generated methods free of unused locals
///
/// # Arguments
///
/// * `framing` - The frame header encoder(s), deciding whether values are staged
/// * `scratch` - Identifier to use for the generated scratch buffer binding
///
/// # Returns
///
/// A token stream containing a mutable `Vec<u8>` declaration when staging is
/// required, or an empty token stream otherwise
fn gen_scratch_decl(framing: EncodeFraming<'_>, scratch: &syn::Ident) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // declare scratch only for staged encoded fields
    // --------------------------------------------------
    if framing.stages_values() {
        quote! { let mut #scratch: ::std::vec::Vec<u8> = ::std::vec::Vec::new(); }
    } else {
        quote! {}
    }
}
