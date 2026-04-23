//! Streaming KLV decoder.
//!
//! [`Decoder<P>`] owns a growing byte buffer and an optional in-flight
//! [`crate::traits::Partial`] state. It yields fully-decoded values of the
//! Partial's `Final` type as enough bytes arrive. It is the user-facing API
//! for parsing KLV from fragmented transports (short TCP reads, ring
//! buffers, UDP re-assembly) where a single logical packet may straddle
//! multiple reads, AND for resuming a packet whose body was cut mid-decode
//! (delivered through [`crate::traits::Progress::NeedMore`]).
//!
//! # Two modes of operation
//!
//! * **Fresh mode** (`partial = None`): [`Decoder::next`] uses
//!   [`crate::traits::SeekSentinel`] to locate each packet boundary and
//!   then runs [`crate::traits::DecodePartial::decode_partial`] on the
//!   framed body. This is the original sentinel-framed streaming flow.
//!
//! * **Resume mode** (`partial = Some(p)`): the decoder was constructed
//!   from a [`Progress::NeedMore`] return inside `decode_partial` and
//!   already carries a half-built packet. [`Decoder::next`] resumes
//!   directly into the existing partial without re-seeking the sentinel,
//!   so previously-decoded fields are not lost.
//!
//! # Framing requirement (fresh mode only)
//!
//! In fresh mode, the final type `T` (`P::Final`) must implement
//! [`crate::traits::SeekSentinel`]. Without a sentinel there is no way to
//! tell one packet's bytes from the next inside a continuous byte stream.
//! Resume mode bypasses the sentinel entirely - the partial is already
//! past the framing.
//!
//! See [`crate::traits::DecodePartial`] for the underlying trait,
//! [`crate::traits::Progress`] for the tri-state signal, and
//! [`crate::traits::Partial`] for the partial-to-final conversion contract.
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::traits::{DecodePartial, Partial, PartialIterator, ResumePartial, SeekSentinel};

// --------------------------------------------------
// external
// --------------------------------------------------
use core::marker::PhantomData;
use winnow::error::ContextError;

#[derive(Debug)]
/// Result of a single [`DecodePartial::decode_partial`] attempt
///
/// Two states only:
///
/// * [`Progress::Ready`] - the partial finalised cleanly into `T`. Input
///   cursor is advanced past the consumed bytes.
/// * [`Progress::NeedMore`] - the decode loop paused because more input
///   is needed. The partial holds every klv field that landed so far;
///   feed it back into the loop (via [`crate::Decoder::feed`] +
///   [`crate::Decoder::next`], or a manual `resume_partial` call) once
///   more bytes are available
pub enum Progress<T, P>
where
    P: Partial<Final = T>,
{
    /// Decode succeeded; `T` is the value and `input` has advanced past it
    Ready(T),

    /// Decode needs more bytes. The partial holds every klv field
    /// landed so far. Hand it back to [`ResumePartial::resume_partial`]
    /// (or wrap into a [`crate::Decoder`] for buffered streaming) once
    /// more bytes are available
    NeedMore(P),
}

#[derive(Debug)]
/// Boundary error type surfaced by [`Decoder::next`].
///
/// Internal [`Progress::NeedMore`] is not surfaced; it is represented as
/// `next() -> None`. Only [`Progress::Malformed`] becomes an `Err`.
pub enum DecodeError {
    /// The bytes present in the buffer could not be parsed as the target
    /// type. The [`Decoder`] has already advanced past the offending bytes
    /// (drained the framed packet, or the consumed prefix in resume mode)
    /// to guarantee forward progress on subsequent calls.
    Malformed(ContextError),
}
impl std::error::Error for DecodeError {}
impl core::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DecodeError::Malformed(e) => write!(f, "malformed KLV: {e}"),
        }
    }
}

/// Owned-buffer streaming decoder, parameterised on the [`Partial`] type.
///
/// Accepts bytes via [`feed`](Self::feed) and yields decoded values via
/// [`next`](Self::next). A `None` from `next` means "need more bytes, call
/// `feed` again." A `Some(Err)` means the current position is malformed;
/// the decoder has already advanced past the bad bytes to avoid looping.
///
/// # Example
///
/// ```ignore
/// // fresh mode (no in-flight partial): seek sentinel + decode framed body
/// let mut dec = Decoder::<<MyPacket as DecodePartial<&[u8]>>::Partial>::new();
/// let mut scratch = [0u8; 2048];
/// loop {
///     let n = socket.recv(&mut scratch)?;
///     dec.feed(&scratch[..n]);
///     while let Some(pkt) = dec.next() {
///         handle(pkt?);
///     }
/// }
///
/// // resume mode: handed back from `Progress::NeedMore`
/// match MyPacket::decode_partial(&mut input) {
///     Progress::Ready(t) => use_packet(t),
///     Progress::NeedMore(mut dec) => {
///         dec.feed(more_bytes);
///         while let Some(pkt) = dec.next() {
///             use_packet(pkt?);
///             break;
///         }
///     }
///     Progress::Malformed(e) => report(e),
/// }
/// ```
pub struct Decoder<P, S> {
    buf: Vec<u8>,
    partial: Option<P>,
    _marker: PhantomData<(P, S)>,
}
/// [`Decoder`] implementation of [`Default`]
impl<P, S> Default for Decoder<P, S> {
    #[inline(always)]
    fn default() -> Self {
        Self::new()
    }
}
/// [`Decoder`] implementation
impl<P, S> Decoder<P, S> {
    #[inline(always)]
    /// Construct an empty decoder in fresh mode (no in-flight partial).
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            partial: None,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    /// Construct an empty decoder with a pre-allocated buffer capacity, in
    /// fresh mode.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
            partial: None,
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    /// Append bytes to the internal buffer. Does not trigger decoding on its
    /// own - call [`next`](Self::next) to try to consume complete values.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    #[inline(always)]
    /// The bytes currently buffered but not yet decoded. Useful for
    /// observability and diagnostics.
    pub fn buffered(&self) -> &[u8] {
        &self.buf
    }

    #[inline(always)]
    /// Borrow the in-flight partial, if any. `Some` only after the decoder
    /// was constructed from a [`Progress::NeedMore`] return and before the
    /// next [`Self::next`] completes the packet.
    pub fn partial(&self) -> Option<&P> {
        self.partial.as_ref()
    }

    #[inline(always)]
    /// Consume the decoder and return its in-flight partial, if any.
    /// Used by the derive-generated `decode_partial` wrapper to lift
    /// "input fully consumed, NeedMore returned" into a finalisation
    /// at the fresh-mode boundary.
    pub fn into_partial(self) -> Option<P> {
        self.partial
    }

    #[inline(always)]
    /// Drop all buffered bytes and any in-flight partial. Call after a
    /// `Malformed` error if you want to resync cleanly rather than
    /// byte-walk through corrupt input.
    pub fn clear(&mut self) {
        self.buf.clear();
        self.partial = None;
    }
}
/// [`Decoder`] for `&[u8]` implementation of [`Iterator`]
impl<P, T, S> Iterator for Decoder<P, S>
where
    Self: PartialIterator<P>,
    S: winnow::stream::Stream,
    P: Partial<Final = T> + Default,
    for<'a> T: DecodePartial<S, Partial = P> + ResumePartial<S> + SeekSentinel<S>,
{
    type Item = Result<T, DecodeError>;

    /// Attempt to decode one `T` from the buffered bytes.
    ///
    /// # Flow
    ///
    /// * **Resume mode** (`self.partial.is_some()`): pull the partial out,
    ///   run `T::decode_partial` against the buffered bytes (the partial
    ///   is moved into the call via the derive-generated resume entry).
    ///   - `Ready(t)`: drain consumed bytes, return `Some(Ok(t))`.
    ///   - `NeedMore(new_dec)`: adopt `new_dec`'s buffered tail and
    ///     partial; return `None` so the caller can `feed` more.
    ///   - `Malformed(e)`: drain consumed bytes, return
    ///     `Some(Err(Malformed(e)))`.
    ///
    /// * **Fresh mode** (`self.partial.is_none()`): scan for the sentinel
    ///   + length, run `decode_partial` on the framed body. Same
    ///     shortfall semantics as before: a NeedMore from a body whose
    ///     length was already declared is malformed.
    fn next(&mut self) -> Option<Self::Item> {
        if self.partial.is_some() {
            return self.next_resume();
        }
        self.next_fresh()
    }
}
/// [`Decoder`] decoding implementation - resume mode
///
/// This is just the &[u8] specialization of [`Decoder`]
///
/// When the decoder carries an in-flight partial, [`next`](Self::next)
/// resumes [`DecodePartial::decode_partial`] without re-seeking the
/// sentinel: the partial state already lives past the framing.
impl<P, T> Decoder<P, &[u8]>
where
    P: Partial<Final = T> + Default,
    for<'a> T:
        DecodePartial<&'a [u8], Partial = P> + ResumePartial<&'a [u8]> + SeekSentinel<&'a [u8]>,
{
    /// Force-finalise the current partial. Use when the upstream signals
    /// "no more bytes coming" and you want the accumulated partial
    /// surfaced (or its required-field failure reported). The decoder
    /// is consumed; the buffered bytes are discarded.
    pub fn finish(self) -> Result<T, DecodeError> {
        let partial = self.partial.unwrap_or_default();
        partial.finalize().map_err(|label| {
            // --------------------------------------------------
            // no live input at finish time. wrap the label with an
            // empty input slice so the ContextError carries the
            // label cleanly without fabricating false position info.
            // --------------------------------------------------
            DecodeError::Malformed(label_to_context_error(&[], label))
        })
    }
}
impl<P, T> PartialIterator<P> for Decoder<P, &[u8]>
where
    P: Partial<Final = T> + Default,
    for<'a> T:
        DecodePartial<&'a [u8], Partial = P> + ResumePartial<&'a [u8]> + SeekSentinel<&'a [u8]>,
{
    /// Resume-mode entry. Consumes the in-flight partial, runs the
    /// derive-generated resume path against `self.buf`, and reconciles
    /// the resulting `Result<Progress, &'static str>` with `self`'s
    /// buffer/partial state.
    ///
    /// This is also the single place that wraps a codegen-emitted
    /// `&'static str` label into a [`ContextError`] - the codegen has
    /// no live input at hand, so it propagates the bare label and
    /// `Decoder` (which DOES have `self.buf`) builds the rich error.
    fn next_resume(&mut self) -> Option<Result<T, DecodeError>> {
        // --------------------------------------------------
        // outcome carries owned data only - no borrow of self.buf
        // escapes the inner block, so the subsequent self.buf.drain(..)
        // call is unborrowed.
        // --------------------------------------------------
        enum Outcome<T, P> {
            Ready(T, usize),
            Label(&'static str, usize),
            NeedMore(P, usize),
        }
        let partial = self.partial.take().unwrap_or_default();
        let outcome = {
            let mut cursor: &[u8] = self.buf.as_slice();
            let before = cursor.len();
            // --------------------------------------------------
            // re-seed the partial into the resume entry. the derive
            // macro emits `ResumePartial<&[u8]>` alongside `DecodePartial`,
            // so this dispatch is direct with no free fn in between.
            // `ResumePartial<S>` itself is generic over `S` - callers
            // with non-byte streams bypass this `Decoder` entirely and
            // invoke the trait directly with their own buffering.
            // --------------------------------------------------
            let progress = <T as ResumePartial<&[u8]>>::resume_partial(&mut cursor, partial);
            let consumed = before - cursor.len();
            match progress {
                Ok(Progress::Ready(t)) => Outcome::Ready(t, consumed),
                Ok(Progress::NeedMore(p)) => Outcome::NeedMore(p, consumed),
                Err(label) => Outcome::Label(label, consumed),
            }
        };
        match outcome {
            Outcome::Ready(t, consumed) => {
                self.buf.drain(..consumed);
                Some(Ok(t))
            }
            Outcome::Label(label, consumed) => {
                // --------------------------------------------------
                // unrecoverable + finalize failed. wrap the bare
                // label into a ContextError using the consumed
                // prefix as the input position. drain the bad
                // bytes so subsequent calls do not re-attempt.
                // --------------------------------------------------
                let ce = label_to_context_error(&self.buf[..consumed], label);
                self.buf.drain(..consumed);
                Some(Err(DecodeError::Malformed(ce)))
            }
            Outcome::NeedMore(p, consumed) => {
                // --------------------------------------------------
                // truncation: the consumed prefix has already been
                // committed into the partial. drain it so subsequent
                // feed/next continues from the unconsumed tail.
                // --------------------------------------------------
                self.buf.drain(..consumed);
                self.partial = Some(p);
                None
            }
        }
    }

    /// Fresh-mode entry. Existing sentinel-framed flow, unchanged in
    /// shape: scan sentinel, take declared body length, run
    /// `decode_partial` on the body slice.
    ///
    /// In sentinel-framed mode the body slice is bounded by the
    /// declared length, so `decode_partial` should produce
    /// `Ok(Progress::Ready)` or an `Err(label)` (unrecoverable +
    /// finalize-fail). A `NeedMore` here means the declared length
    /// was longer than the body could supply for the inner KLV
    /// stream - finalise via `Partial::finalize` and route any
    /// label through the same wrap path as the resume mode.
    fn next_fresh(&mut self) -> Option<Result<T, DecodeError>> {
        if self.buf.is_empty() {
            return None;
        }
        enum Outcome<T> {
            Ready(T, usize),
            Label(&'static str, usize),
        }
        let outcome = {
            let mut scan: &[u8] = self.buf.as_slice();
            let before = scan.len();
            let body = match T::seek_sentinel(&mut scan) {
                Ok(b) => b,
                Err(_) => return None,
            };
            let mut body_cursor: &[u8] = body;
            let framed_consumed = before - scan.len();
            match T::decode_partial(&mut body_cursor) {
                Ok(Progress::Ready(t)) => Outcome::Ready(t, framed_consumed),
                Err(label) => Outcome::Label(label, framed_consumed),
                // --------------------------------------------------
                // body had the declared length but inner could not
                // complete cleanly. attempt finalize on the partial
                // - on success surface as Ready, on failure forward
                // the resulting label through the standard wrap.
                // --------------------------------------------------
                Ok(Progress::NeedMore(p)) => match p.finalize() {
                    Ok(t) => Outcome::Ready(t, framed_consumed),
                    Err(label) => Outcome::Label(label, framed_consumed),
                },
            }
        };

        match outcome {
            Outcome::Ready(t, n) => {
                self.buf.drain(..n);
                Some(Ok(t))
            }
            Outcome::Label(label, n) => {
                let ce = label_to_context_error(&self.buf[..n], label);
                self.buf.drain(..n);
                Some(Err(DecodeError::Malformed(ce)))
            }
        }
    }
}

/// Wrap a `&'static str` label into a [`ContextError`] using the
/// supplied input slice as the position context. Single helper used by
/// both [`Decoder::next_resume`], [`Decoder::next_fresh`], and
/// [`Decoder::finish`] so the label-to-error translation lives in one
/// place.
fn label_to_context_error(input: &[u8], label: &'static str) -> ContextError {
    use winnow::stream::Stream as _;
    let cp = input.checkpoint();
    winnow::error::AddContext::add_context(
        ContextError::new(),
        &input,
        &cp,
        winnow::error::StrContext::Label(label),
    )
}
