//! Decode traits for KLV field values, frames, and partial/streaming decode
//!
//! Core decode-side traits:
//! * [`DecodeValue`] - decodes the value portion `V` of a KLV triple from stream `S`
//! * [`DecodeFrame`] (re-exported from `frame`) - decodes a full key-length-value frame
//! * [`DecodePartial`] / [`Partial`] / [`ResumePartial`] (re-exported from `partial`) -
//!   support for incremental decode of a packet across multiple byte deliveries
//! * [`SeekSentinel`] (re-exported from `sentinel`) - locate a packet boundary in a
//!   continuous byte stream
//! * [`BreakType`] (re-exported from `breaktype`) - the loop-control outcome a container's
//!   `#[klv(break_on = ..)]` expression yields for the derive-generated decode loop
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
mod breaktype;
mod frame;
mod partial;
mod sentinel;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use crate::prelude::*;
pub use breaktype::*;
pub use frame::*;
pub use partial::*;
pub use sentinel::*;

/// Decodes the value portion `V` of a KLV triple from stream `S`
///
/// Encode counterpart: [`crate::traits::EncodeValue`]
///
/// The `S` type parameter is the stream type (most commonly `&[u8]` or
/// `&str`). The method advances `input` by exactly the bytes it consumes;
/// on error the cursor position is unspecified (callers should restore a
/// checkpoint if they need to retry).
///
/// This trait is **automatically implemented** for structs deriving
/// [`tinyklv::Klv`](crate::Klv) when every field has an associated decoder.
/// It is also blanket-implemented for [`Vec<T>`] where `T: DecodeValue<S>`.
///
/// For ad-hoc custom decoders passed directly to `#[klv(dec = ...)]`,
/// you do **not** need to implement this trait. Instead, supply a free
/// function with one of the following signatures:
///
/// * Fixed-length field: `fn decoder(input: &mut S) -> tinyklv::Result<MyType>`
/// * Variable-length field: `fn decoder(len: usize) -> impl Fn(&mut S) -> tinyklv::Result<MyType>`
pub trait DecodeValue<S>: Sized
where
    S: winnow::stream::Stream,
{
    /// Decodes `Self` by consuming bytes from `input`
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to decode from; advanced by the consumed bytes on success
    ///
    /// # Returns
    ///
    /// `Ok(Self)` on success, or a [`winnow::error::ContextError`] describing the failure
    fn decode_value(input: &mut S) -> crate::Result<Self>;
}

/// [`Vec`] implementation of [`DecodeValue`]
///
/// Decodes repeated `T` values until the inner decoder fails, collecting
/// successes into a [`Vec`]. Appropriate for representing a repeated inner
/// field within a single parent KLV packet body.
///
/// For streaming a sequence of top-level packets across fragmented reads
/// (where a packet may straddle a buffer boundary), use [`crate::Decoder`]
/// instead.
///
/// Cursor safety: if `T::decode_value` fails *without* consuming any bytes,
/// the cursor is rewound to the pre-attempt checkpoint so surrounding parsers
/// see the un-eaten bytes. If the inner decoder consumed bytes and then
/// failed, that progress is committed and the loop stops.
impl<S, T> DecodeValue<S> for Vec<T>
where
    S: winnow::stream::Stream,
    T: DecodeValue<S>,
{
    #[inline(always)]
    fn decode_value(input: &mut S) -> crate::Result<Self> {
        // --------------------------------------------------
        // pre-size the accumulator. growing a `Vec` from empty reallocates several times over a
        // short run (cap 0 -> 4 -> 8 -> ..), which measured as the *entire* gap between this path
        // and a hand-written loop (~2.7x slower on an 8-element run); a single up-front allocation
        // erases it. there can be at most `eof_offset()` elements, since every `decode_value`
        // that pushes consumes >= 1 byte - but that bound is in *bytes*, so cap it by a fixed
        // memory budget (divided by the element size) to keep a large/adversarial input from
        // requesting an enormous allocation up front. capacity only; the decoded contents are
        // byte-for-byte identical to growing from empty.
        // --------------------------------------------------
        const PREALLOC_BYTE_BUDGET: usize = 4096;
        let per_elem = ::core::mem::size_of::<T>().max(1);
        let cap = input
            .eof_offset()
            .min((PREALLOC_BYTE_BUDGET / per_elem).max(1));
        let mut acc = Vec::with_capacity(cap);
        loop {
            let before = input.eof_offset();
            let cp = input.checkpoint();
            if let Ok(val) = T::decode_value(input) {
                acc.push(val);
            } else {
                if input.eof_offset() == before {
                    input.reset(&cp);
                }
                break;
            }
        }
        Ok(acc)
    }
}
