//! Orchestration of the encode code-generation pass
//!
//! Top-level entry point for the `tinyklv::prelude::EncodeValue` and
//! `tinyklv::prelude::EncodeFrame` derive implementations. It derives an [`EncodeCtx`] from the
//! container once, then assembles the impls its methods emit, each from the sub-module that owns
//! it:
//!
//! * `encode_value_gen` - the `EncodeValue` impl
//! * `fixed_width` - compile-time byte-width resolution
//! * `frame_body` - what each frame writes after its key, and the value widths known at compile
//!   time
//! * `framing` - the frame header written around each field value, per the container's framing mode
//! * `klv_item_gen` - field-level KLV item emission, the retained unknown frames included
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
/// Resolves fixed byte widths for primitive encode paths
mod fixed_width;
/// What each frame writes after its key, and the value widths known at compile time
mod frame_body;
/// Writes the frame header around each field value, per the container's framing mode
mod framing;
/// Generates per-field KLV item writes and the retained unknown frames' writes
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
use crate::ast::{
    attr::{MainContainer, UnknownField},
    types::SiguledXcoder,
};
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
    /// back-filled sentinel frame's reserve includes, or `None` when it is too large to reserve
    reserve_bytes: Option<usize>,

    /// The frames the `#[klv(unknown)]` field keeps, and their encoder
    unknown: Option<UnknownEncoder<'a>>,
}

/// The [`UnknownFrames`](crate::ast::attr::UnknownFrames) of a container that gets an encode
/// impl, as its encode re-emits them
#[derive(Debug, Clone, Copy)]
struct UnknownEncoder<'a> {
    /// Token expression of the `unknown(enc = ..)` encoder, which writes each kept frame after
    /// its key
    encoder: &'a SiguledXcoder,

    /// The field whose frames are re-emitted
    keep: &'a UnknownField<'a>,
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
    /// The context, or `None` when every field of the struct is without `#[klv(..)]`, or a field,
    /// unknown-frame or framing encoder is missing (`allow_unimplemented_encode`), in which case no
    /// encode impl is generated
    fn new(input: &'a MainContainer<'a>) -> Option<Self> {
        // --------------------------------------------------
        // an encoder on every field, and a KLV field or no field at all; else no encode impl
        // --------------------------------------------------
        if !input.generates_with(|codec| codec.enc.is_some()) {
            return None;
        }
        // --------------------------------------------------
        // an encoder for the retained unknown frames; else no encode impl. Frames no field
        // keeps are not re-emitted
        // --------------------------------------------------
        let unknown = match input.kept() {
            Some(keep) => Some(UnknownEncoder {
                encoder: input
                    .unknown
                    .as_ref()
                    .and_then(|frames| frames.codec.enc.as_ref())?,
                keep,
            }),
            None => None,
        };
        // --------------------------------------------------
        // frame header encoder(s); none means no encode impl
        // --------------------------------------------------
        let framing = EncodeFraming::from_mode(&input.attrs.mode)?;
        Some(Self {
            input,
            framing,
            reserve_bytes: framing.value_reserve_bytes(input),
            unknown,
        })
    }
}

/// Generates all encode trait implementation tokens for a container
///
/// Emits an `EncodeValue` impl when the container's framing mode carries its encoder(s) and
/// every keyed `#[klv]` field carries an encoder, with a KLV field, the `#[klv(unknown)]` one
/// included, or no field at all, and `unknown(..)` carries one when a `#[klv(unknown)]` field
/// keeps unknown frames. Without `allow_unimplemented_encode` a missing encoder is a compile error
/// reported during parsing; with it, a missing field,
/// unknown-frame or framing encoder suppresses the encode impls entirely. If a
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
/// `EncodeFrame` impl when the container declares a sentinel, or `None` when every field of the
/// struct is without `#[klv(..)]`, or a field, unknown-frame or framing encoder is missing under
/// `allow_unimplemented_encode`
pub(super) fn gen_encode_impl(input: &MainContainer) -> Option<TokenStream> {
    let ctx = EncodeCtx::new(input)?;
    // --------------------------------------------------
    // the `EncodeValue` impl, then the `EncodeFrame` impl when a sentinel is declared
    // --------------------------------------------------
    let encode_value = ctx.encode_value_impl()?;
    let encode_frame = ctx.encode_frame_impl();
    Some(quote! {
        #encode_value
        #encode_frame
    })
}
