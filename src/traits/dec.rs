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
    fn decode_value(input: &mut S) -> crate::Result<Self>;
}
/// Automatically implemented for `Vec<T>` where `T` implements [`DecodeValue`].
///
/// Semantics: repeated inner decode until failure. Appropriate for representing
/// a repeated inner field in one parent KLV packet. For streaming a sequence of
/// top-level packets across fragmented reads, use [`crate::Decoder`] instead.
///
/// Cursor safety: if inner `T::decode_value` fails without consuming any bytes,
/// the outer cursor is rewound to the pre-attempt checkpoint so surrounding
/// parsers see the un-eaten bytes. If inner consumed bytes then failed, the
/// progress is committed and the loop stops.
impl<S, T> DecodeValue<S> for Vec<T>
where
    S: winnow::stream::Stream,
    T: DecodeValue<S>,
{
    #[inline(always)]
    fn decode_value(input: &mut S) -> crate::Result<Self> {
        let mut acc = Vec::new();
        loop {
            let before = input.eof_offset();
            let cp = input.checkpoint();
            match T::decode_value(input) {
                Ok(val) => acc.push(val),
                Err(_) => {
                    if input.eof_offset() == before {
                        input.reset(&cp);
                    }
                    break;
                }
            }
        }
        Ok(acc)
    }
}

/// Result of a single [`DecodePartial::decode_partial`] attempt.
///
/// Distinguishes "packet is done and valid" from "not enough bytes yet, try
/// again after feeding more" from "bytes present but malformed." This is the
/// extra signal that one-shot [`DecodeValue`] cannot express.
///
/// * [`Progress::Ready`] - the full value decoded; the input cursor is advanced
///   past the consumed bytes. Caller should commit / drain those bytes.
/// * [`Progress::NeedMore`] - the input is not yet complete. The input cursor
///   has been rewound to the position it held when `decode_partial` was called,
///   so the caller may append more bytes and retry from the same offset.
/// * [`Progress::Malformed`] - the bytes present are not a valid value. The
///   input cursor position is unspecified; callers driving a streaming decode
///   (e.g. [`crate::Decoder`]) are expected to advance past the bad bytes
///   themselves to avoid infinite loops.
pub enum Progress<T> {
    /// Decode succeeded; `T` is the value and `input` has advanced past it.
    Ready(T),
    /// Decode needs more bytes. `input` has been rewound to the pre-attempt
    /// offset; the [`winnow::error::Needed`] hint indicates how many more
    /// bytes are required when known.
    NeedMore(winnow::error::Needed),
    /// Decode failed because the bytes present are malformed.
    Malformed(ContextError),
}

/// Streaming-aware counterpart to [`DecodeValue`]
///
/// Returns a [`Progress<Self>`] instead of a `Result<Self>`, so callers can
/// distinguish "need more bytes" from "malformed." Drives [`crate::Decoder`],
/// the user-facing streaming API
///
/// Automatically implemented for structs deriving [`tinyklv::Klv`](crate::Klv).
/// The stream parameter is `winnow::Partial<&[u8]>` so that a short `take`
/// surfaces as `ErrMode::Incomplete` and can be mapped to
/// [`Progress::NeedMore`] with the input rewound to the pre-attempt offset
pub trait DecodePartial<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode_partial(input: &mut S) -> Progress<Self>;
}
/// [`Vec<T>`] implementation of [`DecodePartial`] for all `T` that implement [`DecodePartial`]
///
/// This parses inner items until one returns [`Progress::NeedMore`]
/// or [`Progress::Malformed`]. A [`Progress::NeedMore`] from the inner parser is
/// committed to the Vec as "done for now" (whatever was accumulated is returned as Ready)
///
/// The cursor has been rewound by the inner call, so the next invocation can resume with
/// more bytes
impl<S, T> DecodePartial<S> for Vec<T>
where
    S: winnow::stream::Stream,
    T: DecodePartial<S>,
{
    fn decode_partial(input: &mut S) -> Progress<Self> {
        let mut acc = Vec::new();
        loop {
            let before = input.eof_offset();
            let cp = input.checkpoint();
            match T::decode_partial(input) {
                Progress::Ready(val) => acc.push(val),
                Progress::NeedMore(_) => return Progress::Ready(acc),
                Progress::Malformed(_) => {
                    if input.eof_offset() == before {
                        input.reset(&cp);
                    }
                    return Progress::Ready(acc);
                }
            }
        }
    }
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
    fn seek_sentinel(input: &mut S) -> crate::Result<S>;
}

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
