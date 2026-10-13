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
//! * [`PartialError`] - why a partial decode or a finalize failed: its [`ErrorKind`] and a label
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::DecodeValue;
use crate::{
    ErrorKind,
    decoder::{DecodeIterError, Packet},
};

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{error::ContextError, stream::Stream as _};

/// Why [`DecodePartial::decode_partial`], [`ResumePartial::resume_partial`] or
/// [`Partial::finalize`] failed: what the error is, and a label naming what was rejected
///
/// It displays as its label. A [`crate::Decoder`] turns it into the error of a
/// [`DecodeIterError::Malformed`], which [`ErrorKind::of`] reads its kind back from
///
/// It is two words wide, as a `&'static str` is: a kind and a label fixed at compile time are
/// held behind one reference, a [`BreakType::Abort`](crate::BreakType::Abort) message on its own
///
/// # Example
///
/// ```rust
/// use tinyklv::{ErrorKind, traits::PartialError};
///
/// let error = PartialError::new(&(ErrorKind::MissingRequired, "`Reading::id` is missing"));
/// assert_eq!(error.kind(), ErrorKind::MissingRequired);
/// assert_eq!(error.to_string(), "`Reading::id` is missing");
/// ```
#[derive(Clone, Copy)]
pub struct PartialError(Rejection);

/// What a [`PartialError`] holds
#[derive(Clone, Copy)]
enum Rejection {
    /// A kind and its label, both fixed at compile time
    Fixed(&'static (ErrorKind, &'static str)),

    /// A `break_on`'s `Abort` message, an [`ErrorKind::Aborted`] error
    Aborted(&'static str),
}

/// [`PartialError`] implementation
impl PartialError {
    /// An error of the kind and label `rejection` pairs
    ///
    /// `rejection` is a constant, such as `&(ErrorKind::MissingRequired, "..")`, which the
    /// compiler places in a `static`
    ///
    /// # Arguments
    ///
    /// * `rejection` - what the error is, and the label naming what was rejected, and why
    ///
    /// # Returns
    ///
    /// The error
    #[inline]
    #[must_use]
    pub const fn new(rejection: &'static (ErrorKind, &'static str)) -> Self {
        Self(Rejection::Fixed(rejection))
    }

    /// An [`ErrorKind::Aborted`] error labelled with the message of a `break_on`'s
    /// [`BreakType::Abort`](crate::BreakType::Abort)
    ///
    /// # Arguments
    ///
    /// * `message` - the `Abort` message
    ///
    /// # Returns
    ///
    /// The error
    #[inline]
    #[must_use]
    pub const fn aborted(message: &'static str) -> Self {
        Self(Rejection::Aborted(message))
    }

    /// What the error is, for a caller to match on instead of its label
    #[inline]
    #[must_use]
    pub const fn kind(self) -> ErrorKind {
        match self.0 {
            Rejection::Fixed(&(kind, _)) => kind,
            Rejection::Aborted(_) => ErrorKind::Aborted,
        }
    }

    /// The label naming what was rejected, and why
    #[inline]
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self.0 {
            Rejection::Fixed(&(_, label)) | Rejection::Aborted(label) => label,
        }
    }

    /// The [`ContextError`] this error is to a caller holding the input, anchored at `input`:
    /// labelled with [`Self::label`] and tagged with [`Self::kind`]
    ///
    /// # Arguments
    ///
    /// * `input` - the bytes the error is anchored to
    ///
    /// # Returns
    ///
    /// The error, whose display is its label's
    // as `__export::with_kind`: `cold` keeps it out of the caller's body, `inline` compiles it in
    // the calling crate
    #[cold]
    #[inline]
    pub(crate) fn to_context_error(self, input: &[u8]) -> ContextError {
        let error = crate::__export::labeled_error(&input, &input.checkpoint(), self.label());
        crate::__export::with_kind(error, self.kind())
    }
}
/// [`PartialError`] implementation of [`core::fmt::Debug`], as its kind and label
impl core::fmt::Debug for PartialError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PartialError")
            .field("kind", &self.kind())
            .field("label", &self.label())
            .finish()
    }
}
/// [`PartialError`] implementation of [`PartialEq`], equal when kind and label are
impl PartialEq for PartialError {
    fn eq(&self, other: &Self) -> bool {
        (self.kind(), self.label()) == (other.kind(), other.label())
    }
}
/// [`PartialError`] implementation of [`Eq`]
impl Eq for PartialError {}
/// [`PartialError`] implementation of [`core::hash::Hash`], over its kind and label
impl core::hash::Hash for PartialError {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        (self.kind(), self.label()).hash(state);
    }
}
/// [`PartialError`] implementation of [`core::fmt::Display`]
impl core::fmt::Display for PartialError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.label())
    }
}
/// [`PartialError`] implementation of [`std::error::Error`]
impl std::error::Error for PartialError {}

/// In-flight parse state that can be finalised into a complete value
///
/// Every [`DecodePartial`] impl has an associated [`DecodePartial::Partial`]
/// type that implements this trait. The partial is a mirror of the final
/// struct where every KLV field is `Option<T>`; [`Partial::finalize`] runs
/// required-field validation and yields the final type on success
///
/// The derive macro emits the partial type automatically as
/// `<Name>PartialPacket`. Manual impls are only needed for types
/// that do not use the derive macro
pub trait Partial: Sized {
    /// The finalised type this partial turns into
    type Final;

    /// Validates required fields and produces the final value
    ///
    /// Missing required fields produce a [`PartialError`] whose label names the
    /// first missing field. The error is intentionally context-free: the
    /// partial has zero knowledge of the live input stream, so a richer
    /// error type would have to fabricate input/checkpoint slots. The
    /// [`crate::Decoder`] (or any caller with input access) turns this
    /// error into a [`winnow::error::ContextError`] with real input and
    /// checkpoint at the point of failure
    ///
    /// # Returns
    ///
    /// `Ok(Self::Final)` when all required fields are present
    ///
    /// # Errors
    ///
    /// - A required field is missing: [`ErrorKind::MissingRequired`], labelled with the first one
    fn finalize(self) -> Result<Self::Final, PartialError>;
}

/// Sealed marker restricting [`PartialIterator`] to [`crate::decoder::Decoder`], so external
/// crates cannot add their own implementations
mod private {
    use crate::decoder::Decoder;

    pub trait Sealed {}
    impl<P, S> Sealed for Decoder<P, S> {}
    impl<P, S> Sealed for &mut Decoder<P, S> {}
}

/// Low-level iterator interface for advancing a [`crate::decoder::Decoder`] one packet at a time
///
/// This is a sealed trait: it is implemented by [`crate::decoder::Decoder`] for the `&[u8]` stream
/// specialisation, and cannot be implemented downstream
///
/// Callers outside of generated code and the decoder internals should prefer
/// [`crate::decoder::Decoder::next`] or the [`Iterator`] impl on
/// [`crate::decoder::DecoderIter`] rather than calling these methods directly
pub trait PartialIterator<P: Partial>: private::Sealed {
    /// Continues decoding a packet that was interrupted with [`Packet::NeedMore`]
    ///
    /// Picks up from the stored in-flight partial, runs
    /// [`ResumePartial::resume_partial`] against the buffered bytes, and
    /// returns the completed value or a [`DecodeIterError`] describing why
    /// decoding could not finish
    ///
    /// # Returns
    ///
    /// `Ok(P::Final)` when the resumed packet completes
    ///
    /// # Errors
    ///
    /// - [`DecodeIterError::Eof`]: the buffer is empty and no partial is in flight
    /// - [`DecodeIterError::NeedMore`]: the packet still needs more bytes; the updated partial is
    ///   kept for the next call
    /// - [`DecodeIterError::Malformed`]: the resumed decode failed
    fn next_resume(&mut self) -> Result<P::Final, DecodeIterError>;

    /// Seeks the next sentinel boundary and decodes the framed body from scratch
    ///
    /// This is the standard sentinel-framed flow: scan for the sentinel,
    /// consume the declared body length, then decode the body slice one-shot
    /// with [`DecodeValue::decode_value`]. The in-flight partial is `None` on entry
    ///
    /// # Returns
    ///
    /// `Ok(P::Final)` when a complete packet is decoded
    ///
    /// # Errors
    ///
    /// - [`DecodeIterError::Eof`]: the buffer is empty
    /// - [`DecodeIterError::NeedMore`]: the sentinel, the length or the body has not all arrived
    /// - [`DecodeIterError::Malformed`]: the body failed to decode, its packet's bytes drained
    fn next_fresh(&mut self) -> Result<P::Final, DecodeIterError>;
}

/// Streaming-aware counterpart to [`DecodeValue`]
///
/// Returns a [`Packet<Self, Self::Partial>`] instead of `crate::Result<Self>`,
/// so callers can distinguish "need more bytes" from "malformed." Parameterises
/// [`crate::Decoder`], the user-facing streaming API, through [`Self::Partial`]
///
/// Every implementor has an associated [`Self::Partial`] type: a mirror of
/// the struct in which every KLV field is `Option<T>`, i.e. a parse state
/// that can be fed incrementally and validated only at the end. The derive
/// macro emits this automatically as `<Name>PartialPacket`
///
/// Automatically implemented for structs deriving [`tinyklv::Klv`](crate::Klv)
///
/// The outer `Result` carries a [`PartialError`] when an item is malformed
/// or rejected, or when the decode loop ends and the partial cannot finalize
///
/// A loop that ends and finalizes is `Ok(Packet::Ready(t))`. Input that runs
/// out, between items or inside one, is `Ok(Packet::NeedMore(p))`
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
    /// When the input runs out, between items or inside one, returns
    /// `Ok(Packet::NeedMore(partial))` holding every field decoded so far, even
    /// when every required field is present, so callers can feed more bytes and
    /// resume. When the decode loop ends first, such as at a `break_on` stop,
    /// the partial is finalized into `Ok(Packet::Ready(value))`
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to decode from; advanced by whatever bytes are consumed
    ///
    /// # Returns
    ///
    /// `Ok(Packet::Ready(Self))` on success, `Ok(Packet::NeedMore(Self::Partial))`
    /// when more bytes are needed
    ///
    /// # Errors
    ///
    /// - An item is malformed or rejected: a [`PartialError`] naming it, returned at once
    /// - The decode loop ends with a required field missing: [`ErrorKind::MissingRequired`],
    ///   naming it
    fn decode_partial(input: &mut S) -> Result<Packet<Self, Self::Partial>, PartialError>;
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
/// This holds the streaming decode loop, which the derive's
/// [`DecodeValue::decode_value`] also runs, one-shot
pub trait ResumePartial<S>: DecodePartial<S>
where
    S: winnow::stream::Stream,
{
    /// Continues a partial decode from `partial`, consuming bytes from `input`
    ///
    /// Has the same outer `Result<Packet<...>, PartialError>` return shape as
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
    /// or `Err(PartialError)` on an unrecoverable parse error
    fn resume_partial(
        input: &mut S,
        partial: Self::Partial,
    ) -> Result<Packet<Self, Self::Partial>, PartialError>;
}

/// [`Vec`] implementation of [`Partial`] for element types that implement [`Partial`]
///
/// Finalises by calling [`Partial::finalize`] on every element in order
/// Returns the first `Err` encountered, or `Ok(Vec<P::Final>)` when all
/// elements finalise successfully
impl<P: Partial> Partial for Vec<P> {
    type Final = Vec<<P as Partial>::Final>;

    #[inline(always)]
    fn finalize(self) -> Result<Self::Final, PartialError> {
        self.into_iter().map(P::finalize).collect()
    }
}

/// [`Vec`] implementation of [`DecodePartial`] for all `T` that implement [`DecodePartial`]
///
/// Parses inner items until one returns [`Packet::NeedMore`], consumes no byte, or propagates an
/// error raised after consuming bytes. A [`Packet::NeedMore`] from the inner parser is treated as
/// "done for now": whatever was accumulated is best-effort-finalised and returned as
/// `Packet::Ready`. The cursor has been rewound by the inner call so the next invocation can resume
/// with more bytes. A cut element that cannot finalise is not pushed: the cursor rewinds to its
/// start, so the next invocation reads it whole
///
/// An element that consumed zero bytes is never pushed, under the same rule as `Vec<T>`'s
/// [`DecodeValue`] impl; the loop stops and returns what was accumulated
impl<S, T> DecodePartial<S> for Vec<T>
where
    S: winnow::stream::Stream,
    T: DecodePartial<S>,
{
    type Partial = Vec<<T as DecodePartial<S>>::Partial>;

    fn decode_partial(input: &mut S) -> Result<Packet<Self, Self::Partial>, PartialError> {
        let mut acc = Vec::new();
        loop {
            let before = input.eof_offset();
            let cp = input.checkpoint();
            let decoded = T::decode_partial(input);
            // --------------------------------------------------
            // nothing consumed: rewind and stop without pushing, the input would yield this
            // element forever
            // --------------------------------------------------
            if input.eof_offset() == before {
                input.reset(&cp);
                return Ok(Packet::Ready(acc));
            }
            match decoded {
                // --------------------------------------------------
                // push the val and continue looping
                // --------------------------------------------------
                Ok(Packet::Ready(val)) => acc.push(val),
                // --------------------------------------------------
                // return the acc values, but try to coerce the
                // last partial packet if possible; one that cannot
                // finalise rewinds to its start for the next call
                // --------------------------------------------------
                Ok(Packet::NeedMore(partial)) => {
                    if let Ok(last_elem) = partial.finalize() {
                        acc.push(last_elem);
                    } else {
                        input.reset(&cp);
                    }
                    return Ok(Packet::Ready(acc));
                }
                // --------------------------------------------------
                // bytes consumed: propagate the error outward
                // --------------------------------------------------
                Err(err) => return Err(err),
            }
        }
    }
}
