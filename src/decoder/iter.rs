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
use crate::{
    ErrorKind,
    traits::{DecodePartial, DecodeValue, Partial, PartialIterator, ResumePartial, SeekSentinel},
};

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::ContextError;

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
/// [`DecodeIterError`] implementation
impl DecodeIterError {
    /// What a [`DecodeIterError::Malformed`] error is, for a caller to match on instead of its
    /// text
    ///
    /// # Returns
    ///
    /// [`ErrorKind::of`] the malformed packet's error, or `None` for [`DecodeIterError::Eof`] and
    /// [`DecodeIterError::NeedMore`], which the variant already tells apart
    #[must_use]
    pub fn kind(&self) -> Option<ErrorKind> {
        match self {
            Self::Malformed(error) => ErrorKind::of(error),
            Self::Eof | Self::NeedMore => None,
        }
    }
}
/// [`DecodeIterError`] implementation of [`std::error::Error`]
///
/// No `source()` is provided: the `Malformed` cause is a [`winnow::error::ContextError`], which does
/// not implement [`std::error::Error`] under this crate's winnow feature set, so it cannot be
/// surfaced as `&dyn Error`. The context is still available through this error's [`Display`], which
/// formats the inner `ContextError`
///
/// [`Display`]: core::fmt::Display
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
/// `next` call until the buffer is exhausted, a packet is incomplete, or one is
/// malformed
///
/// Stopping iteration keeps every buffered byte that may still begin a packet; call
/// [`Decoder::feed`] to add more data then iterate again to continue decoding
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

    /// Attempts to decode one `T` from the buffered bytes
    ///
    /// Dispatches to resume mode or fresh mode depending on whether an in-flight
    /// partial is present. Resume mode continues a previously interrupted packet
    /// without seeking a sentinel; fresh mode scans for the next sentinel,
    /// reads the framed body length, and decodes the body one-shot
    ///
    /// In resume mode a `NeedMore` result re-stores the updated partial and
    /// returns `None` so the caller can feed more bytes. In fresh mode a packet that has not all
    /// arrived in the buffer yet (the seek fails) retains the bytes that may begin it for a later
    /// feed, while a complete body is decoded one-shot, which finalizes what the body holds: the
    /// value when all required fields are present, otherwise malformed. Malformed results always
    /// advance past the offending bytes to guarantee forward progress
    ///
    /// # Returns
    ///
    /// `Some(T)` when a complete, successfully-decoded value is available;
    /// `None` when the buffer is exhausted, a packet is incomplete, or the
    /// framed bytes could not be parsed
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
/// decoder is expected without moving it
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
    for<'a> T: DecodePartial<&'a [u8], Partial = P>
        + ResumePartial<&'a [u8]>
        + DecodeValue<&'a [u8]>
        + SeekSentinel<&'a [u8]>,
{
    /// Resumes decoding a packet that was previously interrupted with [`crate::decoder::Packet::NeedMore`]
    ///
    /// Pops the in-flight partial from `self.partial`, runs
    /// [`crate::traits::ResumePartial::resume_partial`] against the buffered bytes,
    /// then advances the cursor past the consumed prefix regardless of outcome
    ///
    /// Returns `Ok(T)` when the packet is now complete, or one of:
    /// * [`DecodeIterError::Eof`] - buffer is empty and no partial is set
    /// * [`DecodeIterError::NeedMore`] - the partial made progress but still
    ///   needs more bytes; the updated partial is re-stored in `self.partial`
    /// * [`DecodeIterError::Malformed`] - decoding failed; the cursor advances
    ///   past the consumed bytes to move the stream past the bad data
    fn next_resume(&mut self) -> Result<T, DecodeIterError> {
        // --------------------------------------------------
        // early exit - partial should never be none here
        // --------------------------------------------------
        if self.buf.is_exhausted() && self.partial.is_none() {
            return Err(DecodeIterError::Eof);
        }
        // --------------------------------------------------
        // take the in-flight partial; fall back to default if called without one
        // --------------------------------------------------
        let partial = self.partial.take().unwrap_or_default();
        // --------------------------------------------------
        // pin the pre-decode head; borrow the live byte slice
        // --------------------------------------------------
        let start = self.buf.head();
        let mut cursor: &[u8] = self.buf.pending();
        let before = cursor.len();
        // --------------------------------------------------
        // resume the partial decode
        // --------------------------------------------------
        let packet = <T as ResumePartial<&[u8]>>::resume_partial(&mut cursor, partial);
        // --------------------------------------------------
        // compute consumed bytes and map the error
        // --------------------------------------------------
        let consumed = before - cursor.len();
        let result = packet.map_err(|err| {
            DecodeIterError::Malformed(
                err.to_context_error(self.buf.consumed_slice(start, consumed)),
            )
        });
        // --------------------------------------------------
        // advance past consumed bytes (O(1); no front drain)
        // --------------------------------------------------
        self.buf.advance(consumed);
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
    /// [`crate::traits::DecodeValue::decode_value`] on the framed body
    /// slice. The consumed bytes (framing + body) are advanced past in the buffer
    ///
    /// The body is already framed to an exact length, so it is decoded one-shot rather than
    /// resumed: what it holds is finalized, yielded as `T` when every required field is present,
    /// otherwise surfaced as [`DecodeIterError::Malformed`]
    ///
    /// Returns `Ok(T)` when decoding succeeds, or one of:
    /// * [`DecodeIterError::Eof`] - buffer is empty
    /// * [`DecodeIterError::NeedMore`] - the packet has not all arrived in the buffer yet;
    ///   the cursor advances past the bytes the seek proved cannot begin a packet. A miss
    ///   keeps only the last `sentinel_len - 1` bytes, and a found sentinel whose length or
    ///   body is cut keeps the bytes from that sentinel on
    /// * [`DecodeIterError::Malformed`] - the body decode failed, including a body
    ///   missing a required field
    ///
    /// Stray bytes are scanned once and dropped, but a packet whose declared length never
    /// arrives keeps the buffer growing under repeated `feed` + `NeedMore`. Callers that must
    /// bound memory on hostile input should cap the unproductive `NeedMore` window and
    /// [`Decoder::clear`] to resync rather than feeding indefinitely
    fn next_fresh(&mut self) -> Result<T, DecodeIterError> {
        // --------------------------------------------------
        // early exit
        // --------------------------------------------------
        if self.buf.is_exhausted() {
            return Err(DecodeIterError::Eof);
        }
        // --------------------------------------------------
        // borrow the live byte slice
        // --------------------------------------------------
        let mut cursor: &[u8] = self.buf.pending();
        let before = cursor.len();
        // --------------------------------------------------
        // seek sentinel; a failed seek means the packet is not all in the buffer yet - drop the
        // bytes it proved cannot begin one, retain the rest for a later feed
        // --------------------------------------------------
        let Ok(body) = T::seek_sentinel(&mut cursor) else {
            self.buf.advance(before - cursor.len());
            return Err(DecodeIterError::NeedMore);
        };
        // --------------------------------------------------
        // body is a complete length-delimited frame; decode it one-shot
        // --------------------------------------------------
        let mut body_cursor: &[u8] = body;
        let outcome = T::decode_value(&mut body_cursor);
        // --------------------------------------------------
        // advance past framing + body regardless of outcome
        // --------------------------------------------------
        let consumed = before - cursor.len();
        self.buf.advance(consumed);
        // --------------------------------------------------
        // surface decode error as malformed
        // --------------------------------------------------
        outcome.map_err(DecodeIterError::Malformed)
    }
}
