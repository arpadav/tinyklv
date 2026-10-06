//! Shared helpers for the BER codec tests
//!
//! [`ContextErrorExt`] pins which `Expected` context a BER error carries, and [`DecoderExt`]
//! asserts a decoder's outcome on one wire: the value and where it leaves the cursor
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use std::fmt::Debug;
use winnow::error::{ContextError, StrContext, StrContextValue};

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

    /// Asserts the decoder rejects `wire` as a value its type cannot hold, with the cursor
    /// rewound to the start of `wire`
    ///
    /// # Arguments
    ///
    /// * `wire` - the encoded value, all the decoder sees
    ///
    /// # Panics
    ///
    /// If the decode succeeds, fails without expecting a value that fits its type, or moves
    /// the cursor
    fn assert_rewinds(self, wire: &[u8]);

    /// Asserts the decoder runs out of bytes on `wire`, failing with the cursor at the end of
    /// input
    ///
    /// # Arguments
    ///
    /// * `wire` - the start of an encoded value, all the decoder sees
    ///
    /// # Panics
    ///
    /// If the decode succeeds, or fails with any of `wire` left unread
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
            Err(error) if error.expects_fits_target() => {}
            other => panic!("decoding {wire:02X?}: expected an overflow Err, got {other:?}"),
        }
        assert_eq!(cursor, wire, "the cursor after {wire:02X?}");
    }

    #[track_caller]
    fn assert_runs_out(mut self, wire: &[u8]) {
        let mut cursor = wire;
        let result = self(&mut cursor);
        assert!(
            result.is_err(),
            "decoding {wire:02X?}: expected an Err, got {result:?}"
        );
        assert!(
            cursor.is_empty(),
            "the cursor after {wire:02X?}: {cursor:02X?}"
        );
    }
}

/// `Expected`-context queries on a decoder's error
pub(super) trait ContextErrorExt {
    /// Whether the error's context chain carries `Expected(description)`
    ///
    /// # Arguments
    ///
    /// * `description` - the expected-value description to look for
    fn expects(&self, description: &str) -> bool;

    /// Whether a BER overflow error's context chain expects a value that fits the target type
    fn expects_fits_target(&self) -> bool {
        self.expects("a value that fits the target integer type")
    }

    /// Whether a BER length error's context chain expects the bytes its long form claims
    fn expects_length_bytes(&self) -> bool {
        self.expects("enough bytes in stream for length encoding")
    }
}

/// [`ContextError`] implementation of [`ContextErrorExt`]
impl ContextErrorExt for ContextError {
    fn expects(&self, description: &str) -> bool {
        self.context().any(|context| {
            matches!(
                context,
                StrContext::Expected(StrContextValue::Description(expected)) if *expected == description
            )
        })
    }
}
