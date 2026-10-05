//! Orchestration of the encode code-generation pass
//!
//! Top-level entry point for the `tinyklv::prelude::EncodeValue` and
//! `tinyklv::prelude::EncodeFrame` derive implementations. This file only
//! resolves container-level inputs, delegates the actual impl bodies to focused
//! generators, and assembles the final token stream:
//!
//! * `encode_value_gen` - the `EncodeValue` impl
//! * `field_length_plan` - the container length-prefix shape and the fixed-prefix field length
//!   kinds
//! * `fixed_width` - compile-time byte-width resolution
//! * `framing` - the frame header written around each field value, per the container's framing mode
//! * `klv_item_gen` - field-level KLV item emission
//! * `length_slot_gen` - the fixed-width length slot
//! * `sentinel_gen` - the `EncodeFrame` impl (only when a sentinel is declared)
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
/// Generates required `EncodeValue` impl bodies
mod encode_value_gen;
/// Holds the container length-prefix shape and the fixed-prefix field length kinds
mod field_length_plan;
/// Resolves fixed byte widths for primitive encode paths
mod fixed_width;
/// Writes the frame header around each field value, per the container's framing mode
mod framing;
/// Generates per-field KLV item writes
mod klv_item_gen;
/// The fixed-width length slot
mod length_slot_gen;
/// Generates the optional sentinel-framed `EncodeFrame` impl
mod sentinel_gen;

// --------------------------------------------------
// local
// --------------------------------------------------
use self::framing::EncodeFraming;
use crate::ast::attr::MainContainer;

// --------------------------------------------------
// external
// --------------------------------------------------
use quote::quote;

/// Generates all encode trait implementation tokens for a container
///
/// Emits an `EncodeValue` impl when the container's framing mode carries its encoder(s) and
/// every `#[klv]` field carries an encoder, with at least one such field. Without
/// `allow_unimplemented_encode` a missing encoder is a compile error reported during parsing;
/// with it, a missing field or framing encoder suppresses the encode impls entirely. If a
/// sentinel is present, also emits an `EncodeFrame` impl that wraps the value encoding in a KLV
/// packet consisting of the sentinel key, encoded frame length, and encoded value body
///
/// # Arguments
///
/// * `input` - The parsed container descriptor including field metadata
///
/// # Returns
///
/// A token stream containing the generated `EncodeValue` impl followed by the generated
/// `EncodeFrame` impl when the container declares a sentinel, or `None` when the struct has no
/// KLV field, or a field or framing encoder is missing under `allow_unimplemented_encode`
pub(super) fn gen_encode_impl(input: &MainContainer) -> Option<proc_macro2::TokenStream> {
    // --------------------------------------------------
    // an encoder on every field, and at least one field; else no encode impl
    // --------------------------------------------------
    if !input.every_klv_field(|f| f.enc.is_some()) {
        return None;
    }
    // --------------------------------------------------
    // frame header encoder(s); none means no encode impl
    // --------------------------------------------------
    let framing = EncodeFraming::from_mode(&input.attrs.mode)?;
    // --------------------------------------------------
    // the `EncodeValue` impl, sharing its reserve estimate with the frame
    // --------------------------------------------------
    let reserve_bytes = framing.value_reserve_bytes(input);
    let encode_value = encode_value_gen::gen_encode_value_impl(input, framing, reserve_bytes);
    // --------------------------------------------------
    // the `EncodeFrame` impl: a fused container rejects `sentinel` at parse time, so a
    // sentinel always comes with the separate length encoder that writes the frame length
    // --------------------------------------------------
    let encode_frame = match (input.attrs.sentinel.as_ref(), framing) {
        (
            Some(sentinel),
            EncodeFraming::Separate {
                len_encoder,
                len_prefix,
                ..
            },
        ) => sentinel_gen::gen_encode_frame_impl(
            &input.ident,
            input.generics,
            sentinel,
            len_encoder,
            len_prefix,
            reserve_bytes,
        ),
        (Some(_), EncodeFraming::Fused { .. }) | (None, _) => quote! {},
    };
    // --------------------------------------------------
    // assemble: the `EncodeValue` impl, then optional `EncodeFrame` impl
    // --------------------------------------------------
    Some(quote! {
        #encode_value
        #encode_frame
    })
}
