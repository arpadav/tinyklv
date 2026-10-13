//! Streaming KLV decoder
//!
//! [`Decoder<P>`] owns a growing byte buffer and an optional in-flight
//! [`crate::traits::Partial`] state. It yields fully-decoded values of the
//! Partial's `Final` type as enough bytes arrive. It is the user-facing API
//! for parsing KLV from fragmented transports (short TCP reads, ring
//! buffers, UDP re-assembly) where a single logical packet may straddle
//! multiple reads
//!
//! # Two modes of operation
//!
//! * **Fresh mode** (no partial in flight): [`Decoder::next`] uses
//!   [`crate::traits::SeekSentinel`] to locate each packet boundary and
//!   then decodes the framed body one-shot with
//!   [`crate::traits::DecodeValue::decode_value`]. A packet not yet all in
//!   the buffer stays there until a later [`Decoder::feed`] completes it
//!
//! * **Resume mode** (a partial in flight): entered only when
//!   [`crate::traits::PartialIterator::next_resume`], called directly,
//!   decodes the buffered bytes as a body with no sentinel and runs out of
//!   them, keeping the half-built packet. [`Decoder::next`] then resumes
//!   directly into that partial without seeking a sentinel, so
//!   previously-decoded fields are not lost
//!
//! # Framing requirement
//!
//! The final type `T` (`P::Final`) must implement
//! [`crate::traits::SeekSentinel`]. Without a sentinel there is no way to
//! tell one packet's bytes from the next inside a continuous byte stream.
//! The bound holds in resume mode too, which never seeks
//!
//! See [`crate::traits::DecodePartial`] for the underlying trait,
//! [`crate::decoder::Packet`] for the two-state signal (`Ready` / `NeedMore`), and
//! [`crate::traits::Partial`] for the partial-to-final conversion contract
// --------------------------------------------------
// mods
// --------------------------------------------------
mod buf;
mod iter;
mod pkt;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use iter::{DecodeIterError, DecoderIter};
pub use pkt::Packet;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::traits::{DecodePartial, Partial, PartialIterator, ResumePartial, SeekSentinel};
use buf::BufCursor;

// --------------------------------------------------
// external
// --------------------------------------------------
use core::marker::PhantomData;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Empty byte slice used as a positional anchor when constructing errors at [`Decoder::finish`] time, where no live input exists
const EMPTY: &[u8] = &[];

/// Owned-buffer streaming decoder, parameterised on the [`Partial`] type
///
/// Accepts bytes via [`feed`](Self::feed) and yields decoded values via
/// [`next`](Self::next). A `None` from `next` means no packet came out of
/// that call: the buffer is empty or holds part of a packet, so call `feed`
/// again, or the packet it reached was malformed. A malformed packet's bytes
/// are drained, so the next call moves on to whatever follows it. Calling
/// [`PartialIterator::next_fresh`] instead tells the cases apart
///
/// # Example
///
/// ```ignore
/// // feed bytes as they arrive; `next` yields each ready packet. A packet cut
/// // across reads stays buffered across `feed` calls - no manual resume is needed
/// let mut dec: Decoder<MyPacketPartial, &[u8]> = Decoder::new();
/// let mut scratch = [0u8; 2048];
/// loop {
///     let n = socket.recv(&mut scratch)?;
///     dec.feed(&scratch[..n]);
///     while let Some(pkt) = dec.next() {
///         handle(pkt);
///     }
/// }
/// ```
pub struct Decoder<P, S> {
    /// Growing intake buffer plus its consumed-prefix cursor. Each decoded packet advances the
    /// cursor in O(1) rather than draining the buffer front; the consumed prefix is reclaimed in
    /// bulk by [`Decoder::feed`]. See [`BufCursor`] for the `head <= len` invariant
    buf: BufCursor,

    /// In-flight partial when resuming a [`Packet::NeedMore`]; `None` in fresh mode
    partial: Option<P>,

    _marker: PhantomData<S>,
}
/// [`Decoder`] implementation of [`std::fmt::Debug`]
///
/// Bounded only on `P: Debug` - `S` is a [`PhantomData`] marker and carries no value to format
impl<P: std::fmt::Debug, S> std::fmt::Debug for Decoder<P, S> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Decoder")
            .field("buf", &self.buf)
            .field("partial", &self.partial)
            .finish()
    }
}
/// [`Decoder`] implementation of [`Default`]
impl<P, S> Default for Decoder<P, S> {
    #[inline] // not always inline: one-line forward to new(); the optimizer inlines it, no need to force
    fn default() -> Self {
        Self::new()
    }
}
/// [`Decoder`] implementation
impl<P, S> Decoder<P, S> {
    /// Constructs an empty [`Decoder`] in fresh mode with no buffered bytes and no in-flight partial
    ///
    /// Fresh mode means the decoder will seek a sentinel boundary on the next
    /// [`Self::next`] call. Use [`Self::with_capacity`] when the expected
    /// packet size is known in advance, to avoid reallocations
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::Decoder;
    ///
    /// // type inference requires the partial type to be known from context
    /// // let mut dec: Decoder<MyPacketPartial, &[u8]> = Decoder::new();
    /// ```
    #[inline]
    pub fn new() -> Self {
        Self {
            buf: BufCursor::new(),
            partial: None,
            _marker: PhantomData,
        }
    }

    /// Constructs an empty [`Decoder`] in fresh mode with a pre-allocated internal buffer
    ///
    /// Equivalent to [`Self::new`] but avoids the first reallocation when
    /// the caller knows roughly how many bytes will arrive. The `cap` hint is
    /// passed to [`Vec::with_capacity`]; the buffer will still grow beyond `cap`
    /// if needed
    ///
    /// # Arguments
    ///
    /// * `cap` - Initial byte capacity to pre-allocate in the internal buffer
    #[inline]
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: BufCursor::with_capacity(cap),
            partial: None,
            _marker: PhantomData,
        }
    }

    /// Returns the bytes currently buffered but not yet consumed by decoding
    ///
    /// Useful for observability, diagnostics, and logging. The returned slice
    /// is borrowed from the decoder's internal `Vec<u8>` and does not copy
    ///
    /// # Returns
    ///
    /// A `&[u8]` view of the pending bytes; empty when all data has been decoded
    #[inline]
    #[must_use]
    pub fn buffered(&self) -> &[u8] {
        self.buf.pending()
    }

    /// Borrows the in-flight partial, if any
    ///
    /// Returns `Some(&P)` only when the decoder is in resume mode - i.e., after
    /// [`PartialIterator::next_resume`] stored a half-built packet and before the next
    /// [`Self::next`] call successfully completes the packet. Returns `None`
    /// when the decoder is in fresh mode
    ///
    /// # Returns
    ///
    /// `Some(&P)` when a partial packet is in-flight, `None` in fresh mode
    #[inline]
    #[must_use]
    pub fn partial(&self) -> Option<&P> {
        self.partial.as_ref()
    }

    /// Consumes the decoder and returns the in-flight partial, if one exists
    ///
    /// Prefer [`Self::finish`] on the `&[u8]` specialisation when the upstream signals
    /// end-of-stream: it finalizes the partial into the value instead
    ///
    /// # Returns
    ///
    /// `Some(P)` if a partial is in-flight, `None` in fresh mode
    #[inline]
    #[must_use]
    pub fn into_partial(self) -> Option<P> {
        self.partial
    }

    /// Drops all buffered bytes and clears any in-flight partial, resetting the decoder to fresh mode
    ///
    /// Call this after a [`crate::decoder::iter::DecodeIterError::Malformed`] error when
    /// you want a clean resync point rather than byte-walking through the corrupt
    /// region. After `clear`, the next [`Self::next`] call will seek a new sentinel
    /// from scratch
    #[inline]
    pub fn clear(&mut self) {
        self.buf.clear();
        self.partial = None;
    }

    /// Appends bytes to the internal buffer without triggering decoding
    ///
    /// This is the intake point for new data arriving from a socket, ring buffer,
    /// or any other byte source. Decoding is intentionally deferred: call
    /// [`Self::next`] or iterate with [`Self::iter`] after feeding to attempt
    /// to produce complete values from the accumulated bytes
    ///
    /// Before appending, the consumed prefix from already-decoded packets is reclaimed - in O(1)
    /// when the buffer is fully consumed, otherwise only when the freed prefix is at least as large
    /// as the live tail it must shift. This amortises compaction to at most once per feed rather
    /// than once per decoded packet, so a burst of `next` calls between two feeds advances the
    /// cursor without moving any bytes
    ///
    /// # Arguments
    ///
    /// * `bytes` - The new bytes to append; may be a partial packet, multiple
    ///   packets, or anything in between
    #[inline]
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend(bytes);
    }

    /// Returns a borrowing iterator that yields decoded values from the buffer
    ///
    /// The returned [`DecoderIter`] borrows `self` mutably and yields `T`
    /// values (the `Partial::Final` type) until the buffer is exhausted, a
    /// packet is incomplete, or one is malformed. Buffered bytes that may still
    /// begin a packet are retained between iterations so that a later
    /// [`Self::feed`] + re-iteration can pick up where decoding stopped
    ///
    /// # Returns
    ///
    /// A [`DecoderIter`] borrowing this decoder for the lifetime of the iterator
    #[inline]
    pub fn iter(&mut self) -> DecoderIter<'_, P, S> {
        DecoderIter { dec: self }
    }

    /// Decodes and returns the next complete value from the buffer, if one is available
    ///
    /// Calls [`Self::iter`] and pulls one item. Returns `None` when the buffer
    /// holds no complete packet (EOF or incomplete) or the packet it reached is
    /// malformed. A malformed packet's bytes are drained, so the next call moves
    /// past it to any packet buffered after it. Call [`Self::buffered`] afterward
    /// to inspect remaining bytes
    ///
    /// # Returns
    ///
    /// `Some(T)` when a complete, successfully-decoded value is available, `None` otherwise
    #[allow(
        clippy::should_implement_trait,
        reason = "the people should be allowed to use .next() in-line"
    )]
    #[inline]
    #[must_use]
    pub fn next<T>(&mut self) -> Option<T>
    where
        S: winnow::stream::Stream,
        P: Partial<Final = T> + Default,
        Self: PartialIterator<P>,
    {
        self.iter().next()
    }
}

/// [`Decoder`] implementation specialised to `&[u8]` streams, adding [`Decoder::finish`]
///
/// This specialisation adds [`Decoder::finish`], which is only meaningful
/// on `&[u8]` streams where the end-of-stream condition is definite. When
/// the decoder carries an in-flight partial, [`Decoder::next`] resumes it with
/// [`ResumePartial::resume_partial`] without seeking a sentinel, because
/// the partial state already holds the fields decoded so far
impl<P, T> Decoder<P, &[u8]>
where
    P: Partial<Final = T> + Default,
    for<'a> T:
        DecodePartial<&'a [u8], Partial = P> + ResumePartial<&'a [u8]> + SeekSentinel<&'a [u8]>,
{
    /// Force-finalises the in-flight partial, consuming the decoder
    ///
    /// Call this when the upstream transport signals end-of-stream and you
    /// want either the accumulated partial surfaced as `T` (if all required
    /// fields are present) or a definitive error. The decoder is consumed
    /// and the buffered bytes are discarded - no further decoding is possible
    /// after this call
    ///
    /// If no partial is in-flight, a default partial is finalised; depending
    /// on the `T` type this may succeed (all fields optional) or fail (required
    /// fields missing)
    ///
    /// # Returns
    ///
    /// `Ok(T)` when the partial can be finalised with all required fields satisfied
    ///
    /// # Errors
    ///
    /// - A required field is absent: [`DecodeIterError::Malformed`] of kind
    ///   [`ErrorKind::MissingRequired`](crate::ErrorKind::MissingRequired)
    pub fn finish(self) -> Result<T, DecodeIterError> {
        let partial = self.partial.unwrap_or_default();
        partial.finalize().map_err(|err| {
            // --------------------------------------------------
            // no live input at finish time, so the error is
            // anchored at EMPTY
            // --------------------------------------------------
            DecodeIterError::Malformed(err.to_context_error(EMPTY))
        })
    }
}
