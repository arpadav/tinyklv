// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::ContextError;

// --------------------------------------------------
// local
// --------------------------------------------------
pub use crate::prelude::*;

/// Decodes the value portion of a KLV field from stream-type `S`
///
/// Encode counterpart: [`EncodeValue`](crate::traits::EncodeValue)
///
/// Common examples of stream types include `&[u8]` and `&str`
///
/// Automatically implemented for structs deriving the [`tinyklv::Klv`](crate::Klv) trait which have decoders for every field covered.
///
/// For custom decoding functions, ***no need to use this trait***. Instead, please ensure the functions signature matches the following:
///
/// * fixed length:     `fn <name>(input: &mut S)   -> tinyklv::Result<Self>;`
/// * variable length:  `fn <name>(len: usize)      -> impl Fn(&mut S) -> tinyklv::Result<Self>;`
pub trait DecodeValue<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode_value(input: &mut S) -> winnow::Result<Self>;
}

/// Seeks to the beginning of the prescribed type from a stream using the sentinel
///
/// Encode counterpart: sentinel bytes are prepended by [`EncodeFrame`](crate::traits::EncodeFrame)
///
/// This is automatically implemented when `sentinel` is set in the [`crate::Klv`](crate::Klv) attribute
pub trait SeekSentinel<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn seek_sentinel(input: &mut S) -> winnow::Result<S>;
}

/// Full KLV decode pipeline: [`SeekSentinel`] + [`DecodeValue`]
///
/// Encode counterpart: [`EncodeFrame`](crate::traits::EncodeFrame)
pub trait DecodeFrame<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode_frame(input: &mut S) -> winnow::Result<Self>;
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
    fn decode_frame(input: &mut S) -> winnow::Result<Self> {
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
/// [`DecodeValue::decode_value`] fails to start consuming (clean EOF-style end),
/// or (b) a real mid-stream parser error is encountered. In case (a), returns
/// `Ok(items)` with whatever has been accumulated so far. In case (b), the
/// error is propagated with full context - callers are not left guessing
/// whether an empty `Vec` means "no items" or "parse failed on item N".
pub trait RepeatedDecode<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn repeated(input: &mut S) -> winnow::Result<Vec<Self>>;
}
/// [`RepeatedDecode`] implementation for all types `T` that implement [`DecodeValue`]
impl<S, T> RepeatedDecode<S> for T
where
    T: DecodeValue<S>,
    S: winnow::stream::Stream,
{
    fn repeated(input: &mut S) -> winnow::Result<Vec<Self>> {
        winnow::combinator::repeat(0.., Self::decode_value).parse_next(input)
    }
}

/// Decoding-loop break types
pub enum BreakConditionType {
    /// Do nothing in the decoding loop in [`crate::prelude::DecodeValue::decode_value`].
    ///
    /// This is the default, it just means there is nothing to be done and
    /// continue the decoding loop.
    ///
    /// Is named [`BreakConditionType::Proceed`] to refrain from using the keyword
    /// `continue`, since this does not use the reserved word `continue` and
    /// skip anything in the loop.
    ///
    /// This is equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Proceed => (),
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Proceed,

    /// Skips the current value in [`crate::prelude::DecodeValue::decode_value`]
    ///
    /// This is useful for skipping fields that aren't implemented yet,
    /// by skipping over them and continuing the decoding loop.
    ///
    /// This is equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Skip => {
    ///             take(len).parse_next(input); // error handled properly in macro expansion
    ///             continue;
    ///         },
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Skip,

    /// Returns the decoded value, if all required fields are present.
    ///
    /// This does not guarantee to return [`Ok`] from
    /// [`crate::prelude::DecodeValue::decode_value`], since required fields might not
    /// be present.
    ///
    /// This is useful if some un-recoverable issue has occurred but we
    /// still want to return a partially-parsed result.
    ///
    /// For example, if `len` is decoded to be greater than the maximum
    /// size of the packet, then clearly the packet is malformed and we
    /// should at least try to return what has already been parsed.
    ///
    /// This is equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Done => break,
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// return Ok(/* check if required fields are present */);
    /// ```
    Done,

    /// Returns an error from [`crate::prelude::DecodeValue::decode_value`]
    /// without returning any potential decoded values.
    ///
    /// This is should only be used in cases of a fatal error, since an
    /// [`Err`] is **guaranteed** to return from [`crate::prelude::DecodeValue::decode_value`].
    ///
    /// This is equivalent to:
    ///
    /// ```rust ignore
    /// loop {
    ///     match Self::break_condition(key, len) {
    ///         BreakConditionType::Abort(e) => return Err(e),
    ///         _ => ..., // see `BreakConditionType`
    ///     }
    ///     // decoding logic
    /// }
    /// // return
    /// ```
    Abort(ContextError),
}

/// A trait for breaking during during decoding loop
pub trait BreakCondition<S> {
    #[inline(always)]
    #[allow(unused_variables)]
    fn break_condition<K, L>(decoded_key: K, decoded_len: L) -> BreakConditionType {
        BreakConditionType::Proceed
    }
}
/// [`BreakCondition`] blanket implementation for all types `T` that implement [`DecodeValue`]
impl<T, S> BreakCondition<S> for T
where
    T: crate::traits::dec::DecodeValue<S>,
    S: winnow::stream::Stream,
{
}
