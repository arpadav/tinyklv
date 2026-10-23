//! Shared helpers for the BER codec tests
//!
//! [`DecoderExt`] asserts a decoder's outcome on one wire: the value and where it leaves the
//! cursor
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::error::ContextErrorExt;
use tinyklv::ErrorKind;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::fmt::Debug;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The byte the decoder sees after a value it must accept, and must leave unread
const TRAILER: u8 = 0xA5;

/// Outcome assertions for a decoder run over one wire, each on the value and on the cursor
pub(super) trait DecoderExt {
    /// What the decoder reads
    type Value;

    /// Asserts the decoder reads `wire` to `value` and stops at its end
    ///
    /// # Arguments
    ///
    /// * `wire` - the encoded value; the decoder sees it followed by [`TRAILER`]
    /// * `value` - what `wire` decodes to
    ///
    /// # Panics
    ///
    /// If the decode fails, yields another value, or leaves the cursor anywhere but on the
    /// trailer
    fn assert_reads(self, wire: &[u8], value: Self::Value);

    /// Asserts the decoder rejects `wire` as a value its type cannot hold, of kind
    /// [`ErrorKind::MalformedValue`], with the cursor rewound to the start of `wire`
    ///
    /// # Arguments
    ///
    /// * `wire` - the encoded value, all the decoder sees
    ///
    /// # Panics
    ///
    /// If the decode succeeds, fails without expecting a value that fits its type or of another
    /// kind, or moves the cursor
    fn assert_rewinds(self, wire: &[u8]);

    /// Asserts the decoder runs out of bytes on `wire`, failing with an error of kind
    /// [`ErrorKind::Truncated`] and the cursor at the end of input
    ///
    /// # Arguments
    ///
    /// * `wire` - the start of an encoded value, all the decoder sees
    ///
    /// # Panics
    ///
    /// If the decode succeeds, fails with another kind, or fails with any of `wire` left unread
    fn assert_runs_out(self, wire: &[u8]);
}

/// [`DecoderExt`] implementation for every decoder over a byte slice that yields a comparable value
impl<T, F> DecoderExt for F
where
    T: Debug + PartialEq,
    F: FnMut(&mut &[u8]) -> tinyklv::Result<T>,
{
    type Value = T;

    #[track_caller]
    fn assert_reads(mut self, wire: &[u8], value: T) {
        let input = [wire, &[TRAILER]].concat();
        let mut cursor = &input[..];
        assert_eq!(self(&mut cursor), Ok(value), "decoding {wire:02X?}");
        assert_eq!(cursor, [TRAILER], "the cursor after {wire:02X?}");
    }

    #[track_caller]
    fn assert_rewinds(mut self, wire: &[u8]) {
        let mut cursor = wire;
        match self(&mut cursor) {
            Err(error)
                if error.expects_fits_target()
                    && ErrorKind::of(&error) == Some(ErrorKind::MalformedValue) => {}
            other => panic!("decoding {wire:02X?}: expected an overflow Err, got {other:?}"),
        }
        assert_eq!(cursor, wire, "the cursor after {wire:02X?}");
    }

    #[track_caller]
    fn assert_runs_out(mut self, wire: &[u8]) {
        let mut cursor = wire;
        let result = self(&mut cursor);
        assert_eq!(
            result.as_ref().err().and_then(ErrorKind::of),
            Some(ErrorKind::Truncated),
            "decoding {wire:02X?}: {result:?}"
        );
        assert!(
            cursor.is_empty(),
            "the cursor after {wire:02X?}: {cursor:02X?}"
        );
    }
}
