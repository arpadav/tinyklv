//! Iterator adapters and the [`DecodeIterError`] type for [`crate::decoder::Decoder`]
//!
//! Contains:
//! * [`DecodeIterError`] - the error type surfaced when a decode step in the
//!   iterator loop cannot complete (EOF, need-more-bytes, or malformed data)
//! * [`DecoderIter`] - a borrowing iterator over a [`crate::decoder::Decoder`]
//!   that yields fully-decoded values
//! * [`Iterator`] impl for [`DecoderIter`]
//! * [`IntoIterator`] impl for `&mut Decoder`
//! * [`crate::traits::PartialIterator`] impls for both owned and `&mut`
//!   [`crate::decoder::Decoder`] specialised to `&[u8]` streams
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{Decoder, Packet};
use crate::traits::{DecodePartial, Partial, PartialIterator, ResumePartial, SeekSentinel};

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::ContextError;

/// Wraps a `&'static str` label into a [`ContextError`] anchored at the supplied input slice
///
/// Single construction path used by [`Decoder::next_resume`], [`Decoder::next_fresh`],
/// and [`Decoder::finish`] so that label-to-error translation is consistent
/// and does not repeat boilerplate at each call site. The `$input` expression
/// supplies the position context for winnow's error reporting machinery.
macro_rules! label_to_context_error {
    ($input:expr, $label:expr) => {{
        use winnow::stream::Stream as _;
        let cp = $input.checkpoint();
        winnow::error::AddContext::add_context(
            winnow::error::ContextError::new(),
            &$input,
            &cp,
            winnow::error::StrContext::Label($label),
        )
    }};
}

#[derive(Debug)]
#[non_exhaustive]
/// Error type returned by [`Decoder::next`] and the [`crate::traits::PartialIterator`] methods
///
/// All three variants cause the iterator loop to yield `None` (or for the
/// methods to return `Err`). The caller's response differs per variant:
///
/// * `Eof` - no bytes remain; call [`Decoder::feed`] with new data or stop the loop
/// * `NeedMore` - bytes remain but the current packet is incomplete; call
///   [`Decoder::feed`] with additional bytes then resume iteration
/// * `Malformed` - the framed bytes could not be decoded; the [`Decoder`]
///   has already drained the offending bytes to guarantee forward progress,
///   so iteration may safely continue after this error
pub enum DecodeIterError {
    /// The buffer is empty and no more bytes are available to decode
    Eof,

    /// The current packet is incomplete; more bytes must be fed before decoding can finish
    NeedMore,

    /// The bytes present in the buffer could not be parsed as the target type
    ///
    /// The [`Decoder`] has already advanced past the offending bytes
    /// (drained the framed packet, or the consumed prefix in resume mode)
    /// to guarantee forward progress on subsequent calls
    Malformed(ContextError),
}
/// [`DecodeIterError`] implementation of [`std::error::Error`]
impl std::error::Error for DecodeIterError {}
/// [`DecodeIterError`] implementation of [`core::fmt::Display`]
impl core::fmt::Display for DecodeIterError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeIterError::Eof => write!(f, "EOF"),
            DecodeIterError::NeedMore => write!(f, "need more bytes"),
            DecodeIterError::Malformed(e) => write!(f, "malformed KLV: {e}"),
        }
    }
}

/// A borrowing iterator adapter that yields decoded values from a [`Decoder`]
///
/// Created by [`Decoder::iter`] or via the [`IntoIterator`] impl on
/// `&mut Decoder`. Yields `T` (the `Partial::Final` associated type) on each
/// `next` call until the buffer is exhausted or a packet is incomplete.
///
/// Stopping iteration does not discard buffered bytes; call [`Decoder::feed`]
/// to add more data then iterate again to continue decoding.
pub struct DecoderIter<'a, P, S> {
    pub(super) dec: &'a mut Decoder<P, S>,
}
/// [`DecoderIter`] implementation of [`Iterator`]
impl<P, T, S> Iterator for DecoderIter<'_, P, S>
where
    S: winnow::stream::Stream,
    P: Partial<Final = T> + Default,
    Decoder<P, S>: PartialIterator<P>,
{
    type Item = T;

    /// Attempt to decode one `T` from the buffered bytes.
    ///
    /// # Flow
    ///
    /// * **Resume mode** (`self.partial.is_some()`): pull the partial out,
    ///   run `T::decode_partial` against the buffered bytes (the partial
    ///   is moved into the call via the derive-generated resume entry).
    ///   - `Ready(t)`: drain consumed bytes, return `Some(t)`.
    ///   - `NeedMore(new_dec)`: adopt `new_dec`'s buffered tail and
    ///     partial; return `None` so the caller can `feed` more
    ///   - `Malformed(e)`: drain consumed bytes, return
    ///     `None`
    ///
    /// * **Fresh mode** (`self.partial.is_none()`): scan for the sentinel
    ///   + length, run `decode_partial` on the framed body. Same
    ///     shortfall semantics as before: a `NeedMore` from a body whose
    ///     length was already declared is malformed
    fn next(&mut self) -> Option<Self::Item> {
        if self.dec.partial.is_some() {
            return self.dec.next_resume().ok();
        }
        self.dec.next_fresh().ok()
    }
}
/// [`Decoder`] implementation of [`IntoIterator`]
impl<'d, P, T, S> IntoIterator for &'d mut Decoder<P, S>
where
    Decoder<P, S>: PartialIterator<P>,
    S: winnow::stream::Stream,
    P: Partial<Final = T> + Default,
    for<'b> T: DecodePartial<S, Partial = P> + ResumePartial<S> + SeekSentinel<S>,
{
    type Item = T;
    type IntoIter = DecoderIter<'d, P, S>;
    fn into_iter(self) -> Self::IntoIter {
        DecoderIter { dec: self }
    }
}

/// `&mut` [`Decoder`] implementation of [`PartialIterator`] for [`&[u8]`]
///
/// Delegates both methods to the owned [`Decoder`] impl via double-deref so
/// that a mutable reference to a decoder can be used anywhere an owned
/// decoder is expected without moving it.
impl<'a, P, T> PartialIterator<P> for &mut Decoder<P, &'a [u8]>
where
    P: Partial<Final = T> + Default,
    Decoder<P, &'a [u8]>: PartialIterator<P>,
{
    #[inline(always)]
    fn next_resume(&mut self) -> Result<<P as Partial>::Final, DecodeIterError> {
        (**self).next_resume()
    }

    #[inline(always)]
    fn next_fresh(&mut self) -> Result<<P as Partial>::Final, DecodeIterError> {
        (**self).next_fresh()
    }
}

/// [`Decoder`] implementation of [`PartialIterator`] for [`&[u8]`]
impl<P, T> PartialIterator<P> for Decoder<P, &[u8]>
where
    P: Partial<Final = T> + Default,
    for<'a> T:
        DecodePartial<&'a [u8], Partial = P> + ResumePartial<&'a [u8]> + SeekSentinel<&'a [u8]>,
{
    /// Resumes decoding a packet that was previously interrupted with [`crate::decoder::Packet::NeedMore`]
    ///
    /// Pops the in-flight partial from `self.partial`, runs
    /// [`crate::traits::ResumePartial::resume_partial`] against the buffered bytes,
    /// then drains the consumed prefix from the buffer regardless of outcome.
    ///
    /// Returns `Ok(T)` when the packet is now complete, or one of:
    /// * [`DecodeIterError::Eof`] - buffer is empty and no partial is set
    /// * [`DecodeIterError::NeedMore`] - the partial made progress but still
    ///   needs more bytes; the updated partial is re-stored in `self.partial`
    /// * [`DecodeIterError::Malformed`] - decoding failed; the consumed bytes
    ///   are drained to advance the stream past the bad data
    fn next_resume(&mut self) -> Result<T, DecodeIterError> {
        // --------------------------------------------------
        // early exit - partial should never be none here
        // --------------------------------------------------
        if self.buf.is_empty() && self.partial.is_none() {
            return Err(DecodeIterError::Eof);
        }
        // --------------------------------------------------
        // get partial - could be initialized if not already
        // to populate it. however, the partial should always
        // be some
        // --------------------------------------------------
        let partial = self.partial.take().unwrap_or_default();
        // --------------------------------------------------
        // get outcome, and how much consumed
        // --------------------------------------------------
        let mut cursor: &[u8] = self.buf.as_slice();
        let before = cursor.len();
        // --------------------------------------------------
        // re-seed the partial into the resume entry
        // --------------------------------------------------
        let packet = <T as ResumePartial<&[u8]>>::resume_partial(&mut cursor, partial);
        // --------------------------------------------------
        // get consumed + outcome
        // --------------------------------------------------
        let consumed = before - cursor.len();
        let result = packet.map_err(|label| {
            let ce = label_to_context_error!(&self.buf[..consumed], label);
            DecodeIterError::Malformed(ce)
        });
        // --------------------------------------------------
        // continue onwards
        // --------------------------------------------------
        self.buf.drain(..consumed);
        // --------------------------------------------------
        // coerce and return
        // --------------------------------------------------
        match result {
            Ok(Packet::Ready(pkt)) => Ok(pkt),
            Ok(Packet::NeedMore(p)) => {
                self.partial = Some(p);
                Err(DecodeIterError::NeedMore)
            }
            Err(e) => Err(e),
        }
    }

    /// Seeks the next sentinel boundary and decodes the framed packet body
    ///
    /// Runs [`crate::traits::SeekSentinel::seek_sentinel`] to locate and
    /// consume the packet framing (sentinel + length prefix), then calls
    /// [`crate::traits::DecodePartial::decode_partial`] on the framed body
    /// slice. The consumed bytes (framing + body) are drained from the buffer.
    ///
    /// A `NeedMore` result from `decode_partial` inside fresh mode means the
    /// framing declared a body length that the parser did not fully consume,
    /// which is treated as malformed rather than resumable (the body was already
    /// framed to an exact length).
    ///
    /// Returns `Ok(T)` when decoding succeeds, or one of:
    /// * [`DecodeIterError::Eof`] - buffer is empty
    /// * [`DecodeIterError::Malformed`] - sentinel seek or body decode failed
    fn next_fresh(&mut self) -> Result<T, DecodeIterError> {
        // --------------------------------------------------
        // early exit
        // --------------------------------------------------
        if self.buf.is_empty() {
            return Err(DecodeIterError::Eof);
        }
        let mut cursor: &[u8] = self.buf.as_slice();
        let before = cursor.len();
        // --------------------------------------------------
        // seek sentinel, get body cursor
        // --------------------------------------------------
        let body = match T::seek_sentinel(&mut cursor) {
            Ok(b) => b,
            Err(e) => return Err(DecodeIterError::Malformed(e)),
        };
        let mut body_cursor: &[u8] = body;
        let packet = T::decode_partial(&mut body_cursor);
        // --------------------------------------------------
        // get consumed + outcome
        // --------------------------------------------------
        let consumed = before - cursor.len();
        let result = packet.map_err(|label| {
            let ce = label_to_context_error!(&self.buf[..consumed], label);
            DecodeIterError::Malformed(ce)
        });
        // --------------------------------------------------
        // continue onwards
        // --------------------------------------------------
        self.buf.drain(..consumed);
        // --------------------------------------------------
        // coerce and return
        // --------------------------------------------------
        match result {
            Ok(Packet::Ready(pkt)) => Ok(pkt),
            Ok(Packet::NeedMore(p)) => p.finalize().map_err(|label| {
                let ce = label_to_context_error!(&self.buf[..consumed], label);
                DecodeIterError::Malformed(ce)
            }),
            Err(e) => Err(e),
        }
    }
}

// /// A consuming iterator that decodes values as they are encountered
// pub struct ConsumeIter<'a, P, S, I, B, T>
// where
//     I: Iterator<Item = B>,
//     B: AsRef<[u8]> + 'a,
// {
//     pub(super) parser: DecoderIter<'a, P, S>,
//     pub(super) input: I,
//     pub(super) _marker: std::marker::PhantomData<(P, S, T)>,
// }

// /// [`ConsumeIter`] implementation of [`Iterator`]
// impl<'a, P, S, I, B, T> Iterator for ConsumeIter<'a, P, S, I, B, T>
// where
//     I: Iterator<Item = B>,
//     B: AsRef<[u8]> + 'a,
//     S: winnow::stream::Stream,
//     P: Partial<Final = T> + Default,
//     Decoder<P, S>: Feed + PartialIterator<P>,
// {
//     type Item = T;
//     fn next(&mut self) -> Option<Self::Item> {
//         loop {
//             if let Some(v) = self.parser.next() {
//                 return Some(v);
//             }
//             let next = self.input.next()?;
//             self.parser.feed(next.as_ref());
//         }
//     }
// }

// /// A trait to feed bytes to some decoder
// pub trait Feed {
//     fn feed(&mut self, input: &[u8]);
// }
// /// [`Feed`] implementation for [`Decoder`]
// impl<P, S> Feed for Decoder<P, S> {
//     fn feed(&mut self, input: &[u8]) {
//         self.feed(input)
//     }
// }
// /// [`Feed`] implementation for [`DecoderIter`]
// impl<P, S> Feed for DecoderIter<'_, P, S> {
//     fn feed(&mut self, input: &[u8]) {
//         self.dec.feed(input)
//     }
// }
