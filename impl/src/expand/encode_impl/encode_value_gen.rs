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
use super::{field_length_plan::ContainerLengthPrefix, fixed_width, klv_item_gen};
use crate::ast::{
    attr::{MainContainer, size::SizeSpec},
    types,
};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::{format_ident, quote};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Fallback per-field key-byte reserve estimate
const DEFAULT_KEY_BYTES: usize = 1;

/// Fallback per-field length-byte reserve estimate
const DEFAULT_LEN_BYTES: usize = 1;

/// Extra reserve space for a variable-width BER frame length prefix
pub(super) const BER_LEN_PREFIX_BYTES: usize = 8;

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
/// * `key_encoder` - Encoder expression used for each generated field key
/// * `len_encoder` - Encoder expression used for each generated field length
/// * `len_prefix` - Container-level length-prefix strategy shared by fields
///
/// # Returns
///
/// A token stream containing one `EncodeValue` trait implementation for the
/// container described by `input`
///
/// # Example
///
/// ```rust,ignore
/// let tokens = gen_encode_value_impl(&container, &key_encoder, &len_encoder, &len_prefix);
/// assert!(!tokens.is_empty());
/// ```
pub(super) fn gen_encode_value_impl(
    input: &MainContainer,
    key_encoder: &types::SiguledXcoder,
    len_encoder: &types::SiguledXcoder,
    len_prefix: &ContainerLengthPrefix,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // split the container type and reserve plan
    // --------------------------------------------------
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let reserve_bytes = encoded_value_reserve_bytes(input, key_encoder);
    // --------------------------------------------------
    // build scratch and field item emission
    // --------------------------------------------------
    let scratch = format_ident!("__scratch");
    let scratch_decl = gen_scratch_decl(*len_prefix, input, &scratch);
    let klv_items =
        klv_item_gen::gen_klv_items(input, key_encoder, len_encoder, *len_prefix, &scratch);
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

/// Computes the encode-value capacity reserve hint in bytes
///
/// Estimates the number of bytes the generated `encode_value` method should
/// reserve before writing field items. The estimate combines field key width,
/// field length width, and any statically known value width. Unknown widths
/// contribute zero for the value portion, so the result is an allocation hint
/// rather than a wire-format contract
///
/// # Arguments
///
/// * `input` - Parsed container metadata whose attributed fields are counted
/// * `key_encoder` - Container key encoder used to infer a fixed key width
///
/// # Returns
///
/// The total byte reserve hint for all encoded value fields in the container
///
/// # Example
///
/// ```rust,ignore
/// let reserve = encoded_value_reserve_bytes(&container, &key_encoder);
/// assert!(reserve <= usize::MAX);
/// ```
pub(super) fn encoded_value_reserve_bytes(
    input: &MainContainer,
    key_encoder: &types::SiguledXcoder,
) -> usize {
    // --------------------------------------------------
    // estimate per-field key bytes
    // --------------------------------------------------
    let key_bytes = input
        .attrs
        .key
        .size
        .and_then(SizeSpec::capacity_bytes)
        .or_else(|| fixed_width::builtin_fixed_encoder_width(key_encoder))
        .unwrap_or(DEFAULT_KEY_BYTES);
    // --------------------------------------------------
    // estimate per-field length bytes
    // --------------------------------------------------
    let len_bytes = input
        .attrs
        .len
        .size
        .and_then(SizeSpec::capacity_bytes)
        .unwrap_or(DEFAULT_LEN_BYTES);
    // --------------------------------------------------
    // sum known field widths into the reserve hint
    // --------------------------------------------------
    input
        .data
        .iter()
        .filter_map(|field| field.attrs.as_ref().map(|attrs| (field.ty, attrs)))
        .map(|(ty, attrs)| {
            let value_bytes = fixed_width::fixed_value_type_width(ty)
                .or_else(|| attrs.size.and_then(SizeSpec::capacity_bytes))
                .unwrap_or(0);
            key_bytes + len_bytes + value_bytes
        })
        .sum()
}

/// Emits the per-value scratch buffer declaration when field values need staging
///
/// Variable-width container lengths require field values to be encoded into a
/// scratch buffer before their lengths are written. Fixed-width plans can write
/// directly to the caller output, so this helper emits no declaration for that
/// case and keeps generated methods free of unused locals
///
/// # Arguments
///
/// * `len_prefix` - Container length-prefix strategy that controls staging
/// * `input` - Parsed container metadata used to detect encoded fields
/// * `scratch` - Identifier to use for the generated scratch buffer binding
///
/// # Returns
///
/// A token stream containing a mutable `Vec<u8>` declaration when staging is
/// required, or an empty token stream otherwise
///
/// # Example
///
/// ```rust,ignore
/// let scratch = quote::format_ident!("__scratch");
/// let tokens = gen_scratch_decl(len_prefix, &container, &scratch);
/// assert!(tokens.to_string().contains("scratch") || tokens.is_empty());
/// ```
fn gen_scratch_decl(
    len_prefix: ContainerLengthPrefix,
    input: &MainContainer,
    scratch: &syn::Ident,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // declare scratch only for variable-width encoded fields
    // --------------------------------------------------
    if len_prefix.is_variable() && input.data.iter().any(|field| field.attrs.is_some()) {
        quote! { let mut #scratch: ::std::vec::Vec<u8> = ::std::vec::Vec::new(); }
    } else {
        quote! {}
    }
}
