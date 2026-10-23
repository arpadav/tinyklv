//! The [`ErrorKind`] of each rejection a derived decode returns
//!
//! One test per kind. One-shot decode tags its error with the kind, which [`ErrorKind::of`] reads;
//! streaming decode returns a [`PartialError`] of it, at every chunk split. A kind that more than
//! one exit of the decode loop returns is tested at each
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::Result;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::prelude::*;
use tinyklv::traits::{PartialError, SeekSentinel};

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::{ContextError, ParserError};

/// A gauge reading in key + len mode that rejects a key no field declares, under `strict`, with
/// a `break_on` on the struct and on its `level`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel = b"GAUGE",
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    break_on = Gauge::halt,
    deny_unknown_keys,
    strict,
    allow_unimplemented_encode,
)]
struct Gauge {
    /// Sensor id
    #[klv(key = 0x01, dec = decb::u8)]
    id: u8,

    /// Sampling mode, two bytes wide
    #[klv(key = 0x02, dec = decb::be_u16, size(exact = 2))]
    mode: Option<u16>,

    /// Display name
    #[klv(key = 0x03, dec = decs::to_string_utf8_strict, size(lengthed))]
    name: Option<String>,

    /// Fill level, percent
    #[klv(key = 0x04, dec = decb::u8, break_on = Gauge::in_range)]
    level: Option<u8>,
}

/// [`Gauge`] implementation
impl Gauge {
    /// id = 7, the first frame of every wire
    const ID: &[u8] = &[0x01, 0x01, 0x07];

    /// The struct's `break_on`: key `0xFE` aborts the decode
    ///
    /// # Arguments
    ///
    /// * `key` - the frame's key
    /// * `_input` - the input right after the key, left unread
    ///
    /// # Returns
    ///
    /// `Abort` on key `0xFE`, `Proceed` on any other
    fn halt(key: &u8, _input: &mut &[u8]) -> BreakType {
        match key {
            0xFE => BreakType::Abort("halt key"),
            _ => BreakType::Proceed,
        }
    }

    /// The `level` field's `break_on`: a level above 100 aborts the decode
    ///
    /// # Arguments
    ///
    /// * `level` - the decoded level
    ///
    /// # Returns
    ///
    /// `Abort` above 100, `Proceed` otherwise
    fn in_range(level: &u8) -> BreakType {
        if *level > 100 {
            BreakType::Abort("level above 100")
        } else {
            BreakType::Proceed
        }
    }
}

/// A struct in keylen mode whose header frames key `0x01` `Inline`, and peeks any other key
/// without consuming it, framing it `Sliced(0)`
#[derive(Klv, Debug, PartialEq)]
#[klv(keylen(dec = Hollow::header), allow_unimplemented_encode)]
struct Hollow {
    /// A name, whose decoder takes the value's length
    #[klv(key = 0x01, dec = decs::to_string_utf8_strict, size(lengthed))]
    name: Option<String>,
}

/// [`Hollow`] implementation
impl Hollow {
    /// Decodes a frame header: key `0x01`, framed `Inline`, or any other key, peeked and framed
    /// `Sliced(0)`
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at a frame header
    ///
    /// # Returns
    ///
    /// The key and its framing, or an error on empty input
    fn header(input: &mut &[u8]) -> Result<(u8, Framing)> {
        match input.first() {
            Some(0x01) => Ok((decb::u8(input)?, Framing::Inline)),
            Some(&key) => Ok((key, Framing::Sliced(0))),
            None => Err(ContextError::from_input(input)),
        }
    }
}

/// A key no field declares, under `deny_unknown_keys`, is an unknown key
#[test]
fn an_undeclared_key_is_an_unknown_key() {
    let wire = [Gauge::ID, &[0x09, 0x01, 0x00]].concat();
    Gauge::assert_rejects_as(&wire, ErrorKind::UnknownKey);
}

/// Under `strict`, a value its decoder rejects, and one it leaves bytes of unread, are malformed
#[test]
fn a_rejected_or_partly_read_value_is_malformed() {
    // name: 2 bytes, not UTF-8
    let rejected = [Gauge::ID, &[0x03, 0x02, 0xFF, 0xFE]].concat();
    Gauge::assert_rejects_as(&rejected, ErrorKind::MalformedValue);
    // id: 2 bytes, of which its decoder reads 1
    Gauge::assert_rejects_as(&[0x01, 0x02, 0x07, 0x08], ErrorKind::MalformedValue);
}

/// A frame length other than the field's `size(exact = N)`, and an `Inline` item for a
/// `lengthed` field, which carries no length, are the wrong length
#[test]
fn a_length_the_value_cannot_have_is_the_wrong_length() {
    // mode: 3 bytes, not 2
    let mismatched = [Gauge::ID, &[0x02, 0x03, 0x00, 0x00, 0x00]].concat();
    Gauge::assert_rejects_as(&mismatched, ErrorKind::WrongLength);
    // name: `Inline`, "A"
    Hollow::assert_rejects_as(&[0x01, 0x41], ErrorKind::WrongLength);
}

/// An item whose header and value together consumed no input is no progress
#[test]
fn an_item_that_consumed_nothing_is_no_progress() {
    // key 0x07, peeked, framed `Sliced(0)`
    Hollow::assert_rejects_as(&[0x07], ErrorKind::NoProgress);
}

/// A `break_on` that returns `Abort`, on the struct or on a field, aborts
#[test]
fn an_abort_is_aborted() {
    // the struct's halt key
    let halted = [Gauge::ID, &[0xFE]].concat();
    Gauge::assert_rejects_as(&halted, ErrorKind::Aborted);
    // level = 255
    let out_of_range = [Gauge::ID, &[0x04, 0x01, 0xFF]].concat();
    Gauge::assert_rejects_as(&out_of_range, ErrorKind::Aborted);
}

/// A required field with no value once the input is decoded is missing: one-shot finalizes it,
/// streaming pauses with the partial, which finalizes to the same kind
#[test]
fn a_required_value_absent_is_missing_required() {
    // level = 50, no id
    let wire = [0x04, 0x01, 0x32];
    let error = Gauge::oneshot_error(&wire);
    assert_eq!(
        ErrorKind::of(&error),
        Some(ErrorKind::MissingRequired),
        "{error:?}"
    );
    for at in 0..=wire.len() {
        match Gauge::stream_split(&wire, at) {
            (Ok(Packet::NeedMore(partial)), 0) => assert_eq!(
                partial.finalize().err().map(PartialError::kind),
                Some(ErrorKind::MissingRequired),
                "split at {at}",
            ),
            (other, left) => panic!("split at {at}: {other:?}, {left} left"),
        }
    }
}

/// Input holding no sentinel is a missing sentinel
#[test]
fn a_seek_without_a_sentinel_is_a_missing_sentinel() {
    let error = Gauge::seek_sentinel(&mut &b"GAUG"[..]).expect_err("no sentinel");
    assert_eq!(
        ErrorKind::of(&error),
        Some(ErrorKind::MissingSentinel),
        "{error:?}"
    );
}

/// A sentinel whose packet length, or whose body, the end of the input cuts is truncated
#[test]
fn a_seek_cut_after_its_sentinel_is_truncated() {
    // the sentinel alone, then a body of 1 of its 3 bytes
    for wire in [&b"GAUGE"[..], b"GAUGE\x03\x01"] {
        let error = Gauge::seek_sentinel(&mut &wire[..]).expect_err("a cut packet");
        assert_eq!(
            ErrorKind::of(&error),
            Some(ErrorKind::Truncated),
            "{wire:02X?}: {error:?}"
        );
    }
}
