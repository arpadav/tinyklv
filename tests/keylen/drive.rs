//! Generic decode drivers shared by the fused-`keylen` test files
//!
//! [`Drive`] runs a derived type through `decode_partial` / `resume_partial`, `decode_value`,
//! or both, and either asserts the outcome or reports the first unexpected one;
//! [`ContextErrorExt`] reads the labels a one-shot error carries
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
use std::fmt::Debug;
use winnow::error::{ContextError, StrContext};

/// Label queries on a one-shot decode error
trait ContextErrorExt {
    /// The labels of the error's context chain
    ///
    /// # Returns
    ///
    /// Every `StrContext::Label` of the error, in the order the decode added them
    fn labels(&self) -> impl Iterator<Item = &'static str>;
}

impl ContextErrorExt for ContextError {
    fn labels(&self) -> impl Iterator<Item = &'static str> {
        self.context().filter_map(|context| match context {
            StrContext::Label(label) => Some(*label),
            _ => None,
        })
    }
}

/// Decode drivers for every type, each callable once the type implements the decode trait it
/// drives
pub(crate) trait Drive {
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
    /// * `needles` - the substrings the streaming label must contain, and one label of the
    ///   one-shot error's context chain must contain together
    ///
    /// # Panics
    ///
    /// If either driver returns anything but an error naming every needle
    fn assert_rejects_naming<'w>(wire: &'w [u8], needles: &[&str])
    where
        Self: DecodePartial<&'w [u8]> + Debug,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        let names_all = |label: &str| needles.iter().all(|needle| label.contains(needle));
        let mut failures = Vec::new();
        match Self::decode_partial(&mut &wire[..]) {
            Err(label) if names_all(label) => {}
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

    /// Asserts the one-shot decode of `wire` fails with exactly two context labels: the value
    /// decoder's own, then the derive's naming the field
    ///
    /// # Arguments
    ///
    /// * `wire` - the input, expected to fail
    /// * `decoder_label` - the label the value decoder adds
    /// * `field` - the `<Struct>::<field>` the derive's label must contain
    ///
    /// # Panics
    ///
    /// If the decode succeeds, or its context chain is not `decoder_label` then `field`
    fn assert_oneshot_label_chain<'w>(wire: &'w [u8], decoder_label: &str, field: &str)
    where
        Self: DecodeValue<&'w [u8]> + Debug,
    {
        let chain: Vec<_> = match Self::decode_value(&mut &wire[..]) {
            Err(error) => error.labels().collect(),
            other => panic!("expected an Err for {wire:02X?}, got {other:?}"),
        };
        let ok = matches!(
            chain[..],
            [decoder, derived] if decoder == decoder_label && derived.contains(field)
        );
        assert!(ok, "context labels: {chain:?}");
    }
}

impl<T> Drive for T {}
