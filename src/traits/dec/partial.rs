// --------------------------------------------------
// local
// --------------------------------------------------
use super::DecodeValue;
use crate::decoder::{DecodeIterError, Packet};

/// In-flight parse state that can be finalised into a complete value
///
/// Every [`DecodePartial`] impl has an associated [`DecodePartial::Partial`]
/// type that implements this trait. The partial is a mirror of the final
/// struct where every klv field is `Option<T>`; [`Partial::finalize`] runs
/// required-field validation and yields the final type on success
pub trait Partial: Sized {
    /// The finalised type this partial turns into.
    type Final;

    /// Validate required fields and produce the final value. Missing
    /// required fields produce a `&'static str` label naming the first
    /// missing field. The label is intentionally context-free: the
    /// partial has zero knowledge of the live input stream, so a richer
    /// error type would have to fabricate input/checkpoint slots. The
    /// [`crate::Decoder`] (or any caller with input access) wraps this
    /// label into a [`ContextError`] with real input + checkpoint at
    /// the point of failure
    fn finalize(self) -> Result<Self::Final, &'static str>;
}

/// An iterator over [`Partial`] values that can be resumed from a checkpoint
pub trait PartialIterator<P: Partial> {
    /// Resume-mode entry
    fn next_resume(&mut self) -> Result<P::Final, DecodeIterError>;

    /// Fresh-mode entry
    ///
    /// Existing sentinel-framed flow, unchanged in shape: scan sentinel,
    /// take declared body length, run `decode_partial` on the body slice.
    fn next_fresh(&mut self) -> Result<P::Final, DecodeIterError>;
}

/// Streaming-aware counterpart to [`DecodeValue`]
///
/// Returns a [`Packet<Self, Self::Partial>`] instead of a `Result<Self>`,
/// so callers can distinguish "need more bytes" from "malformed." Drives
/// [`crate::Decoder`], the user-facing streaming API.
///
/// Every implementor has an associated [`Self::Partial`] type: a mirror of
/// the struct in which every KLV field is `Option<T>`, i.e. a parse state
/// that can be fed incrementally and validated only at the end. The derive
/// macro emits this automatically as `Tinyklv<Name>PartialPacket`.
///
/// Automatically implemented for structs deriving [`tinyklv::Klv`](crate::Klv).
pub trait DecodePartial<S>: Sized + DecodeValue<S>
where
    S: winnow::stream::Stream,
{
    /// In-flight parse state. `Default` is the fresh (all-fields-missing)
    /// starting point; `Partial<Final = Self>` runs required-field
    /// validation at finalisation time.
    type Partial: Default + Partial<Final = Self>;

    /// Outer `Result` carries the `&'static str` label produced when
    /// the codegen hits an unrecoverable parse condition AND the
    /// partial-to-final conversion also fails. Successful conversion-
    /// on-give-up is just `Ok(Packet::Ready(t))`. Recoverable
    /// truncation surfaces as `Ok(Packet::NeedMore(p))`
    fn decode_partial(input: &mut S) -> Result<Packet<Self, Self::Partial>, &'static str>;
}

#[doc(hidden)]
/// Re-entry point for resuming a paused partial decode from an existing
/// [`DecodePartial::Partial`] state
///
/// This is used internally, and is not expected to be called directly
///
/// Generic over the stream type `S`, same as [`DecodePartial`]. The derive
/// macro emits this impl alongside `DecodePartial`; `decode_partial` itself
/// is just `resume_partial(input, Self::Partial::default())`
///
/// This is the main source-of-truth as per the decoding implementation,
/// since this is called by [`DecodePartial::decode_partial`], and by
/// proxy [`DecodeValue::decode_value`]
pub trait ResumePartial<S>: DecodePartial<S>
where
    S: winnow::stream::Stream,
{
    /// Continue a partial decode from `partial`, consuming input. Same
    /// outer `Result<Packet<...>, &'static str>` return shape as
    /// [`DecodePartial::decode_partial`] - this is the source-of-truth
    /// entry point; `decode_partial` is just `resume_partial(input,
    /// Default::default())`
    fn resume_partial(
        input: &mut S,
        partial: Self::Partial,
    ) -> Result<Packet<Self, Self::Partial>, &'static str>;
}

/// [`Vec<P>`] implementation of [`Partial`]
impl<P: Partial> Partial for Vec<P> {
    type Final = Vec<<P as Partial>::Final>;

    #[inline(always)]
    fn finalize(self) -> Result<Self::Final, &'static str> {
        self.into_iter().map(P::finalize).collect()
    }
}

/// [`Vec<T>`] implementation of [`DecodePartial`] for all `T` that implement [`DecodePartial`]
///
/// This parses inner items until one returns [`Packet::NeedMore`]
/// or [`Packet::Malformed`]. A [`Packet::NeedMore`] from the inner parser is
/// committed to the Vec as "done for now" (whatever was accumulated is returned as Ready)
///
/// The cursor has been rewound by the inner call, so the next invocation can resume with
/// more bytes
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
