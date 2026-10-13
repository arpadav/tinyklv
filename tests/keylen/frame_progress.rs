//! Per-frame progress for the fused `#[klv(keylen(..))]` codec, through both drivers
//!
//! A `keylen` header may read its key without consuming it, as a self-describing format peeks
//! the type byte its value decoder then reads. Such a frame decodes as long as the value moves
//! the cursor; one where header and value together consume nothing could only repeat, so it is
//! rejected. A header that consumed bytes has moved the cursor, so a `Sliced(0)` frame after it is
//! an empty value. [`Peeked`]'s header spends the [`HeaderBudget`], so a loop that repeats it
//! fails the test instead of hanging it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::support::example::Reading;
use crate::support::header_budget::HeaderBudget;
use crate::support::value;
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

    /// type byte `0x04` - `Sliced(0)`, so the type byte is never consumed
    #[klv(key = 0x04, dec = value::dec_string)]
    note: Option<String>,
}

/// [`Peeked`] implementation
impl Peeked {
    /// What both drivers' rejection of a frame that consumed no input names
    const NO_PROGRESS: &[&str] = &["`Peeked` frame", "consumed no input"];

    /// Decodes a frame header: the type byte as the key, left in the input for the value
    ///
    /// Type byte `0x04` frames `Sliced(0)`, every other `Inline`
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
            0x04 => Framing::Sliced(0),
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
            input.finish();
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
            note: None,
        }
    }
}

/// A header that consumes nothing, followed by an `Inline` value that does, decodes in both
/// drivers and across every split, including between a type byte and its value
#[test]
fn header_consuming_nothing_splits_anywhere() {
    Peeked::decoded().assert_decodes_from(&Peeked::wire());
}

/// A header that consumes nothing framing `Sliced(0)` makes no progress: both drivers reject it
#[test]
fn header_consuming_nothing_then_sliced_zero_is_rejected() {
    #[rustfmt::skip]
    let wire = [
        // small = 7 (type 0x01)
        0x01, 0x07,
        // note (type 0x04, Sliced(0)) - the type byte is never consumed
        0x04, 0x00,
    ];
    Peeked::assert_rejects_naming(&wire, Peeked::NO_PROGRESS);
}

/// A header that consumed bytes framing `Sliced(0)` has made progress: its empty value decodes,
/// in both drivers and at every chunk split
#[test]
fn header_consuming_bytes_then_sliced_zero_is_an_empty_value() {
    #[rustfmt::skip]
    let wire = [
        // count = 7 (field 1, wire 0 VARINT, Inline)
        0x08, 0x07,
        // label = "" (field 2, wire 2 LEN, Sliced(0))
        0x12, 0x00,
    ];
    let expected = Reading {
        count: 7,
        label: Some(String::new()),
        ..Reading::count_only()
    };
    expected.assert_decodes_from(&wire);
}
