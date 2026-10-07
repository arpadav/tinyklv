//! Per-frame progress for the fused `#[klv(keylen(..))]` codec, through both drivers
//!
//! A `keylen` header may read its key without consuming it, as a self-describing format peeks
//! the type byte its value decoder then reads. Such a frame decodes as long as the value moves
//! the cursor; one where header and value together consume nothing could only repeat, so it is
//! rejected. [`Peeked`]'s header spends the [`HeaderBudget`], so a loop that repeats it fails the
//! test instead of hanging it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::support::header_budget::HeaderBudget;
use crate::support::pb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::{ContextError, ParserError};

/// Self-describing message: every value leads with a type byte, which is the frame's key
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = Peeked::header),
    allow_unimplemented_encode,
)]
struct Peeked {
    /// type byte `0x01`, then one value byte - `Inline`
    #[klv(key = 0x01, dec = Peeked::small)]
    small: Option<u8>,

    /// type byte `0x02`, then a big-endian `u16` - `Inline`
    #[klv(key = 0x02, dec = Peeked::wide)]
    wide: Option<u16>,

    /// type byte `0x03`, read by a decoder that consumes nothing - `Inline`
    #[klv(key = 0x03, dec = Peeked::flag)]
    flag: Option<bool>,

    /// type byte `0x04` - `Sliced(0)`, so the type byte is never consumed
    #[klv(key = 0x04, dec = pb::dec_string)]
    note: Option<String>,
}

/// [`Peeked`] implementation
impl Peeked {
    /// What both drivers' rejection of a frame that consumed no input names
    const NO_PROGRESS: &[&str] = &["`Peeked` frame", "consumed no input"];

    /// Decodes a frame header: the type byte as the key, left in the input for the value
    ///
    /// Type bytes `0x04` and `0x05` frame `Sliced(0)`, every other `Inline`
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at a frame header; never advanced
    ///
    /// # Returns
    ///
    /// `Ok((type byte, framing))`, or an error on empty input
    ///
    /// # Panics
    ///
    /// On the read past the [`HeaderBudget`]
    fn header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
        HeaderBudget::spend();
        let Some(&kind) = input.first() else {
            return Err(ContextError::from_input(input));
        };
        let framing = match kind {
            0x04 | 0x05 => Framing::Sliced(0),
            _ => Framing::Inline,
        };
        Ok((kind, framing))
    }

    /// Decodes a type byte and the `N` bytes after it
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the type byte; advanced past the value on
    ///   success, and to the end of input when fewer than `N` bytes follow the type byte, so a
    ///   cut value is truncation
    ///
    /// # Returns
    ///
    /// The `N` bytes after the type byte, or an error when the input ends first
    fn typed<const N: usize>(input: &mut &[u8]) -> tinyklv::Result<[u8; N]> {
        let Some((value, rest)) = input
            .split_first()
            .and_then(|(_, rest)| rest.split_first_chunk::<N>())
        else {
            *input = &[];
            return Err(ContextError::from_input(input));
        };
        let value = *value;
        *input = rest;
        Ok(value)
    }

    /// Decodes the `small` value: its type byte, then one byte
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the type byte
    ///
    /// # Returns
    ///
    /// The value byte, or the error of [`Peeked::typed`]
    fn small(input: &mut &[u8]) -> tinyklv::Result<u8> {
        Self::typed(input).map(|[value]| value)
    }

    /// Decodes the `wide` value: its type byte, then a big-endian `u16`
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the type byte
    ///
    /// # Returns
    ///
    /// The `u16`, or the error of [`Peeked::typed`]
    fn wide(input: &mut &[u8]) -> tinyklv::Result<u16> {
        Self::typed(input).map(u16::from_be_bytes)
    }

    /// Decodes the `flag` value from its type byte alone, without consuming it
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the type byte; never advanced
    ///
    /// # Returns
    ///
    /// `Ok(true)` while the type byte is there, or an error on empty input
    fn flag(input: &mut &[u8]) -> tinyklv::Result<bool> {
        input
            .first()
            .map(|_| true)
            .ok_or_else(|| ContextError::from_input(input))
    }

    /// The wire of [`Peeked::decoded`]: two frames whose headers consume nothing
    #[rustfmt::skip]
    fn wire() -> Vec<u8> {
        vec![
            // small = 7 (type 0x01)
            0x01, 0x07,
            // wide = 300 (type 0x02)
            0x02, 0x01, 0x2C,
        ]
    }

    /// The decoded [`Peeked`] matching [`Peeked::wire`]
    fn decoded() -> Self {
        Self {
            small: Some(7),
            wide: Some(300),
            flag: None,
            note: None,
        }
    }
}

/// A header that consumes nothing, followed by an `Inline` value that does, decodes in both
/// drivers to the end of input
#[test]
fn header_consuming_nothing_then_inline_value_decodes() {
    let wire = Peeked::wire();
    let mut cursor = &wire[..];
    assert_eq!(Peeked::decode_value(&mut cursor), Ok(Peeked::decoded()));
    assert!(cursor.is_empty(), "one-shot stops at {cursor:02X?}");
    let mut cursor = &wire[..];
    Peeked::decoded().assert_streams_to(&mut cursor);
    assert!(cursor.is_empty(), "streaming stops at {cursor:02X?}");
}

/// A header that consumes nothing streams across every split, including between a type byte
/// and its value
#[test]
fn header_consuming_nothing_splits_anywhere() {
    Peeked::decoded().assert_decodes_from(&Peeked::wire());
}

/// A header that consumes nothing framing `Sliced(0)` makes no progress, for a known key and
/// an unknown one alike: both drivers reject it
#[test]
fn header_consuming_nothing_then_sliced_zero_is_rejected() {
    #[rustfmt::skip]
    let known = [
        // small = 7 (type 0x01)
        0x01, 0x07,
        // note (type 0x04, Sliced(0)) - the type byte is never consumed
        0x04, 0x00,
    ];
    #[rustfmt::skip]
    let unknown = [
        // small = 7 (type 0x01)
        0x01, 0x07,
        // unknown type 0x05, Sliced(0) - the type byte is never consumed
        0x05, 0x00,
    ];
    for wire in [&known[..], &unknown[..]] {
        Peeked::assert_rejects_naming(wire, Peeked::NO_PROGRESS);
    }
}

/// A header that consumes nothing, followed by an `Inline` value that consumes nothing either,
/// makes no progress: both drivers reject it
#[test]
fn header_and_inline_value_consuming_nothing_is_rejected() {
    #[rustfmt::skip]
    let wire = [
        // small = 7 (type 0x01)
        0x01, 0x07,
        // flag (type 0x03) - read without consuming the type byte
        0x03, 0x00,
    ];
    Peeked::assert_rejects_naming(&wire, Peeked::NO_PROGRESS);
}
