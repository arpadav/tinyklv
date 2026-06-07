//! In-flight parse state and resumable partial-decode traits for streaming KLV
//!
//! Contains:
//! * [`Partial`] - the in-flight parse state that mirrors a KLV struct with every
//!   field as `Option<T>`; its [`Partial::finalize`] method validates required fields
//!   and produces the final value
//! * [`PartialIterator`] - the low-level iterator interface for advancing a
//!   [`crate::decoder::Decoder`] one step at a time, in either fresh or resume mode
//! * [`DecodePartial`] - the streaming-aware counterpart to [`DecodeValue`],
//!   returning [`Packet`] instead of a plain `Result` to distinguish "need more bytes"
//!   from "malformed"
//! * [`ResumePartial`] - the internal re-entry point that resumes a paused partial
//!   decode from an existing [`DecodePartial::Partial`] state
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::DecodeValue;
use crate::decoder::{DecodeIterError, Packet};

/// In-flight parse state that can be finalised into a complete value
///
/// Every [`DecodePartial`] impl has an associated [`DecodePartial::Partial`]
/// type that implements this trait. The partial is a mirror of the final
/// struct where every KLV field is `Option<T>`; [`Partial::finalize`] runs
/// required-field validation and yields the final type on success
///
/// The derive macro emits the partial type automatically as
/// `Tinyklv<Name>PartialPacket`. Manual impls are only needed for types
/// that do not use the derive macro
pub trait Partial: Sized {
    /// The finalised type this partial turns into
    type Final;

    /// Validates required fields and produces the final value
    ///
    /// Missing required fields produce a `&'static str` label naming the
    /// first missing field. The label is intentionally context-free: the
    /// partial has zero knowledge of the live input stream, so a richer
    /// error type would have to fabricate input/checkpoint slots. The
    /// [`crate::Decoder`] (or any caller with input access) wraps this
    /// label into a [`winnow::error::ContextError`] with real input and
    /// checkpoint at the point of failure
    ///
    /// # Returns
    ///
    /// `Ok(Self::Final)` when all required fields are present, or
    /// `Err(&'static str)` naming the first missing required field
    fn finalize(self) -> Result<Self::Final, &'static str>;
}

/// Low-level iterator interface for advancing a [`crate::decoder::Decoder`] one packet at a time
///
/// Implemented by [`crate::decoder::Decoder`] for the `&[u8]` stream specialisation
/// Callers outside of generated code and the decoder internals should prefer
/// [`crate::decoder::Decoder::next`] or the [`Iterator`] impl on
/// [`crate::decoder::DecoderIter`] rather than calling these methods directly
pub trait PartialIterator<P: Partial> {
    /// Continues decoding a packet that was previously interrupted with [`Packet::NeedMore`]
    ///
    /// Picks up from the stored in-flight partial, runs
    /// [`ResumePartial::resume_partial`] against the buffered bytes, and
    /// returns the completed value or a [`DecodeIterError`] describing why
    /// decoding could not finish
    ///
    /// # Returns
    ///
    /// `Ok(P::Final)` when the resumed packet completes, or a [`DecodeIterError`]
    /// variant describing whether more bytes are needed, the buffer is empty,
    /// or the bytes were malformed
    fn next_resume(&mut self) -> Result<P::Final, DecodeIterError>;

    /// Seeks the next sentinel boundary and decodes the framed body from scratch
    ///
    /// This is the standard sentinel-framed flow: scan for the sentinel,
    /// consume the declared body length, then run [`DecodePartial::decode_partial`]
    /// on the body slice. The in-flight partial is `None` on entry
    ///
    /// # Returns
    ///
    /// `Ok(P::Final)` when a complete packet is decoded, or a [`DecodeIterError`]
    /// variant describing whether the sentinel has not yet arrived, more bytes
    /// are needed for the body, or the framed bytes were malformed
    fn next_fresh(&mut self) -> Result<P::Final, DecodeIterError>;
}

/// Streaming-aware counterpart to [`DecodeValue`]
///
/// Returns a [`Packet<Self, Self::Partial>`] instead of `crate::Result<Self>`,
/// so callers can distinguish "need more bytes" from "malformed." Drives
/// [`crate::Decoder`], the user-facing streaming API
///
/// Every implementor has an associated [`Self::Partial`] type: a mirror of
/// the struct in which every KLV field is `Option<T>`, i.e. a parse state
/// that can be fed incrementally and validated only at the end. The derive
/// macro emits this automatically as `Tinyklv<Name>PartialPacket`
///
/// Automatically implemented for structs deriving [`tinyklv::Klv`](crate::Klv)
///
/// The outer `Result` carries a `&'static str` label when the codegen hits an
/// unrecoverable parse condition and the partial-to-final conversion also fails
/// Successful conversion on give-up is `Ok(Packet::Ready(t))`. Recoverable
/// truncation surfaces as `Ok(Packet::NeedMore(p))`
pub trait DecodePartial<S>: Sized + DecodeValue<S>
where
    S: winnow::stream::Stream,
{
    /// In-flight parse state
    ///
    /// `Default` is the fresh (all-fields-missing) starting point;
    /// `Partial<Final = Self>` runs required-field validation at finalisation time
    type Partial: Default + Partial<Final = Self>;

    /// Attempts to decode `Self` from `input`, returning a [`Packet`] that
    /// distinguishes completion from a need-more-bytes pause
    ///
    /// On clean completion, returns `Ok(Packet::Ready(value))` with the fully
    /// decoded value. When the stream ends before all required fields are seen,
    /// returns `Ok(Packet::NeedMore(partial))` so callers can feed more bytes and
    /// resume. Returns `Err(label)` only for unrecoverable conditions such as
    /// a parse error after consuming bytes
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to decode from; advanced by whatever bytes are consumed
    ///
    /// # Returns
    ///
    /// `Ok(Packet::Ready(Self))` on success, `Ok(Packet::NeedMore(Self::Partial))`
    /// when more bytes are needed, or `Err(&'static str)` on an unrecoverable error
    fn decode_partial(input: &mut S) -> Result<Packet<Self, Self::Partial>, &'static str>;
}

#[doc(hidden)]
/// Re-entry point for resuming a paused partial decode from an existing [`DecodePartial::Partial`] state
///
/// This is used internally and is not expected to be called directly
///
/// Generic over the stream type `S`, same as [`DecodePartial`]. The derive
/// macro emits this impl alongside [`DecodePartial`]; [`DecodePartial::decode_partial`]
/// itself is just `resume_partial(input, Self::Partial::default())`
///
/// This is the canonical source-of-truth for the decoding implementation:
/// it is called by [`DecodePartial::decode_partial`] and, by proxy, by
/// [`DecodeValue::decode_value`]
pub trait ResumePartial<S>: DecodePartial<S>
where
    S: winnow::stream::Stream,
{
    /// Continues a partial decode from `partial`, consuming bytes from `input`
    ///
    /// Has the same outer `Result<Packet<...>, &'static str>` return shape as
    /// [`DecodePartial::decode_partial`]. This is the source-of-truth entry
    /// point; [`DecodePartial::decode_partial`] is just
    /// `resume_partial(input, Default::default())`
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to continue decoding from; advanced by whatever bytes are consumed
    /// * `partial` - The accumulated in-flight state from a previous [`DecodePartial::decode_partial`]
    ///   or [`ResumePartial::resume_partial`] call
    ///
    /// # Returns
    ///
    /// `Ok(Packet::Ready(Self))` when all required fields are present,
    /// `Ok(Packet::NeedMore(Self::Partial))` when more bytes are needed,
    /// or `Err(&'static str)` on an unrecoverable parse error
    fn resume_partial(
        input: &mut S,
        partial: Self::Partial,
    ) -> Result<Packet<Self, Self::Partial>, &'static str>;
}

/// [`Vec`] implementation of [`Partial`] for element types that implement [`Partial`]
///
/// Finalises by calling [`Partial::finalize`] on every element in order
/// Returns the first `Err` encountered, or `Ok(Vec<P::Final>)` when all
/// elements finalise successfully
impl<P: Partial> Partial for Vec<P> {
    type Final = Vec<<P as Partial>::Final>;

    #[inline(always)]
    fn finalize(self) -> Result<Self::Final, &'static str> {
        self.into_iter().map(P::finalize).collect()
    }
}

/// [`Vec`] implementation of [`DecodePartial`] for all `T` that implement [`DecodePartial`]
///
/// Parses inner items until one returns [`Packet::NeedMore`] or an error without
/// consuming bytes. A [`Packet::NeedMore`] from the inner parser is treated as
/// "done for now": whatever was accumulated is best-effort-finalised and returned
/// as `Packet::Ready`. The cursor has been rewound by the inner call so the next
/// invocation can resume with more bytes
impl<S, T> DecodePartial<S> for Vec<T>
where
    S: winnow::stream::Stream,
    T: DecodePartial<S>,
{
    type Partial = Vec<<T as DecodePartial<S>>::Partial>;

    fn decode_partial(input: &mut S) -> Result<Packet<Self, Self::Partial>, &'static str> {
        let mut acc = Vec::new();
        loop {
            let before = input.eof_offset();
            let cp = input.checkpoint();
            match T::decode_partial(input) {
                // --------------------------------------------------
                // push the val and continue looping
                // --------------------------------------------------
                Ok(Packet::Ready(val)) => acc.push(val),
                // --------------------------------------------------
                // return the acc values, but try to coerce the
                // last partial packet if possible
                // --------------------------------------------------
                Ok(Packet::NeedMore(partial)) => {
                    if let Ok(last_elem) = partial.finalize() {
                        acc.push(last_elem);
                    }
                    return Ok(Packet::Ready(acc));
                }
                // --------------------------------------------------
                // if no bytes consumed, rewind and return the acc values as Ready
                // otherwise, propagate the error label outward
                // --------------------------------------------------
                Err(label) => {
                    if input.eof_offset() == before {
                        input.reset(&cp);
                        return Ok(Packet::Ready(acc));
                    }
                    return Err(label);
                }
            }
        }
    }
}
