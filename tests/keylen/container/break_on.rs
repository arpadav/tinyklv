//! `#[klv(break_on = ..)]` on a fused `#[klv(keylen(..))]` container, through both drivers
//!
//! The key literal ends the loop right after the header of the frame carrying that key: the header
//! stays consumed, its value is never read, the frames before it decode and the frames after it
//! do not. That holds for a key a field declares too,
//! even one whose `size(exact = N)` the frame's length breaks. Without the terminator, every
//! frame decodes and streaming waits for more. A fn is handed the input right after the header,
//! and the loop ends where the fn leaves it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::support::example::Status;
use crate::support::{header, value};
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;

/// A message sent with frames on both sides of where its terminator goes
trait Terminated {
    /// The frames before the terminator
    const BEFORE: &'static [u8];

    /// The frames after the terminator, which a loop that ran past it would decode
    const AFTER: &'static [u8];

    /// The wire with a terminator frame between [`Terminated::BEFORE`] and
    /// [`Terminated::AFTER`]
    ///
    /// # Arguments
    ///
    /// * `end_header` - the terminator's frame header
    /// * `end_value` - the terminator's value bytes, which follow its header
    ///
    /// # Returns
    ///
    /// The wire, and the offset just past `end_header`, where the decode stops
    fn wire(end_header: &[u8], end_value: &[u8]) -> (Vec<u8>, usize) {
        let stop = Self::BEFORE.len() + end_header.len();
        let wire = [Self::BEFORE, end_header, end_value, Self::AFTER].concat();
        (wire, stop)
    }
}

/// Message keyed by field number that ends at field 15, whichever wire type carries it
///
/// It also sets `deny_unknown_keys`, and field 15 is not one of its keys, so the terminator must
/// end the loop before the unknown-key check runs
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_field),
    allow_unimplemented_encode,
    deny_unknown_keys,
    break_on = 15,
    default(typ = i32, dec = varint::dec_i32),
    default(typ = String, dec = value::dec_string),
)]
struct Logbook {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 1)]
    count: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 2)]
    label: Option<String>,

    /// field 3, wire 0 VARINT - `Inline` custom enum
    #[klv(key = 3, dec = Status::decode_value)]
    status: Option<Status>,
}

/// [`Logbook`] implementation
impl Logbook {
    /// What the frames before the terminator decode to
    fn before() -> Self {
        Self {
            count: 3,
            label: Some("hi".to_owned()),
            status: None,
        }
    }
}

/// [`Logbook`] implementation of [`Terminated`]
impl Terminated for Logbook {
    #[rustfmt::skip]
    const BEFORE: &'static [u8] = &[
        // count = 3 (field 1, wire 0 VARINT, Inline)
        0x08, 0x03,
        // label = "hi" (field 2, wire 2 LEN, Sliced(2))
        0x12, 0x02, 0x68, 0x69,
    ];

    #[rustfmt::skip]
    const AFTER: &'static [u8] = &[
        // count = 9 (field 1, wire 0 VARINT, Inline)
        0x08, 0x09,
        // status = Error (field 3, wire 0 VARINT, Inline)
        0x18, 0x02,
    ];
}

/// A checksum, a little-endian `u32` on the wire
#[derive(Debug, Clone, Copy, PartialEq)]
struct Crc32(u32);

/// [`Crc32`] implementation of [`DecodeValue`]: a little-endian `u32`
impl DecodeValue<&[u8]> for Crc32 {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        decb::le_u32.map(Self).parse_next(input)
    }
}

/// Message keyed by field number whose checksum, field 1, is both a declared `exact` field and
/// the terminator, as a checksum-last packet sends it
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_field),
    allow_unimplemented_encode,
    break_on = 1,
    default(typ = String, dec = value::dec_string),
)]
struct Sealed {
    /// field 1, a fixed32 of `size(exact = 4)` - the terminator, so its value is never read
    #[klv(key = 1, dec = Crc32::decode_value, size(exact = 4))]
    checksum: Option<Crc32>,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 2)]
    label: Option<String>,

    /// field 3, wire 0 VARINT - `Inline` custom enum
    #[klv(key = 3, dec = Status::decode_value)]
    status: Option<Status>,
}

/// [`Sealed`] implementation
impl Sealed {
    /// What the frames before the terminator decode to
    fn before() -> Self {
        Self {
            checksum: None,
            label: Some("hi".to_owned()),
            status: None,
        }
    }
}

/// [`Sealed`] implementation of [`Terminated`]
impl Terminated for Sealed {
    #[rustfmt::skip]
    const BEFORE: &'static [u8] = &[
        // label = "hi" (field 2, wire 2 LEN, Sliced(2))
        0x12, 0x02, 0x68, 0x69,
    ];

    #[rustfmt::skip]
    const AFTER: &'static [u8] = &[
        // status = Error (field 3, wire 0 VARINT, Inline)
        0x18, 0x02,
    ];
}

/// [`Sealed`] ended by a fn that reads the checksum after its header
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_field),
    allow_unimplemented_encode,
    break_on = Checked::end_after_checksum,
    default(typ = String, dec = value::dec_string),
)]
struct Checked {
    /// field 1, a fixed32 of `size(exact = 4)` - read by the fn, never by the field
    #[klv(key = 1, dec = Crc32::decode_value, size(exact = 4))]
    checksum: Option<Crc32>,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 2)]
    label: Option<String>,

    /// field 3, wire 0 VARINT - `Inline` custom enum
    #[klv(key = 3, dec = Status::decode_value)]
    status: Option<Status>,
}

/// [`Checked`] implementation
impl Checked {
    /// Ends the message once the checksum after field 1's header is read
    ///
    /// # Arguments
    ///
    /// * `field` - the frame's field number
    /// * `input` - the input right after the frame's header
    ///
    /// # Returns
    ///
    /// `Done` past the checksum, else `Proceed`, a checksum cut short included, which then
    /// decodes as a frame cut short
    fn end_after_checksum(field: &u32, input: &mut &[u8]) -> BreakType {
        if *field != 1 {
            return BreakType::Proceed;
        }
        match Crc32::decode_value(input) {
            Ok(_) => BreakType::Done,
            Err(_) => BreakType::Proceed,
        }
    }
}

/// A terminator framed `Sliced` stops the decode after its header: its length is read, its
/// value bytes are not
#[test]
fn sliced_terminator_stops_after_its_header() {
    #[rustfmt::skip]
    let (wire, stop) = Logbook::wire(
        // end (field 15, wire 2 LEN, Sliced(2))
        &[0x7A, 0x02],
        // the value, never read
        &[0xAA, 0xBB],
    );
    Logbook::before().assert_stops_at(&wire, stop);
}

/// A terminator framed `Inline` stops the decode after its header too, its value never read
#[test]
fn inline_terminator_stops_after_its_header() {
    #[rustfmt::skip]
    let (wire, stop) = Logbook::wire(
        // end (field 15, wire 0 VARINT, Inline)
        &[0x78],
        // the value, never read
        &[0x05],
    );
    Logbook::before().assert_stops_at(&wire, stop);
}

/// Without the terminator the decode runs to the end of the input: one-shot decodes every frame,
/// and streaming, at every chunk split, waits for more
#[test]
fn a_terminator_that_never_arrives_stops_nothing() {
    let wire = [Sealed::BEFORE, Sealed::AFTER].concat();
    let expected = Sealed {
        status: Some(Status::Error),
        ..Sealed::before()
    };
    expected.assert_decodes_from(&wire);
}

/// A terminator that is a declared field's key stops the decode after its header, before its
/// field's value or `size(exact = 4)` check: a frame of another length, even one running past the
/// end of the input, ends the decode rather than being rejected
#[test]
fn a_terminator_stops_before_its_exact_check() {
    #[rustfmt::skip]
    let headers: [&[u8]; 2] = [
        // checksum (field 1, wire 2 LEN, Sliced(2))
        &[0x0A, 0x02],
        // checksum (field 1, wire 2 LEN, Sliced(64)), more than the input holds
        &[0x0A, 0x40],
    ];
    for header in headers {
        // the value, never read
        let (wire, stop) = Sealed::wire(header, &[0xAA, 0xBB]);
        Sealed::before().assert_stops_at(&wire, stop);
    }
}

/// A fn reads past the header it follows, and the decode stops where it leaves the input: after
/// the checksum's value, which the field never decodes
#[test]
fn a_fn_stops_after_the_value_it_reads() {
    #[rustfmt::skip]
    let (wire, header_end) = Sealed::wire(
        // checksum (field 1, wire 5 I32, Sliced(4))
        &[0x0D],
        // its value, read by the fn
        &[0xEF, 0xBE, 0xAD, 0xDE],
    );
    let before = Checked {
        checksum: None,
        label: Some("hi".to_owned()),
        status: None,
    };
    before.assert_stops_at(&wire, header_end + 4);
}
