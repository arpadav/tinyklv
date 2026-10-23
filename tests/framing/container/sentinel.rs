//! A key + len struct's `sentinel(..)`: the packet length framed by a codec of its own, or by the
//! container's
//!
//! The `len(..)` in `sentinel(..)` writes and reads each packet's length in place of the
//! container's `len(..)`, which frames each item; without one, the container's frames the packet
//! too. A packet body that does not fit its length's byte budget set by `size(exact = N)` panics
//! in a debug build, and is left out, sentinel and all, in a release build
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

/// Where a reading was taken, a key + len struct of its own
#[derive(Klv, Debug, Clone, Copy, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Position {
    #[klv(key = 0x01, dec = decb::be_i16, enc = *encb::be_i16)]
    lat: i16,
    #[klv(key = 0x02, dec = decb::be_i16, enc = *encb::be_i16)]
    lon: i16,
}

/// A reading behind `READ`: each packet's length two big-endian bytes, each item's one byte
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel(
        value = b"READ",
        len(dec = decb::be_u16_as_usize, enc = encb::be_u16_from_usize),
    ),
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Reading {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    seq: u8,
    #[klv(key = 0x02, dec = Position::decode_value, enc = Position::encode_value)]
    at: Position,
}

/// [`Reading`] implementation
impl Reading {
    /// The reading [`ITEMS`] encodes
    const SAMPLE: Self = Self {
        seq: 7,
        at: Position { lat: 258, lon: -2 },
    };

    /// A packet's sentinel and its length, that of [`ITEMS`]
    #[rustfmt::skip]
    const HEADER: [u8; 6] = [
        // sentinel
        b'R', b'E', b'A', b'D',
        // packet length, two bytes big-endian
        0x00, 0x0D,
    ];
}

/// [`Reading`] with no `len(..)` of the sentinel's own: the container's frames the packet
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel(value = b"READ"),
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Report {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    seq: u8,
    #[klv(key = 0x02, dec = Position::decode_value, enc = Position::encode_value)]
    at: Position,
}

/// [`Report`] implementation
impl Report {
    /// The report [`ITEMS`] encodes
    const SAMPLE: Self = Self {
        seq: Reading::SAMPLE.seq,
        at: Reading::SAMPLE.at,
    };

    /// A packet's sentinel and its length, that of [`ITEMS`]
    #[rustfmt::skip]
    const HEADER: [u8; 5] = [
        // sentinel
        b'R', b'E', b'A', b'D',
        // packet length, one byte as each item's
        0x0D,
    ];
}

/// The items of [`Reading::SAMPLE`] and [`Report::SAMPLE`] alike
#[rustfmt::skip]
const ITEMS: [u8; 13] = [
    // seq = 7
    0x01, 0x01, 0x07,
    // at, 8 bytes
    0x02, 0x08,
    // lat = 258
    0x01, 0x02, 0x01, 0x02,
    // lon = -2
    0x02, 0x02, 0xFF, 0xFE,
];

/// A log line behind `LOG`, whose packet length is one byte by `size(exact = 1)`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel(
        value = b"LOG",
        len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    ),
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Log {
    #[klv(key = 0x01, dec = decs::to_string_utf8, enc = &encs::from_string_utf8, size(lengthed))]
    note: String,
}

/// The longest note one length byte counts: its item, with its key and length, is two bytes more
const LONGEST_NOTE: usize = u8::MAX as usize;

/// What a note is filled with
const FILLER: &str = "x";

/// A packet's sentinel's `len(..)` writes and reads the packet length in place of the
/// container's, two bytes where each item's is one
#[test]
fn a_sentinel_len_frames_the_packet_in_place_of_the_container_len() {
    let mut wire = Vec::new();
    Reading::SAMPLE.encode_frame(&mut wire);
    assert_eq!(wire, [Reading::HEADER.as_slice(), &ITEMS].concat());
    let mut input = wire.as_slice();
    let reading = Reading::decode_frame(&mut input).expect("the packet decodes");
    assert_eq!(reading, Reading::SAMPLE);
    assert!(input.is_empty(), "{input:02X?}");
}

/// Without a `len(..)` of the sentinel's own, the container's frames the packet as it does each
/// item
#[test]
fn without_a_sentinel_len_the_container_len_frames_the_packet() {
    let mut wire = Vec::new();
    Report::SAMPLE.encode_frame(&mut wire);
    assert_eq!(wire, [Report::HEADER.as_slice(), &ITEMS].concat());
    let mut input = wire.as_slice();
    let report = Report::decode_frame(&mut input).expect("the packet decodes");
    assert_eq!(report, Report::SAMPLE);
    assert!(input.is_empty(), "{input:02X?}");
}

/// A packet body one length byte cannot count, though its one item fits its own: a debug build
/// panics naming the packet, and a release build writes nothing, not even the sentinel
#[test]
#[cfg_attr(
    debug_assertions,
    should_panic(
        expected = "tinyklv: packet body of 257 bytes does not fit the 1-byte length budget set by `size(exact = 1)`"
    )
)]
fn a_packet_body_over_its_length_budget_panics_in_debug_and_is_left_out_in_release() {
    let log = Log {
        note: FILLER.repeat(LONGEST_NOTE),
    };
    let mut wire = Vec::new();
    log.encode_frame(&mut wire);
    assert!(wire.is_empty(), "{wire:02X?}");
}
