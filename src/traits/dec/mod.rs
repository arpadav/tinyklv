//! Decode traits for KLV field values, frames, and partial/streaming decode
//!
//! Core decode-side traits:
//! * [`DecodeValue`] - decodes the value portion of a KLV triple from stream `S`
//! * [`DecodeFrame`] / [`DrainFrames`] (re-exported from `frame`) - seek a sentinel and decode
//!   the packet body it frames, once or until one fails
//! * [`DecodePartial`] / [`Partial`] / [`PartialIterator`] / [`ResumePartial`] (re-exported from
//!   `partial`) - support for incremental decode of a packet across multiple byte deliveries, and
//!   [`PartialError`], why one failed
//! * [`SeekSentinel`] (re-exported from `sentinel`) - locate a packet boundary in a
//!   continuous byte stream
//! * [`BreakType`] (re-exported from `breaktype`) - the loop-control outcome a container's or a
//!   field's `#[klv(break_on = ..)]` function yields for the derive-generated decode loop
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
pub use breaktype::BreakType;
pub use frame::{DecodeFrame, DrainFrames};
pub use partial::{DecodePartial, Partial, PartialError, PartialIterator, ResumePartial};
pub use sentinel::SeekSentinel;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The pre-allocation byte budget for decoding value vectors, which [`DecodeValue::NUM_ELEM`]
/// divides by the element size
///
/// TODO: investigate what is best/optimal for pre-sizing? right
/// now arbitrarily set to 4096, which is baked into the binary
const DECODE_VEC_PREALLOC_BYTE_BUDGET: usize = 4096;

/// Decodes the value portion of a KLV triple from stream `S`
///
/// Encode counterpart: [`crate::traits::EncodeValue`]
///
/// The `S` type parameter is the stream type (most commonly `&[u8]` or
/// `&str`). The method advances `input` by exactly the bytes it consumes;
/// on error the cursor position is unspecified (callers should restore a
/// checkpoint if they need to retry)
///
/// This trait is **automatically implemented** for structs deriving
/// [`tinyklv::Klv`](crate::Klv) when every field has an associated decoder
///
/// It is also blanket-implemented for [`Vec<T>`] where `T: DecodeValue<S>`
///
/// For ad-hoc custom decoders passed directly to `#[klv(dec = ...)]`,
/// you do **not** need to implement this trait. Instead, supply a free
/// function with one of the following signatures:
///
/// * A field without `size(lengthed)`: `fn decoder(input: &mut S) -> tinyklv::Result<MyType>`
/// * A `size(lengthed)` field, handed its value length:
///   `fn decoder(len: usize) -> impl Fn(&mut S) -> tinyklv::Result<MyType>`
pub trait DecodeValue<S>: Sized
where
    S: winnow::stream::Stream,
{
    /// The number of elements to pre-allocate, if the [`DecodeValue`] is
    /// implemented for a [`Vec`] type. Otherwise, this is ignored
    const NUM_ELEM: usize = DECODE_VEC_PREALLOC_BYTE_BUDGET / ::core::mem::size_of::<Self>();

    /// Decodes `Self` by consuming bytes from `input`
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to decode from; advanced by the consumed bytes on success
    ///
    /// # Returns
    ///
    /// `Ok(Self)` on success
    ///
    /// # Errors
    ///
    /// - `input` does not hold a valid `Self`: a [`winnow::error::ContextError`] describing the
    ///   failure, the cursor where the impl left it. For a derived struct that is a malformed or
    ///   rejected item, a `break_on` `Abort`, or a required field missing once the loop ends;
    ///   [`crate::Framing`] states which failures are truncation instead
    fn decode_value(input: &mut S) -> crate::Result<Self>;
}

/// [`Vec`] implementation of [`DecodeValue`]
///
/// Decodes repeated `T` values until the inner decoder fails or consumes nothing, collecting
/// successes into a [`Vec`]. Appropriate for representing a repeated inner
/// field within a single parent KLV packet body
///
/// For streaming a sequence of top-level packets across fragmented reads
/// (where a packet may straddle a buffer boundary), use [`crate::Decoder`]
/// instead
///
/// Cursor safety: if the inner decoder consumed bytes and then failed, that
/// progress is committed and the loop stops
///
/// An element that consumed zero bytes is never pushed, whether `T::decode_value`
/// succeeded or failed: the cursor rewinds to the pre-attempt checkpoint, so
/// surrounding parsers see the un-eaten bytes, and the loop stops. Such an element
/// decoded nothing, and the next attempt would see the same input forever. A
/// derived `T` whose fields are all `Option` does this whenever its loop ends
/// without consuming a byte, e.g. on empty input
impl<S, T> DecodeValue<S> for Vec<T>
where
    S: winnow::stream::Stream,
    T: DecodeValue<S>,
{
    /// The number of elements to pre-allocate
    const NUM_ELEM: usize = DECODE_VEC_PREALLOC_BYTE_BUDGET / ::core::mem::size_of::<T>();

    #[inline(always)]
    fn decode_value(input: &mut S) -> crate::Result<Self> {
        // --------------------------------------------------
        // pre-size to remaining bytes capped at a budget to
        // bound adversarial allocation; this is done, since
        // Vec::with_capacity() is significantly faster than
        // Vec::new() when the capacity is known in advance
        // --------------------------------------------------
        let cap = input.eof_offset().min(Self::NUM_ELEM).max(1);
        let mut acc = Vec::with_capacity(cap);
        loop {
            let before = input.eof_offset();
            let cp = input.checkpoint();
            let decoded = T::decode_value(input);
            let consumed = input.eof_offset() != before;
            match decoded {
                Ok(val) if consumed => acc.push(val),
                _ => {
                    if !consumed {
                        input.reset(&cp);
                    }
                    break;
                }
            }
        }
        Ok(acc)
    }
}
