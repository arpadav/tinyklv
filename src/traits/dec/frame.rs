// --------------------------------------------------
// local
// --------------------------------------------------
use super::*;

/// Full KLV decode pipeline: [`SeekSentinel`] + [`DecodeValue`]
///
/// Encode counterpart: [`EncodeFrame`](crate::traits::EncodeFrame)
pub trait DecodeFrame<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode_frame(input: &mut S) -> crate::Result<Self>;
}
/// [`DecodeFrame`] implementation for all types `T` that implement [`SeekSentinel`] and [`DecodeValue`]
///
/// Flow:
///
/// * [`SeekSentinel::seek_sentinel`] locates the sentinel in `input`, consumes sentinel + length + body, and returns the body as a sub-slice (`sought`).
/// * [`DecodeValue::decode_value`] is then run on the sub-slice, so partial consumption during decode does not bleed into the outer `input`.
/// * On decode failure, context is attached referencing the outer `input` position for better error messages.
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

/// Decodes repeatedly, accumulating results into a [`Vec`].
///
/// Semantics: decodes zero or more items until either (a) the next call to
/// [`DecodeFrame::decode_frame`] fails to start consuming (clean EOF-style end),
/// or (b) a real mid-stream parser error is encountered. In case (a), returns
/// `Ok(items)` with whatever has been accumulated so far. In case (b), the
/// error is propagated with full context - callers are not left guessing
/// whether an empty `Vec` means "no items" or "parse failed on item N".
pub trait DrainFrames<S>: Sized
where
    S: winnow::stream::Stream,
{
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
