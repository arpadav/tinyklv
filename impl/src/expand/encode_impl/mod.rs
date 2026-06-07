//! Orchestration of the encode code-generation pass
//!
//! Top-level entry point for the `tinyklv::prelude::EncodeValue` and
//! `tinyklv::prelude::EncodeFrame` derive implementations This file only
//! resolves container-level inputs, delegates the actual impl bodies to focused
//! generators, and assembles the final token stream:
//!
//! * `encode_value_gen` - the `EncodeValue` impl
//! * `encode_frame_gen` - the `EncodeFrame` impl (only when a sentinel is declared)
//! * `klv_item_gen` - field-level KLV item emission
//! * `field_length_plan` - fixed/variable length strategy decisions
//! * `length_slot_gen` - fixed-width length-slot emission
//! * `fixed_width` - compile-time byte-width resolution
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
/// Generates optional `EncodeFrame` impl bodies
mod encode_frame_gen;
/// Generates required `EncodeValue` impl bodies
mod encode_value_gen;
/// Plans fixed and variable field length writes
mod field_length_plan;
/// Resolves fixed byte widths for primitive encode paths
mod fixed_width;
/// Generates per-field KLV item writes
mod klv_item_gen;
/// Generates fixed-width length-slot back-fill writes
mod length_slot_gen;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::{attr::MainContainer, types};

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates all encode trait implementation tokens for a container
///
/// Emits an `EncodeValue` impl for every container. If a sentinel is present,
/// also emits a full `EncodeFrame` impl that wraps the value encoding in a KLV
/// packet consisting of the sentinel key, encoded frame length, and encoded
/// value body. The returned stream is empty only for the optional frame portion
/// when the parsed container has no sentinel attribute
///
/// # Arguments
///
/// * `input` - The parsed container descriptor including field metadata
/// * `key_encoder` - Token expression used to encode each field's key
/// * `len_encoder` - Token expression used to encode the packet length
///
/// # Returns
///
/// A token stream containing the generated `EncodeValue` impl followed by the
/// generated `EncodeFrame` impl when the container declares a sentinel
///
/// # Example
///
/// ```rust,ignore
/// # use tinyklv_impl::expand::encode_impl::gen_encode_impl;
/// let tokens = gen_encode_impl(&container, &key_encoder, &len_encoder);
/// assert!(!tokens.is_empty());
/// ```
pub(crate) fn gen_encode_impl(
    input: &MainContainer,
    key_encoder: &types::SiguledXcoder,
    len_encoder: &types::SiguledXcoder,
) -> proc_macro2::TokenStream {
    // --------------------------------------------------
    // container length-prefix shape declared by `len(.., size(..))`
    // --------------------------------------------------
    let len_prefix = field_length_plan::ContainerLengthPrefix::from_size(input.attrs.len.size);
    // --------------------------------------------------
    // impl generators
    // --------------------------------------------------
    let encode_value =
        encode_value_gen::gen_encode_value_impl(input, key_encoder, len_encoder, &len_prefix);
    let encode_frame = encode_frame_gen::gen_encode_frame_impl(
        input,
        len_encoder,
        &len_prefix,
        encode_value_gen::encoded_value_reserve_bytes(input, key_encoder),
    );
    // --------------------------------------------------
    // assemble: the `EncodeValue` impl, then optional `EncodeFrame` impl
    // --------------------------------------------------
    quote! {
        #encode_value
        #encode_frame
    }
}
