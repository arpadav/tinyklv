//! Full KLV decode pipeline traits: sentinel-seeking plus value decode
//!
//! Provides [`DecodeFrame`], which combines [`SeekSentinel`] and [`DecodeValue`]
//! into a single operation that locates the next packet in a byte stream and
//! decodes its body, and [`DrainFrames`], which calls [`DecodeFrame`] repeatedly
//! to collect every packet from a stream into a [`Vec`]
//!
//! Both traits are blanket-implemented for any `T` that satisfies their
//! respective bounds, so manual impls are almost never needed
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::*;

/// Full KLV decode pipeline: seeks the sentinel boundary then decodes the framed body
///
/// Encode counterpart: [`EncodeFrame`](crate::traits::EncodeFrame)
///
/// Combines [`SeekSentinel`] and [`DecodeValue`] into a single operation:
/// the sentinel is located and consumed (along with the length prefix and body
/// bytes), and then [`DecodeValue::decode_value`] is run on the isolated body
/// slice. Because decoding runs on a sub-slice, partial consumption during
/// [`DecodeValue::decode_value`] does not advance the outer `input` cursor
///
/// A blanket impl covers every `T: SeekSentinel<S> + DecodeValue<S>`, so this
/// trait is automatically available for all derive-generated KLV structs
///
/// # Example
///
/// ```rust
/// use tinyklv::prelude::*;
/// use tinyklv::Klv;
///
/// #[derive(Debug, PartialEq, Klv)]
/// #[klv(
///     stream = &[u8],
///     sentinel = b"\xAA",
///     key(enc = tinyklv::codecs::binary::enc::u8, dec = tinyklv::codecs::binary::dec::u8),
///     len(enc = tinyklv::codecs::binary::enc::u8_from_usize, dec = tinyklv::codecs::binary::dec::u8_as_usize),
/// )]
/// struct SensorReading {
///     #[klv(
///         key = 0x01,
///         dec = tinyklv::codecs::binary::dec::be_u16,
///         enc = *tinyklv::codecs::binary::enc::be_u16,
///     )]
///     temperature: u16,
/// }
///
/// let bytes: &[u8] = &[
///     0xAA, // sentinel
///     0x04, // body length
///     0x01, // key
///     0x02, // value length
///     0x00, 0x64, // value: 100
/// ];
/// let mut input = bytes;
/// let result = SensorReading::decode_frame(&mut input).unwrap();
/// assert_eq!(result.temperature, 100);
/// ```
pub trait DecodeFrame<S>: Sized
where
    S: winnow::stream::Stream,
{
    /// Seeks the sentinel boundary in `input` and decodes the framed packet body
    ///
    /// On success, `input` is advanced past the sentinel, length prefix, and
    /// all body bytes. On failure, context is attached referencing the outer
    /// `input` position so that errors report the stream offset of the packet,
    /// not the offset within the isolated body slice
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to search; advanced past the entire framed packet on success
    ///
    /// # Returns
    ///
    /// `Ok(Self)` when the sentinel is found and the body decodes successfully,
    /// or a [`winnow::error::ContextError`] if either step fails
    fn decode_frame(input: &mut S) -> crate::Result<Self>;
}
/// [`DecodeFrame`] implementation for all types `T` that implement [`SeekSentinel`] and [`DecodeValue`]
///
/// Flow:
///
/// * [`SeekSentinel::seek_sentinel`] locates the sentinel in `input`, consumes sentinel + length + body, and returns the body as a sub-slice (`sought`)
/// * [`DecodeValue::decode_value`] is then run on the sub-slice, so partial consumption during decode does not bleed into the outer `input`
/// * On decode failure, context is attached referencing the outer `input` position for better error messages
impl<S, T> DecodeFrame<S> for T
where
    S: winnow::stream::Stream,
    T: SeekSentinel<S> + DecodeValue<S>,
{
    fn decode_frame(input: &mut S) -> crate::Result<Self> {
        let mut sought = T::seek_sentinel.parse_next(input)?;
        let checkpoint = input.checkpoint();
        Self::decode_value(&mut sought).map_err(|e| {
            e.add_context(
                input,
                &checkpoint,
                winnow::error::StrContext::Label("Unable to parse data embedded in packet"),
            )
        })
    }
}

/// Decodes all available KLV frames from a stream, accumulating them into a [`Vec`]
///
/// Calls [`DecodeFrame::decode_frame`] in a loop until the stream is exhausted
/// or a clean end-of-input is reached. The two termination cases are:
///
/// * (a) The next [`DecodeFrame::decode_frame`] call fails without consuming any
///   bytes - this is an EOF-style end and `Ok(items)` is returned with whatever
///   was accumulated
/// * (b) A real mid-stream parser error is encountered - the error is propagated
///   with full context so callers are not left guessing whether an empty [`Vec`]
///   means "no items" or "parse failed on item N"
///
/// A blanket impl covers every `T: DecodeFrame<S>`
///
/// # Example
///
/// ```rust
/// use tinyklv::prelude::*;
/// use tinyklv::Klv;
///
/// #[derive(Debug, PartialEq, Klv)]
/// #[klv(
///     stream = &[u8],
///     sentinel = b"\xBB",
///     key(enc = tinyklv::codecs::binary::enc::u8, dec = tinyklv::codecs::binary::dec::u8),
///     len(enc = tinyklv::codecs::binary::enc::u8_from_usize, dec = tinyklv::codecs::binary::dec::u8_as_usize),
/// )]
/// struct Tag {
///     #[klv(
///         key = 0x01,
///         dec = tinyklv::codecs::binary::dec::u8,
///         enc = *tinyklv::codecs::binary::enc::u8,
///     )]
///     value: u8,
/// }
///
/// // two back-to-back KLV packets
/// let bytes: &[u8] = &[
///     0xBB, 0x03, 0x01, 0x01, 0x0A, // packet 1: sentinel, len, key, vlen, value=10
///     0xBB, 0x03, 0x01, 0x01, 0x14, // packet 2: sentinel, len, key, vlen, value=20
/// ];
/// let mut input = bytes;
/// let tags = Tag::drain_frames(&mut input).unwrap();
/// assert_eq!(tags.len(), 2);
/// assert_eq!(tags[0].value, 10);
/// assert_eq!(tags[1].value, 20);
/// ```
pub trait DrainFrames<S>: Sized
where
    S: winnow::stream::Stream,
{
    /// Repeatedly calls [`DecodeFrame::decode_frame`] and collects results into a [`Vec`]
    ///
    /// Returns `Ok(Vec<Self>)` once the stream is cleanly exhausted, or
    /// propagates the first mid-stream error encountered
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to drain; advanced past every successfully decoded frame
    ///
    /// # Returns
    ///
    /// `Ok(Vec<Self>)` containing all decoded frames, or a [`winnow::error::ContextError`]
    /// if a parse error occurs mid-stream
    fn drain_frames(input: &mut S) -> crate::Result<Vec<Self>>;
}
/// [`DrainFrames`] implementation for all types `T` that implement [`DecodeFrame`]
impl<S, T> DrainFrames<S> for T
where
    T: DecodeFrame<S>,
    S: winnow::stream::Stream,
{
    fn drain_frames(input: &mut S) -> crate::Result<Vec<Self>> {
        winnow::combinator::repeat(0.., Self::decode_frame).parse_next(input)
    }
}
