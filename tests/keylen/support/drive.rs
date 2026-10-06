//! Generic codec drivers shared by the fused-`keylen` test files
//!
//! [`Drive`] encodes a derived type into a fresh buffer, or runs it through `decode_partial` /
//! `resume_partial`, `decode_value`, or both, and either asserts the outcome or reports the
//! first unexpected one; [`ContextErrorExt`] reads the labels, display and cause chain a
//! one-shot error carries
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::traits::ResumePartial;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::{error::Error, fmt::Debug, iter};
use winnow::error::{ContextError, StrContext};

/// Queries on a one-shot decode error
pub(crate) trait ContextErrorExt {
    /// The labels of the error's context chain
    ///
    /// # Returns
    ///
    /// Every `StrContext::Label` of the error, field first: a derived error keeps only the first
    /// label the value decoder added, and lists it after the label naming the field
    fn labels(&self) -> impl Iterator<Item = &'static str>;

    /// Whether the error's display contains every needle, each after the one before it
    ///
    /// # Arguments
    ///
    /// * `needles` - the substrings, in the order the display must name them
    fn names_in_order(&self, needles: &[&str]) -> bool;

    /// The error's cause chain
    ///
    /// # Returns
    ///
    /// The error's cause, then each error down that cause's `source` chain; empty if the error
    /// has no cause
    fn causes(&self) -> impl Iterator<Item = &(dyn Error + 'static)>;

    /// The outermost error of type `E` in the error's cause chain
    ///
    /// # Returns
    ///
    /// The first error of [`ContextErrorExt::causes`] that is an `E`; none if no error of the
    /// chain is
    fn find_cause<E: Error + 'static>(&self) -> Option<&E> {
        self.causes().find_map(|cause| cause.downcast_ref())
    }
}

/// [`ContextError`] implementation of [`ContextErrorExt`]
impl ContextErrorExt for ContextError {
    fn labels(&self) -> impl Iterator<Item = &'static str> {
        self.context().filter_map(|context| match context {
            StrContext::Label(label) => Some(*label),
            _ => None,
        })
    }

    fn names_in_order(&self, needles: &[&str]) -> bool {
        let display = self.to_string();
        needles
            .iter()
            .try_fold(display.as_str(), |rest, needle| {
                rest.split_once(needle).map(|(_, after)| after)
            })
            .is_some()
    }

    fn causes(&self) -> impl Iterator<Item = &(dyn Error + 'static)> {
        let cause = self.cause().map(|cause| cause as &(dyn Error + 'static));
        // the deref copies the chain's reference out, so `source` borrows for the chain's lifetime
        iter::successors(cause, |cause| (*cause).source())
    }
}

/// Codec drivers for every type, each callable once the type implements the codec trait it
/// drives
pub(crate) trait Drive {
    /// Encodes `self` into a fresh buffer
    ///
    /// # Returns
    ///
    /// The bytes `encode_value` appends to an empty buffer
    fn encoded(&self) -> Vec<u8>
    where
        Self: EncodeValue,
    {
        let mut out = Vec::new();
        self.encode_value(&mut out);
        out
    }

    /// Streams `cursor` to its end and asserts it pauses with `NeedMore`, then finalizes to
    /// `self`
    ///
    /// # Arguments
    ///
    /// * `cursor` - the input; left wherever the streaming decode stopped
    ///
    /// # Panics
    ///
    /// If the decode does not pause, or the partial does not finalize to `self`
    #[track_caller]
    fn assert_streams_to<'w>(&self, cursor: &mut &'w [u8])
    where
        Self: DecodePartial<&'w [u8]> + Debug + PartialEq,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        match Self::decode_partial(cursor) {
            Ok(Packet::NeedMore(partial)) => assert_eq!(partial.finalize().as_ref(), Ok(self)),
            other => panic!("expected NeedMore, got {other:?}"),
        }
    }

    /// Feeds `wire` as two chunks split at `at`, resuming on the unconsumed bytes of the first
    /// chunk plus the second, and finalizes the partial at the end
    ///
    /// # Arguments
    ///
    /// * `wire` - the complete encoding of one `Self`
    /// * `at` - the offset of the chunk boundary, `0..=wire.len()`
    ///
    /// # Returns
    ///
    /// The finalized `Self`, or a description of the first unexpected outcome
    fn feed_split<'w>(wire: &'w [u8], at: usize) -> Result<Self, String>
    where
        Self: ResumePartial<&'w [u8]> + Debug,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        // --------------------------------------------------
        // first chunk: must pause, never finish or fail
        // --------------------------------------------------
        let (mut cursor, _) = wire.split_at(at);
        let partial = match Self::decode_partial(&mut cursor) {
            Ok(Packet::NeedMore(partial)) => partial,
            other => return Err(format!("first chunk: {other:?}")),
        };
        // --------------------------------------------------
        // second chunk: the unconsumed head and the tail are contiguous in `wire`
        // --------------------------------------------------
        let (_, mut cursor) = wire.split_at(at - cursor.len());
        match Self::resume_partial(&mut cursor, partial) {
            Ok(Packet::NeedMore(partial)) if cursor.is_empty() => partial
                .finalize()
                .map_err(|label| format!("finalize: {label}")),
            other => Err(format!(
                "second chunk: {other:?}, {} bytes left",
                cursor.len()
            )),
        }
    }

    /// Asserts every split offset of `wire` decodes through [`Drive::feed_split`] to `self`
    ///
    /// # Arguments
    ///
    /// * `wire` - the complete encoding of `self`
    ///
    /// # Panics
    ///
    /// If any split does not decode to `self`, listing every failing offset with its reason
    #[track_caller]
    fn assert_every_split_completes<'w>(&self, wire: &'w [u8])
    where
        Self: ResumePartial<&'w [u8]> + Debug + PartialEq,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        let failures: Vec<_> = (0..=wire.len())
            .filter_map(|at| match Self::feed_split(wire, at) {
                Ok(decoded) if decoded == *self => None,
                Ok(decoded) => Some((at, format!("decoded {decoded:?}"))),
                Err(reason) => Some((at, reason)),
            })
            .collect();
        assert!(
            failures.is_empty(),
            "split offsets that failed: {failures:#?}"
        );
    }

    /// Asserts both drivers reject `wire` with an error naming every needle, never a decoded
    /// value
    ///
    /// # Arguments
    ///
    /// * `wire` - the input, decoded once per driver
    /// * `needles` - the substrings the streaming label must contain, matched anywhere in the
    ///   whole label with `contains`, and one label of the one-shot error's context chain must
    ///   contain together
    ///
    /// # Panics
    ///
    /// If either driver returns anything but an error naming every needle, or the streaming
    /// label does not start with `invalid `
    #[track_caller]
    fn assert_rejects_naming<'w>(wire: &'w [u8], needles: &[&str])
    where
        Self: DecodePartial<&'w [u8]> + Debug,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        let names_all = |label: &str| needles.iter().all(|needle| label.contains(needle));
        let mut failures = Vec::new();
        match Self::decode_partial(&mut &wire[..]) {
            Err(label) if label.starts_with("invalid ") && names_all(label) => {}
            other => failures.push(format!("streaming: {other:?}")),
        }
        match Self::decode_value(&mut &wire[..]) {
            Err(error) if error.labels().any(names_all) => {}
            other => failures.push(format!("one-shot: {other:?}")),
        }
        assert!(
            failures.is_empty(),
            "expected an Err naming {needles:?} for {wire:02X?}: {failures:#?}"
        );
    }

    /// The error the one-shot decode of `wire` fails with
    ///
    /// # Arguments
    ///
    /// * `wire` - the input, expected to fail
    ///
    /// # Panics
    ///
    /// If the decode succeeds
    #[track_caller]
    fn oneshot_error<'w>(wire: &'w [u8]) -> ContextError
    where
        Self: DecodeValue<&'w [u8]> + Debug,
    {
        match Self::decode_value(&mut &wire[..]) {
            Err(error) => error,
            other => panic!("expected an Err for {wire:02X?}, got {other:?}"),
        }
    }

    /// Asserts the one-shot decode of `wire` fails with exactly two context labels: the
    /// derive's naming the field, then the value decoder's own
    ///
    /// # Arguments
    ///
    /// * `wire` - the input, expected to fail
    /// * `field` - the `<Struct>::<field>` the derive's label must contain
    /// * `decoder_label` - the label the value decoder adds
    ///
    /// # Panics
    ///
    /// If the decode succeeds, or its context chain is not `field` then `decoder_label`
    #[track_caller]
    fn assert_oneshot_label_chain<'w>(wire: &'w [u8], field: &str, decoder_label: &str)
    where
        Self: DecodeValue<&'w [u8]> + Debug,
    {
        let chain: Vec<_> = Self::oneshot_error(wire).labels().collect();
        let ok = matches!(
            chain[..],
            [derived, decoder] if derived.contains(field) && decoder == decoder_label
        );
        assert!(ok, "context labels: {chain:?}");
    }
}

/// [`Drive`] implementation for every type
impl<T> Drive for T {}
