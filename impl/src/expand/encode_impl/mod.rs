//! Orchestration of the encode code-generation pass
//!
//! Top-level entry point for the `tinyklv::prelude::EncodeValue` and
//! `tinyklv::prelude::EncodeFrame` derive implementations. It derives an [`EncodeCtx`] from the
//! container once, then assembles the impls its methods emit, each from the sub-module that owns
//! it:
//!
//! * `encode_value_gen` - the `EncodeValue` impl
//! * `field_length_plan` - the fixed-prefix field length kinds
//! * `fixed_width` - compile-time byte-width resolution
//! * `framing` - the frame header written around each field value, per the container's framing mode
//! * `klv_item_gen` - field-level KLV item emission
//! * `length_prefix` - the container's length prefix: a fixed-width slot or a variable-width length
//! * `sentinel_gen` - the `EncodeFrame` impl (only when a sentinel is declared)
//! * `staging` - the staged write: a value encoded into a buffer before its frame header
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
/// Generates required `EncodeValue` impl bodies
mod encode_value_gen;
/// Holds the fixed-prefix field length kinds
mod field_length_plan;
/// Resolves fixed byte widths for primitive encode paths
mod fixed_width;
/// Writes the frame header around each field value, per the container's framing mode
mod framing;
/// Generates per-field KLV item writes
mod klv_item_gen;
/// The container's length prefix: a fixed-width slot or a variable-width length
mod length_prefix;
/// Generates the optional sentinel-framed `EncodeFrame` impl
mod sentinel_gen;
/// Stages a value in a buffer before its frame header is written
mod staging;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::ast::attr::MainContainer;
use framing::EncodeFraming;

// --------------------------------------------------
// external
// --------------------------------------------------
use proc_macro2::TokenStream;
use quote::quote;

/// A container that gets encode impls, with everything those impls derive from it
///
/// Built once per derive by [`EncodeCtx::new`]. Each encode impl is a method, defined in the
/// sub-module that generates it
struct EncodeCtx<'a> {
    /// The parsed container, providing the name, generics, fields and attributes
    input: &'a MainContainer<'a>,

    /// The frame header written around each field value, per the container's framing mode
    framing: EncodeFraming<'a>,

    /// The `EncodeValue` body's encode reserve ([`EncodeFraming::value_reserve_bytes`]), which a
    /// back-filled sentinel frame's reserve includes
    reserve_bytes: usize,
}

/// [`EncodeCtx`] implementation
impl<'a> EncodeCtx<'a> {
    /// Derives the encode context of a container
    ///
    /// # Arguments
    ///
    /// * `input` - The parsed container descriptor including field metadata
    ///
    /// # Returns
    ///
    /// The context, or `None` when the struct has no KLV field, or a field or framing encoder is
    /// missing (`allow_unimplemented_encode`), in which case no encode impl is generated
    fn new(input: &'a MainContainer<'a>) -> Option<Self> {
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
        Some(Self {
            input,
            framing,
            reserve_bytes: framing.value_reserve_bytes(input),
        })
    }
}

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
pub(super) fn gen_encode_impl(input: &MainContainer) -> Option<TokenStream> {
    let ctx = EncodeCtx::new(input)?;
    // --------------------------------------------------
    // the `EncodeValue` impl, then the `EncodeFrame` impl when a sentinel is declared
    // --------------------------------------------------
    let encode_value = ctx.encode_value_impl();
    let encode_frame = ctx.encode_frame_impl();
    Some(quote! {
        #encode_value
        #encode_frame
    })
}
