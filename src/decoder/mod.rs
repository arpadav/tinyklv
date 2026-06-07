//! Streaming KLV decoder
//!
//! [`Decoder<P>`] owns a growing byte buffer and an optional in-flight
//! [`crate::traits::Partial`] state. It yields fully-decoded values of the
//! Partial's `Final` type as enough bytes arrive. It is the user-facing API
//! for parsing KLV from fragmented transports (short TCP reads, ring
//! buffers, UDP re-assembly) where a single logical packet may straddle
//! multiple reads, AND for resuming a packet whose body was cut mid-decode
//! (delivered through [`crate::decoder::Packet::NeedMore`])
//!
//! # Two modes of operation
//!
//! * **Fresh mode** (`partial = None`): [`Decoder::next`] uses
//!   [`crate::traits::SeekSentinel`] to locate each packet boundary and
//!   then runs [`crate::traits::DecodePartial::decode_partial`] on the
//!   framed body. This is the original sentinel-framed streaming flow
//!
//! * **Resume mode** (`partial = Some(p)`): the decoder was constructed
//!   from a [`Packet::NeedMore`] return inside `decode_partial` and
//!   already carries a half-built packet. [`Decoder::next`] resumes
//!   directly into the existing partial without re-seeking the sentinel,
//!   so previously-decoded fields are not lost
//!
//! # Framing requirement (fresh mode only)
//!
//! In fresh mode, the final type `T` (`P::Final`) must implement
//! [`crate::traits::SeekSentinel`]. Without a sentinel there is no way to
//! tell one packet's bytes from the next inside a continuous byte stream
//! Resume mode bypasses the sentinel entirely - the partial is already
//! past the framing
//!
//! See [`crate::traits::DecodePartial`] for the underlying trait,
//! [`crate::decoder::Packet`] for the two-state signal (`Ready` / `NeedMore`), and
//! [`crate::traits::Partial`] for the partial-to-final conversion contract
// --------------------------------------------------
// mods
// --------------------------------------------------
#[macro_use]
mod iter;
mod buf;
mod pkt;

// --------------------------------------------------
// re-exports
// --------------------------------------------------
pub use iter::*;
pub use pkt::*;

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
/// [`next`](Self::next). A `None` from `next` means "need more bytes,
/// call `feed` again" - malformed packets are silently skipped (the
/// framed bytes are drained to guarantee forward progress)
///
/// # Example
///
/// ```ignore
/// // feed bytes as they arrive; `next` yields each ready packet. A partial packet
/// // is carried internally across `feed` calls - no manual resume is needed
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
    /// a [`Packet::NeedMore`] result was stored and before the next
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
    /// Used by the derive-generated `decode_partial` wrapper to convert a
    /// "buffer fully consumed with `NeedMore` still pending" state into a
    /// finalisation decision at the fresh-mode boundary. Callers outside of
    /// generated code rarely need this directly; prefer [`Self::finish`] on the
    /// `&[u8]` specialisation when the upstream signals end-of-stream
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
    /// as the live tail it must shift (see [`BufCursor::extend`]). This amortises compaction to at
    /// most once per feed rather than once per decoded packet, so a burst of `next` calls between
    /// two feeds advances the cursor without moving any bytes
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
    /// values (the `Partial::Final` type) until the buffer is exhausted or a
    /// packet is incomplete. Buffered-but-undecoded bytes are retained between
    /// iterations so that a later [`Self::feed`] + re-iteration can pick up
    /// where decoding stopped
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
    /// contains no complete packet (EOF, incomplete, or malformed). Malformed
    /// packets are silently skipped - the framed bytes are drained - to keep
    /// the decoder advancing. Call [`Self::buffered`] afterward to inspect
    /// remaining bytes
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
/// the decoder carries an in-flight partial, [`Decoder::next`] resumes
/// [`DecodePartial::decode_partial`] without re-seeking the sentinel because
/// the partial state already contains the bytes consumed past the framing
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
    /// `Ok(T)` when the partial can be finalised with all required fields satisfied,
    /// or `Err(DecodeIterError::Malformed)` when required fields are absent
    pub fn finish(self) -> Result<T, DecodeIterError> {
        let partial = self.partial.unwrap_or_default();
        partial.finalize().map_err(|label| {
            // --------------------------------------------------
            // no live input at finish time, so the error carries
            // the label anchored at EMPTY
            // --------------------------------------------------
            DecodeIterError::Malformed(label_to_context_error!(EMPTY, label))
        })
    }
}
