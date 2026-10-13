//! Full KLV decode pipeline traits: sentinel-seeking plus value decode
//!
//! Provides [`DecodeFrame`], which combines [`SeekSentinel`] and [`DecodeValue`]
//! into a single operation that locates the next packet in a byte stream and
//! decodes its body, and [`DrainFrames`], which calls [`DecodeFrame`] repeatedly
//! to collect the packets of a stream into a [`Vec`] until one fails
//!
//! Both traits are blanket-implemented for any `T` that satisfies their
//! respective bounds, so manual impls are almost never needed
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{DecodeValue, SeekSentinel};

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{Parser as _, error::AddContext as _};

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
    /// Once the seek succeeds, `input` is advanced past the sentinel, length prefix, and all body
    /// bytes, whether or not the body then decodes. A body that fails to decode gains the label
    /// `Unable to parse data embedded in packet` after any labels it carries, and the error's
    /// display prints only the first
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to search; advanced past the entire framed packet on success
    ///
    /// # Returns
    ///
    /// `Ok(Self)` when the sentinel is found and the body decodes successfully
    ///
    /// # Errors
    ///
    /// - [`SeekSentinel::seek_sentinel`] finds no complete packet: its error, `input` where the
    ///   seek left it
    /// - The body fails to decode: [`DecodeValue::decode_value`]'s error with the label above,
    ///   `input` past the packet
    fn decode_frame(input: &mut S) -> crate::Result<Self>;
}
/// [`DecodeFrame`] implementation for all types `T` that implement [`SeekSentinel`] and [`DecodeValue`]
///
/// Flow:
///
/// * [`SeekSentinel::seek_sentinel`] locates the sentinel in `input`, consumes sentinel + length + body, and returns the body as a sub-slice (`sought`)
/// * [`DecodeValue::decode_value`] is then run on the sub-slice, so partial consumption during decode does not bleed into the outer `input`
/// * On decode failure, the error gains a trailing label naming the embedded packet
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

/// Decodes KLV frames from a stream until one fails, accumulating them into a [`Vec`]
///
/// Calls [`DecodeFrame::decode_frame`] in a loop. The first call that fails ends the loop,
/// whatever its cause: the end of the input, a missing sentinel, or a malformed body. `input` is
/// rewound to where that call began and the frames before it are returned, so an error is never
/// propagated: a malformed frame, like a cut one, shows only as `input` left at it
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
    /// Stops at the first call that fails, rewinding `input` to where it began
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to drain; advanced past every successfully decoded frame
    ///
    /// # Returns
    ///
    /// `Ok(Vec<Self>)` containing every frame decoded before the first failure
    ///
    /// # Errors
    ///
    /// - The blanket impl returns none: a frame that fails ends the loop instead. Another impl
    ///   may
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
