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
//! tell one packet's bytes from the next inside a continuous byte stream.
//! Resume mode bypasses the sentinel entirely - the partial is already
//! past the framing
//!
//! See [`crate::traits::DecodePartial`] for the underlying trait,
//! [`crate::decoder::Packet`] for the two-state signal (`Ready` / `NeedMore`), and
//! [`crate::traits::Partial`] for the partial-to-final conversion contract.
// --------------------------------------------------
// mods
// --------------------------------------------------
#[macro_use]
mod iter;
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

// --------------------------------------------------
// external
// --------------------------------------------------
use core::marker::PhantomData;

// --------------------------------------------------
// constants
// --------------------------------------------------
const EMPTY: &[u8] = &[];

/// Owned-buffer streaming decoder, parameterised on the [`Partial`] type.
///
/// Accepts bytes via [`feed`](Self::feed) and yields decoded values via
/// [`next`](Self::next). A `None` from `next` means "need more bytes,
/// call `feed` again" - malformed packets are silently skipped (the
/// framed bytes are drained to guarantee forward progress).
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
///         handle(pkt);
///     }
/// }
///
/// // resume mode: handed back from `Packet::NeedMore`
/// match MyPacket::decode_partial(&mut input) {
///     Ok(Packet::Ready(t)) => use_packet(t),
///     Ok(Packet::NeedMore(partial)) => {
///         let mut dec = Decoder::new();
///         dec.feed(more_bytes);
///         while let Some(pkt) = dec.next() {
///             use_packet(pkt);
///             break;
///         }
///     }
///     Err(label) => report(label),
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
    /// The bytes currently buffered but not yet decoded. Useful for
    /// observability and diagnostics.
    pub fn buffered(&self) -> &[u8] {
        &self.buf
    }

    #[inline(always)]
    /// Borrow the in-flight partial, if any. `Some` only after the decoder
    /// was constructed from a [`Packet::NeedMore`] return and before the
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

    #[inline(always)]
    /// Append bytes to the internal buffer. Does not trigger decoding on its
    /// own - call [`next`](Self::next) to try to consume complete values.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    #[inline(always)]
    /// Returns an iterator over the buffered values, consuming them as they are
    /// decoded
    pub fn iter(&mut self) -> DecoderIter<'_, P, S> {
        DecoderIter { dec: self }
    }

    #[allow(
        clippy::should_implement_trait,
        reason = "the people should be allowed to use .next() in-line"
    )]
    #[inline(always)]
    /// Decode the next complete value from the buffer, if one is available
    pub fn next<T>(&mut self) -> Option<T>
    where
        S: winnow::stream::Stream,
        P: Partial<Final = T> + Default,
        Self: PartialIterator<P>,
    {
        self.iter().next()
    }

    // #[inline(always)]
    // /// Consumes an iterator of bytes, decoding complete values as they are
    // /// encountered
    // pub fn consume<'a, I, B, T>(&'a mut self, input: I) -> ConsumeIter<'a, P, S, I::IntoIter, B, T>
    // where
    //     I: IntoIterator<Item = B> + 'a,
    //     B: AsRef<[u8]> + 'a,
    //     S: winnow::stream::Stream,
    //     P: Partial<Final = T> + Default,
    //     Self: PartialIterator<P> + 'a,
    // {
    //     ConsumeIter {
    //         parser: self.iter(),
    //         input: input.into_iter(),
    //         _marker: std::marker::PhantomData,
    //     }
    // }
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
    pub fn finish(self) -> Result<T, DecodeIterError> {
        let partial = self.partial.unwrap_or_default();
        partial.finalize().map_err(|label| {
            // --------------------------------------------------
            // no live input at finish time - is this okay? carries label
            // but no live input to report position info.
            // --------------------------------------------------
            DecodeIterError::Malformed(label_to_context_error!(EMPTY, label))
        })
    }
}
