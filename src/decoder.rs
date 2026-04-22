//! Streaming KLV decoder.
//!
//! [`Decoder<T>`] owns a growing byte buffer and yields fully-decoded `T`
//! values as enough bytes arrive. It is the user-facing API for parsing
//! KLV from fragmented transports (short TCP reads, ring buffers, UDP
//! re-assembly) where a single logical packet may straddle multiple
//! reads.
//!
//! # Framing requirement
//!
//! `T` must implement [`crate::traits::SeekSentinel`]. The decoder uses
//! the sentinel + declared packet length to locate each packet's exact
//! body before handing it to [`crate::traits::DecodePartial`]. Without a
//! sentinel there is no way to tell one packet's bytes from the next
//! inside a continuous byte stream - `decode_partial` would greedily
//! merge them.
//!
//! For structs without a sentinel, call [`crate::traits::DecodeValue::decode_value`]
//! directly on each complete packet you already have in hand.
//!
//! See [`crate::traits::DecodePartial`] for the underlying trait and
//! [`crate::traits::Progress`] for the tri-state signal.
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::traits::{DecodePartial, Progress, SeekSentinel};

// --------------------------------------------------
// external
// --------------------------------------------------
use core::marker::PhantomData;
use winnow::error::ContextError;

#[derive(Debug)]
/// Boundary error type surfaced by [`Decoder::next`].
///
/// Internal [`Progress::NeedMore`] is not surfaced; it is represented as
/// `next() -> None`. Only [`Progress::Malformed`] becomes an `Err`.
pub enum DecodeError {
    /// The bytes present in the buffer could not be parsed as the target
    /// type. The [`Decoder`] has already advanced past the offending byte
    /// (by at least one) to guarantee forward progress on subsequent calls.
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

#[derive(Default)]
/// Owned-buffer streaming decoder.
///
/// Accepts bytes via [`feed`](Self::feed) and yields decoded values via
/// [`next`](Self::next). A `None` from `next` means "need more bytes, call
/// `feed` again." A `Some(Err)` means the current position is malformed;
/// the decoder has already advanced past the bad byte to avoid looping.
///
/// # Example
///
/// ```ignore
/// let mut dec = Decoder::<MyPacket>::new();
/// let mut scratch = [0u8; 2048];
/// loop {
///     let n = socket.recv(&mut scratch)?;
///     dec.feed(&scratch[..n]);
///     while let Some(pkt) = dec.next() {
///         handle(pkt?);
///     }
/// }
/// ```
pub struct Decoder<T> {
    buf: Vec<u8>,
    _marker: PhantomData<T>,
}
/// [`Decoder`] implementation
impl<T> Decoder<T> {
    #[inline(always)]
    /// Construct an empty decoder.
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            _marker: PhantomData,
        }
    }

    #[inline(always)]
    /// Construct an empty decoder with a pre-allocated buffer capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
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
    /// Drop all buffered bytes. Call after a `Malformed` error if you want
    /// to resync cleanly rather than byte-walk through corrupt input.
    pub fn clear(&mut self) {
        self.buf.clear();
    }
}

impl<T> Decoder<T>
where
    for<'a> T: DecodePartial<&'a [u8]> + SeekSentinel<&'a [u8]>,
{
    /// Attempt to decode one `T` from the buffered bytes.
    ///
    /// # Note on naming
    ///
    /// This method is deliberately named `next` despite [`Iterator`] not
    /// being implemented: the return type is `Option<Result<T, _>>`, not
    /// `Option<T>`, so the `Iterator` contract would force callers to
    /// lose the error-vs-exhaustion distinction. `#[allow]` below
    /// suppresses the corresponding clippy lint.
    ///
    /// Framing flow:
    ///
    /// 1. Scan the buffer for `T`'s sentinel bytes.
    /// 2. Read the declared packet length that follows the sentinel.
    /// 3. If the full body is present, run [`DecodePartial::decode_partial`]
    ///    on it and emit the result.
    ///
    /// Any shortfall at steps 1-3 returns `None` without dropping buffered
    /// bytes - the caller just needs to [`feed`](Self::feed) more.
    ///
    /// Return values:
    ///
    /// * `Some(Ok(t))` - `t` was decoded; its bytes (sentinel + length +
    ///   body) are drained from the buffer.
    /// * `Some(Err(DecodeError::Malformed(_)))` - the body's bytes were
    ///   present but could not be decoded. The framing bytes + body are
    ///   still drained so the next packet is not masked by the bad one.
    /// * `None` - need more bytes, or no sentinel found yet.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<Result<T, DecodeError>> {
        if self.buf.is_empty() {
            return None;
        }
        // --------------------------------------------------
        // `scan` is a cheap copy of the slice head-pointer; mutating
        // it via `seek_sentinel` does NOT touch `self.buf`. we commit
        // consumption to `buf` only after we know how far we want to
        // advance (on Ok, or on Malformed to drop the bad packet).
        // --------------------------------------------------
        let mut scan: &[u8] = self.buf.as_slice();
        let before = scan.len();
        let body = match T::seek_sentinel(&mut scan) {
            Ok(b) => b,
            Err(_) => {
                // --------------------------------------------------
                // seek_sentinel failed: sentinel not found, length
                // couldn't be read, or declared length exceeded what
                // we have buffered. in a streaming context all three
                // look identical from here - keep buffering and try
                // again after the next `feed`. no bytes dropped.
                // --------------------------------------------------
                return None;
            }
        };
        // --------------------------------------------------
        // we own a complete body slice now. run `decode_partial`
        // against it; any `NeedMore` at this point is malformed
        // (the body length was declared - if it cannot complete, the
        // declared length is wrong or the body is ill-formed).
        // --------------------------------------------------
        let mut body_cursor: &[u8] = body;
        let framed_consumed = before - scan.len();
        match T::decode_partial(&mut body_cursor) {
            Progress::Ready(t) => {
                self.buf.drain(..framed_consumed);
                Some(Ok(t))
            }
            Progress::Malformed(e) => {
                // --------------------------------------------------
                // body is present but bad. drain the whole framed
                // packet so we don't try it again on the next call,
                // and surface the parser's context to the caller
                // --------------------------------------------------
                self.buf.drain(..framed_consumed);
                Some(Err(DecodeError::Malformed(e)))
            }
            Progress::NeedMore(_) => {
                // --------------------------------------------------
                // body had the declared length but still wanted more
                // bytes. that's a malformed declared-length or a
                // body whose inner structure is broken - neither is
                // recoverable by buffering, so drop the packet. the
                // context error here is intentionally minimal;
                // enriching it requires plumbing a StrContext through
                // the boundary, which is follow-up work.
                // --------------------------------------------------
                self.buf.drain(..framed_consumed);
                Some(Err(DecodeError::Malformed(ContextError::new())))
            }
        }
    }
}
