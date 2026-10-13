//! [`Drive`], the generic codec drivers a derived type is tested through
//!
//! A driver encodes a value into a fresh buffer, or decodes a wire through one-shot
//! `decode_value`, streaming `decode_partial` / `resume_partial`, or both, and asserts the
//! outcome. The streaming assertions cut the wire into two chunks at every offset
//! ([`Drive::stream_split`]), and list every run that went wrong
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::error::ContextErrorExt;
use tinyklv::ErrorKind;
use tinyklv::prelude::*;
use tinyklv::traits::{PartialError, ResumePartial};

// --------------------------------------------------
// external
// --------------------------------------------------
use std::fmt::Debug;
use winnow::error::ContextError;

/// The outcome of one streaming run
type Streamed<T, P> = Result<Packet<T, P>, PartialError>;

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

    /// Streams `wire` cut into two chunks at `at`: the first chunk, then, when it pauses with
    /// `NeedMore`, its unconsumed bytes plus the second chunk
    ///
    /// # Arguments
    ///
    /// * `wire` - the input
    /// * `at` - the offset of the chunk boundary, `0..=wire.len()`
    ///
    /// # Returns
    ///
    /// The last outcome, and the bytes of `wire` it left unconsumed
    fn stream_split<'w>(
        wire: &'w [u8],
        at: usize,
    ) -> (
        Streamed<Self, <Self as DecodePartial<&'w [u8]>>::Partial>,
        usize,
    )
    where
        Self: ResumePartial<&'w [u8]>,
    {
        let (mut cursor, _) = wire.split_at(at);
        match Self::decode_partial(&mut cursor) {
            Ok(Packet::NeedMore(partial)) => {
                let (_, mut cursor) = wire.split_at(at - cursor.len());
                let outcome = Self::resume_partial(&mut cursor, partial);
                (outcome, cursor.len())
            }
            other => (other, cursor.len() + wire.len() - at),
        }
    }

    /// Asserts both drivers decode all of `wire` to `self`
    ///
    /// One-shot must consume every byte. Streaming, at every chunk split, must pause with
    /// `NeedMore` at the end of the wire and finalize to `self`: with no sentinel or `break_on`,
    /// a streaming decode never knows its last frame has arrived
    ///
    /// # Arguments
    ///
    /// * `wire` - the complete encoding of `self`
    ///
    /// # Panics
    ///
    /// If any run does not decode to `self`, listing every failing run
    #[track_caller]
    fn assert_decodes_from<'w>(&self, wire: &'w [u8])
    where
        Self: ResumePartial<&'w [u8]> + Debug + PartialEq,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        let mut failures = Vec::new();
        // --------------------------------------------------
        // one-shot, over the whole wire
        // --------------------------------------------------
        let mut cursor = wire;
        match Self::decode_value(&mut cursor) {
            Ok(decoded) if decoded == *self && cursor.is_empty() => {}
            other => failures.push(format!("one-shot: {other:?}, {} bytes left", cursor.len())),
        }
        // --------------------------------------------------
        // streaming, split at every offset
        // --------------------------------------------------
        for at in 0..=wire.len() {
            match Self::stream_split(wire, at) {
                (Ok(Packet::NeedMore(partial)), 0) => match partial.finalize() {
                    Ok(decoded) if decoded == *self => {}
                    other => failures.push(format!("split at {at}: finalized {other:?}")),
                },
                (other, left) => failures.push(format!("split at {at}: {other:?}, {left} left")),
            }
        }
        assert!(
            failures.is_empty(),
            "expected {self:?} from {wire:02X?}: {failures:#?}"
        );
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

    /// Asserts both drivers decode `wire` to `self` and stop at `stop`, leaving the bytes from
    /// there on unconsumed
    ///
    /// Streaming runs at every chunk split: a first chunk that reaches `stop` stops in it, any
    /// other pauses with `NeedMore` and stops in the resumed second chunk
    ///
    /// # Arguments
    ///
    /// * `wire` - the input; its frames from `stop` on are never decoded
    /// * `stop` - the offset the decode ends at, `0..=wire.len()`
    ///
    /// # Panics
    ///
    /// If any run does not stop at `stop` with `self`, listing every failing run
    #[track_caller]
    fn assert_stops_at<'w>(&self, wire: &'w [u8], stop: usize)
    where
        Self: ResumePartial<&'w [u8]> + Debug + PartialEq,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        let mut failures = Vec::new();
        // --------------------------------------------------
        // one-shot, over the whole wire
        // --------------------------------------------------
        let mut cursor = wire;
        match Self::decode_value(&mut cursor) {
            Ok(decoded) if decoded == *self && wire.len() - cursor.len() == stop => {}
            other => failures.push(format!("one-shot: {other:?}, {} bytes left", cursor.len())),
        }
        // --------------------------------------------------
        // streaming, split at every offset
        // --------------------------------------------------
        for at in 0..=wire.len() {
            let (outcome, left) = Self::stream_split(wire, at);
            let stopped = wire.len() - left;
            match outcome {
                Ok(Packet::Ready(decoded)) if decoded == *self && stopped == stop => {}
                other => failures.push(format!("split at {at}: {other:?}, stopped at {stopped}")),
            }
        }
        assert!(
            failures.is_empty(),
            "expected {self:?}, stopped at {stop} of {wire:02X?}: {failures:#?}"
        );
    }

    /// Asserts both drivers reject `wire` with an error naming every needle, never a decoded
    /// value and never a pause for more bytes
    ///
    /// # Arguments
    ///
    /// * `wire` - the input; one-shot decodes it whole, streaming at every chunk split
    /// * `needles` - the substrings every rejection contains: the streaming label, which also
    ///   starts with `invalid `, and one label of the one-shot error's context chain
    ///
    /// # Panics
    ///
    /// If any run returns anything but such an error, listing every failing run
    #[track_caller]
    fn assert_rejects_naming<'w>(wire: &'w [u8], needles: &[&str])
    where
        Self: ResumePartial<&'w [u8]> + Debug,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        let names_all = |label: &str| needles.iter().all(|needle| label.contains(needle));
        let mut failures = Vec::new();
        // --------------------------------------------------
        // one-shot: one label of the context chain names every needle
        // --------------------------------------------------
        match Self::decode_value(&mut &wire[..]) {
            Err(error) if error.labels().any(names_all) => {}
            other => failures.push(format!("one-shot: {other:?}")),
        }
        // --------------------------------------------------
        // streaming, split at every offset
        // --------------------------------------------------
        for at in 0..=wire.len() {
            match Self::stream_split(wire, at) {
                (Err(err), _) if err.label().starts_with("invalid ") && names_all(err.label()) => {}
                (other, left) => failures.push(format!("split at {at}: {other:?}, {left} left")),
            }
        }
        assert!(
            failures.is_empty(),
            "expected an Err naming {needles:?} for {wire:02X?}: {failures:#?}"
        );
    }

    /// Asserts both drivers reject `wire` with exactly `message`, leaving the cursor `at` bytes
    /// into it
    ///
    /// Streaming returns `message` after an `invalid ` prefix at every chunk split, and stops at
    /// `at` at every split from `at` on: a split before it can cut short a frame the whole wire
    /// holds, which is then rejected at its start. One-shot carries `message` as its error's only
    /// label
    ///
    /// # Arguments
    ///
    /// * `wire` - the input; one-shot decodes it whole, streaming at every chunk split
    /// * `at` - the offset the rejection leaves the cursor at
    /// * `message` - the rejection
    ///
    /// # Panics
    ///
    /// If any run returns anything but `message`, or leaves the cursor elsewhere, listing every
    /// failing run
    #[track_caller]
    fn assert_rejects_at<'w>(wire: &'w [u8], at: usize, message: &str)
    where
        Self: ResumePartial<&'w [u8]> + Debug,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        let mut failures = Vec::new();
        // --------------------------------------------------
        // streaming, split at every offset
        // --------------------------------------------------
        for split in 0..=wire.len() {
            let (outcome, left) = Self::stream_split(wire, split);
            let stopped = wire.len() - left;
            match outcome {
                Err(err)
                    if err.label().strip_prefix("invalid ") == Some(message)
                        && (split < at || stopped == at) => {}
                other => {
                    failures.push(format!("split at {split}: {other:?}, stopped at {stopped}"));
                }
            }
        }
        // --------------------------------------------------
        // one-shot
        // --------------------------------------------------
        let mut cursor = wire;
        match Self::decode_value(&mut cursor) {
            Err(error) if error.labels().eq([message]) => {}
            other => failures.push(format!("one-shot: {other:?}")),
        }
        let stopped = wire.len() - cursor.len();
        if stopped != at {
            failures.push(format!("one-shot stopped at {stopped}"));
        }
        assert!(
            failures.is_empty(),
            "expected {message:?} at {at} of {wire:02X?}: {failures:#?}"
        );
    }

    /// Asserts both drivers reject `wire` with an error of `kind`
    ///
    /// # Arguments
    ///
    /// * `wire` - the input; one-shot decodes it whole, streaming at every chunk split
    /// * `kind` - what the rejection is
    ///
    /// # Panics
    ///
    /// If any run returns anything but an error of `kind`, listing every failing run
    #[track_caller]
    fn assert_rejects_as<'w>(wire: &'w [u8], kind: ErrorKind)
    where
        Self: ResumePartial<&'w [u8]> + Debug,
        <Self as DecodePartial<&'w [u8]>>::Partial: Debug,
    {
        let mut failures = Vec::new();
        // --------------------------------------------------
        // one-shot, its error tagged with `kind`
        // --------------------------------------------------
        match Self::decode_value(&mut &wire[..]) {
            Err(error) if ErrorKind::of(&error) == Some(kind) => {}
            other => failures.push(format!("one-shot: {other:?}")),
        }
        // --------------------------------------------------
        // streaming, split at every offset
        // --------------------------------------------------
        for at in 0..=wire.len() {
            match Self::stream_split(wire, at) {
                (Err(error), _) if error.kind() == kind => {}
                (other, left) => failures.push(format!("split at {at}: {other:?}, {left} left")),
            }
        }
        assert!(
            failures.is_empty(),
            "expected {kind:?} for {wire:02X?}: {failures:#?}"
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
