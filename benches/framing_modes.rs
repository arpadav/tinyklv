//! The decode cost of three framing choices, each measured between structs that differ only in
//! that choice: a varint value with a length prefix against one without, opting in to
//! `unknown(..)`, and opting in to `strict`; the decode cost of a `keylen` struct's `break_on`
//! fn; and the encode cost of the `keylen` frame header
//!
//! Every decode benchmark is a one-shot `decode_value` over a wire encoded ahead of time, checked
//! to decode to its expected value with every byte read before anything is timed:
//!
//! * `varint_decode_value` - four varints (`u32`, `u32`, `i32`, `u64`) under one-byte varint keys.
//!   [`PrefixedVarints`] frames each as key, length, value; [`ImplicitVarints`], under
//!   `size(implicit)`, as key, value, the bytes protobuf writes for them. The two differ by the
//!   length read and the `take` it bounds
//! * `unknown_decode_value` - eight fixed-width fields in key + len mode, a one-byte key and a
//!   one-byte length. [`FixedWidths`] has no `unknown(..)`, [`FixedWidthsDroppingUnknown`] has it
//!   and no `#[klv(unknown)]` field, [`FixedWidthsKeepingUnknown`] keeps the frames in one. All
//!   three decode the wire with no unknown frame; the two that opt in also decode it with two
//!   unknown frames after the known ones
//! * `strict_decode_value` - a key + len struct with two strings and a nested struct,
//!   [`Contact`], and a keylen protobuf-shaped one with a varint, a float, a string and a nested
//!   message, [`Shape`]. [`StrictContact`] and [`StrictShape`], and the structs nested in them,
//!   set `strict`. Each pair decodes the same valid wire, so the gap is what `strict` costs a
//!   decode that passes it
//! * `break_on_decode_value` - two keylen structs with a `break_on` fn, [`stop::at_tag`], that
//!   lets every item of their wires proceed: [`BreakOnShape`], [`Shape`] under the fn, and
//!   [`BreakOnGauges`], four `Sliced(4)` values. Its arms differ in shape, so it compares one tree
//!   against another, not one arm against another
//!
//! Every encode benchmark is an `encode_value` into a cleared buffer, checked to write its expected
//! wire before anything is timed:
//!
//! * `keylen_encode_value` - messages under the [`proto::enc_tag`] header, the tag, then a length
//!   for a wire 2 LEN value alone: [`KeylenFixedWidths`], three fixed-width values;
//!   [`KeylenVarints`], eight varints; [`KeylenNamed`], a varint and a string of 2, 32 and 300
//!   bytes; [`KeylenNested`], a varint and a nested [`KeylenNamed`]. Its arms differ in shape,
//!   so it compares one tree against another, not one arm against another
//!
//! Lower is faster
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

// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use std::{any::type_name, fmt::Debug, hint::black_box};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Criterion samples per benchmark, as in `benches/suite/mod.rs`: far above the default 100, so
/// the confidence intervals are tight enough to order arms a few nanoseconds apart
const SAMPLE_SIZE: usize = 1000;

/// Bench id parameter of [`FIXED_WIDTHS_WIRE`]
const KNOWN_ONLY: &str = "known_only";

/// Bench id parameter of [`FIXED_WIDTHS_WIRE`] followed by [`UNKNOWN_FRAMES`]
const TWO_UNKNOWN: &str = "two_unknown";

/// Bench id parameter of [`CONTACT_WIRE`]
const KEY_LEN: &str = "key_len";

/// Bench id parameter of [`SHAPE_WIRE`]
const KEYLEN: &str = "keylen";

/// The bytes of [`FixedWidths::DECODED`]: each field after its key and its one-byte length
#[rustfmt::skip]
const FIXED_WIDTHS_WIRE: &[u8] = &[
    // channel: key, length 1, 3
    0x01, 0x01, 0x03,
    // sequence: key, length 2, 1_042
    0x02, 0x02, 0x04, 0x12,
    // uptime: key, length 4, 86_400
    0x03, 0x04, 0x00, 0x01, 0x51, 0x80,
    // timestamp: key, length 8, 1_760_000_000_000_000
    0x04, 0x08, 0x00, 0x06, 0x40, 0xB5, 0xEE, 0xCE, 0x00, 0x00,
    // temperature: key, length 2, -1_250
    0x05, 0x02, 0xFB, 0x1E,
    // altitude: key, length 4, 152_400
    0x06, 0x04, 0x00, 0x02, 0x53, 0x50,
    // voltage: key, length 4, 3.3
    0x07, 0x04, 0x40, 0x53, 0x33, 0x33,
    // latitude: key, length 8, 47.4979
    0x08, 0x08, 0x40, 0x47, 0xBF, 0xBB, 0x2F, 0xEC, 0x56, 0xD6,
];

/// Two frames whose keys no field declares, as [`FixedWidthsKeepingUnknown::with_unknown_frames`]
/// keeps them
#[rustfmt::skip]
const UNKNOWN_FRAMES: &[u8] = &[
    // key 0x20, length 3
    0x20, 0x03, 0x01, 0x04, 0x02,
    // key 0x21, length 2
    0x21, 0x02, 0x80, 0x01,
];

/// The bytes of [`Contact::decoded`]: each field after its key and its one-byte length
#[rustfmt::skip]
const CONTACT_WIRE: &[u8] = &[
    // id: key, length 4, 12_345
    0x01, 0x04, 0x00, 0x00, 0x30, 0x39,
    // name: key, length 5, "Alice"
    0x02, 0x05, 0x41, 0x6C, 0x69, 0x63, 0x65,
    // place: key, length 12
    0x03, 0x0C,
    // place.latitude: key, length 4, 47_497_912
    0x01, 0x04, 0x02, 0xD4, 0xC2, 0xB8,
    // place.longitude: key, length 4, 19_040_235
    0x02, 0x04, 0x01, 0x22, 0x87, 0xEB,
    // email: key, length 17, "alice@example.org"
    0x04, 0x11,
    0x61, 0x6C, 0x69, 0x63, 0x65, 0x40, 0x65, 0x78, 0x61, 0x6D, 0x70, 0x6C, 0x65, 0x2E, 0x6F, 0x72,
    0x67,
];

/// The bytes of [`Shape::decoded`]: each field after its tag, and a length-delimited one after its
/// length
#[rustfmt::skip]
const SHAPE_WIRE: &[u8] = &[
    // id: field 1 VARINT, 150
    0x08, 0x96, 0x01,
    // scale: field 2 I32, 0.75
    0x15, 0x00, 0x00, 0x40, 0x3F,
    // name: field 3 LEN, length 5, "Alice"
    0x1A, 0x05, 0x41, 0x6C, 0x69, 0x63, 0x65,
    // origin: field 4 LEN, length 9
    0x22, 0x09,
    // origin.x: field 1 VARINT, 300
    0x08, 0xAC, 0x02,
    // origin.label: field 2 LEN, length 4, "home"
    0x12, 0x04, 0x68, 0x6F, 0x6D, 0x65,
];

/// LEB128 varint codecs: seven value bits per byte, low group first, the high bit set on every
/// byte but the last
mod varint {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use tinyklv::Result;
    use tinyklv::prelude::*;

    // --------------------------------------------------
    // external
    // --------------------------------------------------
    use winnow::error::{ContextError, ParserError};

    // --------------------------------------------------
    // constants
    // --------------------------------------------------
    /// The bit of a varint byte that says another byte follows
    const CONTINUATION: u8 = 0x80;

    /// Value bits each varint byte carries
    const PAYLOAD_BITS: u32 = 7;

    /// The shift of a 10th varint byte, which holds only bit 63 of a `u64`
    const LAST_SHIFT: u32 = 63;

    /// Decodes one varint as a `u64`
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced past the varint on success, to the end
    ///   of the input when it ends first, and left on a 10th byte carrying bits past the 64th
    ///
    /// # Returns
    ///
    /// `Ok(u64)`, or an error on truncation or on a 10th byte carrying bits past the 64th
    pub(super) fn dec_u64(input: &mut &[u8]) -> Result<u64> {
        let mut value = 0;
        let mut shift = 0;
        loop {
            let Some((&byte, rest)) = input.split_first() else {
                return Err(ContextError::from_input(input));
            };
            if shift == LAST_SHIFT && byte > 1 {
                return Err(ContextError::from_input(input));
            }
            *input = rest;
            value |= u64::from(byte & !CONTINUATION) << shift;
            if byte & CONTINUATION == 0 {
                return Ok(value);
            }
            shift += PAYLOAD_BITS;
        }
    }

    /// Decodes one varint as a `u32`, a key or a counter
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced as [`dec_u64`] advances it, and
    ///   rewound to the varint's start when the value does not fit a `u32`
    ///
    /// # Returns
    ///
    /// `Ok(u32)`, or the error of [`dec_u64`], or an error on a value past `u32::MAX`
    pub(super) fn dec_u32(input: &mut &[u8]) -> Result<u32> {
        dec_u64.try_map(u32::try_from).parse_next(input)
    }

    /// Decodes one varint as a `usize`, a value length
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced as [`dec_u64`] advances it, and
    ///   rewound to the varint's start when the value does not fit a `usize`
    ///
    /// # Returns
    ///
    /// `Ok(usize)`, or the error of [`dec_u64`], or an error on a value past `usize::MAX`
    pub(super) fn dec_usize(input: &mut &[u8]) -> Result<usize> {
        dec_u64.try_map(usize::try_from).parse_next(input)
    }

    /// Decodes one varint as an `i32`: the low 32 bits of the `u64`, so a negative value's
    /// ten-byte sign extension reads back
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced as [`dec_u64`] advances it
    ///
    /// # Returns
    ///
    /// `Ok(i32)`, or the error of [`dec_u64`]
    pub(super) fn dec_i32(input: &mut &[u8]) -> Result<i32> {
        dec_u64.map(|value| value as i32).parse_next(input)
    }

    /// Encodes one `u64` as a varint, the inverse of [`dec_u64`]
    ///
    /// # Arguments
    ///
    /// * `value` - the value to encode
    /// * `out` - the buffer the varint bytes are appended to
    pub(super) fn enc_u64(mut value: u64, out: &mut Vec<u8>) {
        while value >= u64::from(CONTINUATION) {
            out.push(value as u8 | CONTINUATION);
            value >>= PAYLOAD_BITS;
        }
        out.push(value as u8);
    }

    /// Encodes one `i32` as a varint, the inverse of [`dec_i32`]: a negative value sign-extends
    /// to ten bytes
    ///
    /// # Arguments
    ///
    /// * `value` - the value to encode
    /// * `out` - the buffer the varint bytes are appended to
    pub(super) fn enc_i32(value: i32, out: &mut Vec<u8>) {
        enc_u64(i64::from(value) as u64, out);
    }

    /// Encodes one `usize` as a varint, the inverse of [`dec_usize`]
    ///
    /// # Arguments
    ///
    /// * `value` - the value to encode
    /// * `out` - the buffer the varint bytes are appended to
    pub(super) fn enc_usize(value: usize, out: &mut Vec<u8>) {
        enc_u64(value as u64, out);
    }
}

/// The protobuf frame header a `keylen` encoder writes: the tag varint, whose low three bits are
/// the wire type, then the value length as a varint for wire 2 LEN alone
mod proto {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use super::varint;

    // --------------------------------------------------
    // constants
    // --------------------------------------------------
    /// The tag bits that carry the wire type
    const WIRE_TYPE_MASK: u32 = 0x07;

    /// The wire type of a length-delimited value
    const LEN: u32 = 2;

    /// Encodes one frame header
    ///
    /// # Arguments
    ///
    /// * `(tag, value_len)` - the field tag and the byte length of the value that follows
    /// * `out` - the buffer the header bytes are appended to
    pub(super) fn enc_tag((tag, value_len): (u32, usize), out: &mut Vec<u8>) {
        varint::enc_u64(u64::from(tag), out);
        if tag & WIRE_TYPE_MASK == LEN {
            varint::enc_usize(value_len, out);
        }
    }
}

/// The codec of a frame no field declares, in a struct whose every frame carries a one-byte
/// length: the value is its bytes
mod raw {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use tinyklv::Result;
    use tinyklv::dec::binary as decb;
    use tinyklv::enc::binary as encb;
    use tinyklv::prelude::*;

    // --------------------------------------------------
    // external
    // --------------------------------------------------
    use winnow::error::{ContextError, ParserError};

    /// Decodes the value of a key + len frame: everything after its key
    ///
    /// # Arguments
    ///
    /// * `_key` - the frame's key; every frame carries a length, so the key does not decide how
    ///   the value is framed
    ///
    /// # Returns
    ///
    /// A decoder reading the length, then that many bytes; it fails on a missing length, or with
    /// the input consumed to its end when fewer bytes remain, so a decode loop reads truncation
    pub(super) fn dec(_key: u8) -> impl Fn(&mut &[u8]) -> Result<Vec<u8>> {
        |input| {
            let len = decb::u8_as_usize(input)?;
            let Some((value, rest)) = input.split_at_checked(len) else {
                input.finish();
                return Err(ContextError::from_input(input));
            };
            *input = rest;
            Ok(value.to_vec())
        }
    }

    /// Encodes the value of a key + len frame, the inverse of [`dec`]: its length, then its bytes
    ///
    /// # Arguments
    ///
    /// * `value` - the bytes to encode
    /// * `out` - the buffer the length and bytes are appended to
    pub(super) fn enc(value: &[u8], out: &mut Vec<u8>) {
        encb::u8_from_usize(value.len(), out);
        out.extend_from_slice(value);
    }
}

/// The protobuf frame header the keylen structs read: a varint tag, whose low three bits say how
/// the value after it is framed
mod tag {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use super::varint;
    use tinyklv::Result;
    use tinyklv::prelude::*;

    // --------------------------------------------------
    // external
    // --------------------------------------------------
    use winnow::error::{ContextError, ParserError};

    // --------------------------------------------------
    // constants
    // --------------------------------------------------
    /// Mask selecting the wire type from a tag's low three bits
    const WIRE_TYPE: u32 = 0b111;

    /// Wire type 0 VARINT: self-delimiting
    const VARINT: u32 = 0;

    /// Wire type 2 LEN: a varint length, then that many bytes
    const LEN: u32 = 2;

    /// Wire type 5 I32: four bytes
    const I32: u32 = 5;

    /// Decodes one frame header: the tag, then the framing its wire type gives the value
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at a frame header; advanced past it on success
    ///
    /// # Returns
    ///
    /// `Ok((tag, framing))`, the error of a varint read, or an error on a wire type no struct here
    /// uses
    pub(super) fn header(input: &mut &[u8]) -> Result<(u32, Framing)> {
        let tag = varint::dec_u32(input)?;
        let framing = match tag & WIRE_TYPE {
            VARINT => Framing::Inline,
            LEN => Framing::Sliced(varint::dec_usize(input)?),
            I32 => Framing::Sliced(size_of::<u32>()),
            _ => return Err(ContextError::from_input(input)),
        };
        Ok((tag, framing))
    }
}

/// The `break_on` fn of the keylen structs that set one: field 15 LEN ends the message, field 14
/// LEN is skipped, and every other item proceeds. It returns each of the three, so a decode keeps
/// the code of every decision, and reads nothing past the header
mod stop {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use tinyklv::prelude::*;

    // --------------------------------------------------
    // constants
    // --------------------------------------------------
    /// Field 15, wire 2 LEN: ends the message
    const END: u32 = 0x7A;

    /// Field 14, wire 2 LEN: skipped
    const PAD: u32 = 0x72;

    /// Decides what the decode does with the item after `tag`
    ///
    /// # Arguments
    ///
    /// * `tag` - the item's tag
    /// * `_input` - the input right after the item's header, left unread
    ///
    /// # Returns
    ///
    /// `Done` on [`END`], `Skip` on [`PAD`], else `Proceed`
    pub(super) fn at_tag(tag: &u32, _input: &mut &[u8]) -> BreakType {
        match *tag {
            END => BreakType::Done,
            PAD => BreakType::Skip,
            _ => BreakType::Proceed,
        }
    }
}

/// A radio link's counters, each varint after its key and its length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    default(typ = u32, dec = varint::dec_u32),
    default(typ = i32, dec = varint::dec_i32),
    default(typ = u64, dec = varint::dec_u64),
    allow_unimplemented_encode,
)]
struct PrefixedVarints {
    /// Packets sent
    #[klv(key = 0x08)]
    packets: u32,

    /// Packets sent again
    #[klv(key = 0x10)]
    retries: u32,

    /// Received signal strength, dBm
    #[klv(key = 0x18)]
    rssi: i32,

    /// Bytes sent
    #[klv(key = 0x20)]
    transferred: u64,
}

/// [`PrefixedVarints`] implementation
impl PrefixedVarints {
    /// The bytes of [`Self::DECODED`]
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // packets: key, length 2, 1_500
        0x08, 0x02, 0xDC, 0x0B,
        // retries: key, length 1, 3
        0x10, 0x01, 0x03,
        // rssi: key, length 10, -67 sign-extended
        0x18, 0x0A, 0xBD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
        // transferred: key, length 5, 6_000_000_000
        0x20, 0x05, 0x80, 0xF8, 0x82, 0xAD, 0x16,
    ];

    /// The counters [`Self::WIRE`] carries
    const DECODED: Self = Self {
        packets: 1_500,
        retries: 3,
        rssi: -67,
        transferred: 6_000_000_000,
    };
}

/// [`PrefixedVarints`] with every varint after its key alone, delimited by its own last byte
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    default(typ = u32, dec = varint::dec_u32, size(implicit)),
    default(typ = i32, dec = varint::dec_i32, size(implicit)),
    default(typ = u64, dec = varint::dec_u64, size(implicit)),
    allow_unimplemented_encode,
)]
struct ImplicitVarints {
    /// Packets sent
    #[klv(key = 0x08)]
    packets: u32,

    /// Packets sent again
    #[klv(key = 0x10)]
    retries: u32,

    /// Received signal strength, dBm
    #[klv(key = 0x18)]
    rssi: i32,

    /// Bytes sent
    #[klv(key = 0x20)]
    transferred: u64,
}

/// [`ImplicitVarints`] implementation
impl ImplicitVarints {
    /// The bytes of [`Self::DECODED`]
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // packets: key, 1_500
        0x08, 0xDC, 0x0B,
        // retries: key, 3
        0x10, 0x03,
        // rssi: key, -67 sign-extended
        0x18, 0xBD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
        // transferred: key, 6_000_000_000
        0x20, 0x80, 0xF8, 0x82, 0xAD, 0x16,
    ];

    /// The counters [`Self::WIRE`] carries
    const DECODED: Self = Self {
        packets: 1_500,
        retries: 3,
        rssi: -67,
        transferred: 6_000_000_000,
    };
}

/// A sensor sample of eight fixed-width fields, without `unknown(..)`: a frame no field declares
/// is skipped by its length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    default(typ = u64, dec = decb::be_u64, enc = *encb::be_u64),
    default(typ = i16, dec = decb::be_i16, enc = *encb::be_i16),
    default(typ = i32, dec = decb::be_i32, enc = *encb::be_i32),
    default(typ = f32, dec = decb::be_f32, enc = *encb::be_f32),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
struct FixedWidths {
    /// Radio channel
    #[klv(key = 0x01)]
    channel: u8,

    /// Sample sequence number
    #[klv(key = 0x02)]
    sequence: u16,

    /// Seconds since boot
    #[klv(key = 0x03)]
    uptime: u32,

    /// Microseconds since the Unix epoch
    #[klv(key = 0x04)]
    timestamp: u64,

    /// Hundredths of a degree Celsius
    #[klv(key = 0x05)]
    temperature: i16,

    /// Millimetres above sea level
    #[klv(key = 0x06)]
    altitude: i32,

    /// Supply voltage
    #[klv(key = 0x07)]
    voltage: f32,

    /// Degrees north
    #[klv(key = 0x08)]
    latitude: f64,
}

/// [`FixedWidths`] implementation
impl FixedWidths {
    /// The sample [`FIXED_WIDTHS_WIRE`] carries
    const DECODED: Self = Self {
        channel: 3,
        sequence: 1_042,
        uptime: 86_400,
        timestamp: 1_760_000_000_000_000,
        temperature: -1_250,
        altitude: 152_400,
        voltage: 3.3,
        latitude: 47.4979,
    };
}

/// [`FixedWidths`] with `unknown(..)` and no `#[klv(unknown)]` field: a frame no field declares is
/// decoded by [`raw::dec`] and dropped
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    unknown(dec = raw::dec, enc = raw::enc),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    default(typ = u64, dec = decb::be_u64, enc = *encb::be_u64),
    default(typ = i16, dec = decb::be_i16, enc = *encb::be_i16),
    default(typ = i32, dec = decb::be_i32, enc = *encb::be_i32),
    default(typ = f32, dec = decb::be_f32, enc = *encb::be_f32),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
struct FixedWidthsDroppingUnknown {
    /// Radio channel
    #[klv(key = 0x01)]
    channel: u8,

    /// Sample sequence number
    #[klv(key = 0x02)]
    sequence: u16,

    /// Seconds since boot
    #[klv(key = 0x03)]
    uptime: u32,

    /// Microseconds since the Unix epoch
    #[klv(key = 0x04)]
    timestamp: u64,

    /// Hundredths of a degree Celsius
    #[klv(key = 0x05)]
    temperature: i16,

    /// Millimetres above sea level
    #[klv(key = 0x06)]
    altitude: i32,

    /// Supply voltage
    #[klv(key = 0x07)]
    voltage: f32,

    /// Degrees north
    #[klv(key = 0x08)]
    latitude: f64,
}

/// [`FixedWidthsDroppingUnknown`] implementation
impl FixedWidthsDroppingUnknown {
    /// The sample [`FIXED_WIDTHS_WIRE`] carries, with or without [`UNKNOWN_FRAMES`] after it
    const DECODED: Self = Self {
        channel: 3,
        sequence: 1_042,
        uptime: 86_400,
        timestamp: 1_760_000_000_000_000,
        temperature: -1_250,
        altitude: 152_400,
        voltage: 3.3,
        latitude: 47.4979,
    };
}

/// [`FixedWidths`] with `unknown(..)` and a `#[klv(unknown)]` field: a frame no field declares is
/// decoded by [`raw::dec`] and kept
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    unknown(dec = raw::dec, enc = raw::enc),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    default(typ = u64, dec = decb::be_u64, enc = *encb::be_u64),
    default(typ = i16, dec = decb::be_i16, enc = *encb::be_i16),
    default(typ = i32, dec = decb::be_i32, enc = *encb::be_i32),
    default(typ = f32, dec = decb::be_f32, enc = *encb::be_f32),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
struct FixedWidthsKeepingUnknown {
    /// Radio channel
    #[klv(key = 0x01)]
    channel: u8,

    /// Sample sequence number
    #[klv(key = 0x02)]
    sequence: u16,

    /// Seconds since boot
    #[klv(key = 0x03)]
    uptime: u32,

    /// Microseconds since the Unix epoch
    #[klv(key = 0x04)]
    timestamp: u64,

    /// Hundredths of a degree Celsius
    #[klv(key = 0x05)]
    temperature: i16,

    /// Millimetres above sea level
    #[klv(key = 0x06)]
    altitude: i32,

    /// Supply voltage
    #[klv(key = 0x07)]
    voltage: f32,

    /// Degrees north
    #[klv(key = 0x08)]
    latitude: f64,

    /// Every other frame, its key and its bytes
    #[klv(unknown)]
    extra: Vec<(u8, Vec<u8>)>,
}

/// [`FixedWidthsKeepingUnknown`] implementation
impl FixedWidthsKeepingUnknown {
    /// The sample [`FIXED_WIDTHS_WIRE`] carries, keeping no frame
    const DECODED: Self = Self {
        channel: 3,
        sequence: 1_042,
        uptime: 86_400,
        timestamp: 1_760_000_000_000_000,
        temperature: -1_250,
        altitude: 152_400,
        voltage: 3.3,
        latitude: 47.4979,
        extra: Vec::new(),
    };

    /// The sample [`FIXED_WIDTHS_WIRE`] carries, keeping [`UNKNOWN_FRAMES`] after it
    ///
    /// # Returns
    ///
    /// [`Self::DECODED`] with the two frames in `extra`, in wire order
    fn with_unknown_frames() -> Self {
        Self {
            extra: vec![(0x20, vec![0x01, 0x04, 0x02]), (0x21, vec![0x80, 0x01])],
            ..Self::DECODED
        }
    }
}

/// Where a [`Contact`] is: two coordinates, each after its key and its one-byte length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    default(typ = i32, dec = decb::be_i32),
    allow_unimplemented_encode,
)]
struct Place {
    /// Microdegrees north
    #[klv(key = 0x01)]
    latitude: i32,

    /// Microdegrees east
    #[klv(key = 0x02)]
    longitude: i32,
}

/// A contact, each field after its key and its one-byte length: an id, a name, where it is and
/// an email address
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    default(typ = u32, dec = decb::be_u32),
    default(typ = String, dec = decs::to_string_utf8_strict, size(lengthed)),
    allow_unimplemented_encode,
)]
struct Contact {
    /// Identifier
    #[klv(key = 0x01)]
    id: u32,

    /// Name
    #[klv(key = 0x02)]
    name: String,

    /// Where the contact is
    #[klv(key = 0x03, dec = Place::decode_value)]
    place: Place,

    /// Email address
    #[klv(key = 0x04)]
    email: String,
}

/// [`Contact`] implementation
impl Contact {
    /// The contact [`CONTACT_WIRE`] carries
    fn decoded() -> Self {
        Self {
            id: 12_345,
            name: "Alice".to_owned(),
            place: Place {
                latitude: 47_497_912,
                longitude: 19_040_235,
            },
            email: "alice@example.org".to_owned(),
        }
    }
}

/// [`Place`] under `strict`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    default(typ = i32, dec = decb::be_i32),
    allow_unimplemented_encode,
    strict,
)]
struct StrictPlace {
    /// Microdegrees north
    #[klv(key = 0x01)]
    latitude: i32,

    /// Microdegrees east
    #[klv(key = 0x02)]
    longitude: i32,
}

/// [`Contact`] under `strict`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    default(typ = u32, dec = decb::be_u32),
    default(typ = String, dec = decs::to_string_utf8_strict, size(lengthed)),
    allow_unimplemented_encode,
    strict,
)]
struct StrictContact {
    /// Identifier
    #[klv(key = 0x01)]
    id: u32,

    /// Name
    #[klv(key = 0x02)]
    name: String,

    /// Where the contact is
    #[klv(key = 0x03, dec = StrictPlace::decode_value)]
    place: StrictPlace,

    /// Email address
    #[klv(key = 0x04)]
    email: String,
}

/// [`StrictContact`] implementation
impl StrictContact {
    /// The contact [`CONTACT_WIRE`] carries
    fn decoded() -> Self {
        Self {
            id: 12_345,
            name: "Alice".to_owned(),
            place: StrictPlace {
                latitude: 47_497_912,
                longitude: 19_040_235,
            },
            email: "alice@example.org".to_owned(),
        }
    }
}

/// A proto3 message nested in [`Shape`]: `int32 x = 1` (a varint) and `string label = 2`
/// (length-delimited)
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = tag::header),
    default(typ = i32, dec = varint::dec_i32),
    default(typ = String, dec = decs::to_string_utf8_strict, size(lengthed)),
    allow_unimplemented_encode,
)]
struct Point {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    x: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x12)]
    label: String,
}

/// A proto3 message: `int32 id = 1` (a varint), `float scale = 2` (four bytes), `string name = 3`
/// and `Point origin = 4` (length-delimited)
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = tag::header),
    default(typ = i32, dec = varint::dec_i32),
    default(typ = f32, dec = decb::le_f32),
    default(typ = String, dec = decs::to_string_utf8_strict, size(lengthed)),
    allow_unimplemented_encode,
)]
struct Shape {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire 5 I32 - `Sliced(4)`
    #[klv(key = 0x15)]
    scale: f32,

    /// field 3, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x1A)]
    name: String,

    /// field 4, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x22, dec = Point::decode_value)]
    origin: Point,
}

/// [`Shape`] implementation
impl Shape {
    /// The message [`SHAPE_WIRE`] carries
    fn decoded() -> Self {
        Self {
            id: 150,
            scale: 0.75,
            name: "Alice".to_owned(),
            origin: Point {
                x: 300,
                label: "home".to_owned(),
            },
        }
    }
}

/// [`Point`] under `strict`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = tag::header),
    default(typ = i32, dec = varint::dec_i32),
    default(typ = String, dec = decs::to_string_utf8_strict, size(lengthed)),
    allow_unimplemented_encode,
    strict,
)]
struct StrictPoint {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    x: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x12)]
    label: String,
}

/// [`Shape`] under `strict`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = tag::header),
    default(typ = i32, dec = varint::dec_i32),
    default(typ = f32, dec = decb::le_f32),
    default(typ = String, dec = decs::to_string_utf8_strict, size(lengthed)),
    allow_unimplemented_encode,
    strict,
)]
struct StrictShape {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire 5 I32 - `Sliced(4)`
    #[klv(key = 0x15)]
    scale: f32,

    /// field 3, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x1A)]
    name: String,

    /// field 4, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x22, dec = StrictPoint::decode_value)]
    origin: StrictPoint,
}

/// [`StrictShape`] implementation
impl StrictShape {
    /// The message [`SHAPE_WIRE`] carries
    fn decoded() -> Self {
        Self {
            id: 150,
            scale: 0.75,
            name: "Alice".to_owned(),
            origin: StrictPoint {
                x: 300,
                label: "home".to_owned(),
            },
        }
    }
}

/// [`Shape`] under the `break_on` fn [`stop::at_tag`], which lets every item of [`SHAPE_WIRE`]
/// proceed
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = tag::header),
    break_on = stop::at_tag,
    default(typ = i32, dec = varint::dec_i32),
    default(typ = f32, dec = decb::le_f32),
    default(typ = String, dec = decs::to_string_utf8_strict, size(lengthed)),
    allow_unimplemented_encode,
)]
struct BreakOnShape {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire 5 I32 - `Sliced(4)`
    #[klv(key = 0x15)]
    scale: f32,

    /// field 3, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x1A)]
    name: String,

    /// field 4, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x22, dec = Point::decode_value)]
    origin: Point,
}

/// [`BreakOnShape`] implementation
impl BreakOnShape {
    /// The message [`SHAPE_WIRE`] carries
    fn decoded() -> Self {
        Self {
            id: 150,
            scale: 0.75,
            name: "Alice".to_owned(),
            origin: Point {
                x: 300,
                label: "home".to_owned(),
            },
        }
    }
}

/// A power meter's gauges in `keylen` mode under the `break_on` fn [`stop::at_tag`], which lets
/// every item proceed: four fixed-width values, each `Sliced(4)` by its tag
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = tag::header),
    break_on = stop::at_tag,
    default(typ = u32, dec = decb::le_u32),
    default(typ = i32, dec = decb::le_i32),
    default(typ = f32, dec = decb::le_f32),
    allow_unimplemented_encode,
)]
struct BreakOnGauges {
    /// Seconds since boot, field 1, wire 5 I32
    #[klv(key = 0x0D)]
    uptime: u32,

    /// Supply voltage, field 2, wire 5 I32
    #[klv(key = 0x15)]
    voltage: f32,

    /// Hundredths of a degree Celsius, field 3, wire 5 I32
    #[klv(key = 0x1D)]
    temperature: i32,

    /// Watts drawn, field 4, wire 5 I32
    #[klv(key = 0x25)]
    power: f32,
}

/// [`BreakOnGauges`] implementation
impl BreakOnGauges {
    /// The bytes of [`Self::DECODED`]: each value after its tag
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // uptime: field 1 I32, 86_400
        0x0D, 0x80, 0x51, 0x01, 0x00,
        // voltage: field 2 I32, 3.3
        0x15, 0x33, 0x33, 0x53, 0x40,
        // temperature: field 3 I32, -1_250
        0x1D, 0x1E, 0xFB, 0xFF, 0xFF,
        // power: field 4 I32, 152.5
        0x25, 0x00, 0x80, 0x18, 0x43,
    ];

    /// The gauges [`Self::WIRE`] carries
    const DECODED: Self = Self {
        uptime: 86_400,
        voltage: 3.3,
        temperature: -1_250,
        power: 152.5,
    };
}

/// A sensor sample in `keylen` mode, three fixed-width values: each frame is the tag alone, then
/// the value
#[derive(Klv, Debug)]
#[klv(
    keylen(enc = proto::enc_tag),
    allow_unimplemented_decode,
    default(typ = f32, enc = *encb::le_f32),
    default(typ = i64, enc = *encb::le_i64),
    default(typ = u32, enc = *encb::le_u32),
)]
struct KeylenFixedWidths {
    /// Supply voltage, field 2, wire 5 I32
    #[klv(key = 0x15)]
    voltage: f32,

    /// Microseconds since the Unix epoch, field 4, wire 1 I64
    #[klv(key = 0x21)]
    timestamp: i64,

    /// Seconds since boot, field 5, wire 5 I32
    #[klv(key = 0x2D)]
    uptime: u32,
}

/// [`KeylenFixedWidths`] implementation
impl KeylenFixedWidths {
    /// The sample [`Self::WIRE`] carries
    const SAMPLE: Self = Self {
        voltage: 3.3,
        timestamp: 1_760_000_000_000_000,
        uptime: 86_400,
    };

    /// The bytes of [`Self::SAMPLE`]: each value little-endian after its tag
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // voltage: tag, 3.3
        0x15, 0x33, 0x33, 0x53, 0x40,
        // timestamp: tag, 1_760_000_000_000_000
        0x21, 0x00, 0x00, 0xCE, 0xEE, 0xB5, 0x40, 0x06, 0x00,
        // uptime: tag, 86_400
        0x2D, 0x80, 0x51, 0x01, 0x00,
    ];
}

/// A radio link's counters in `keylen` mode, eight varints: each frame is the tag alone, then the
/// varint
#[derive(Klv, Debug)]
#[klv(
    keylen(enc = proto::enc_tag),
    allow_unimplemented_decode,
    default(typ = i32, enc = *varint::enc_i32),
)]
struct KeylenVarints {
    /// Packets sent
    #[klv(key = 0x08)]
    packets: i32,

    /// Packets sent again
    #[klv(key = 0x10)]
    retries: i32,

    /// Received signal strength, dBm
    #[klv(key = 0x18)]
    rssi: i32,

    /// Radio channel
    #[klv(key = 0x20)]
    channel: i32,

    /// Frames received corrupt
    #[klv(key = 0x28)]
    errors: i32,

    /// Packets waiting to be sent
    #[klv(key = 0x30)]
    queued: i32,

    /// Packets given up on
    #[klv(key = 0x38)]
    dropped: i32,

    /// Round-trip time, microseconds
    #[klv(key = 0x40)]
    latency: i32,
}

/// [`KeylenVarints`] implementation
impl KeylenVarints {
    /// The counters [`Self::WIRE`] carries
    const SAMPLE: Self = Self {
        packets: 1_500,
        retries: 3,
        rssi: -67,
        channel: 11,
        errors: 0,
        queued: 42,
        dropped: 7,
        latency: 12_000,
    };

    /// The bytes of [`Self::SAMPLE`]: each varint after its tag
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // packets: tag, 1_500
        0x08, 0xDC, 0x0B,
        // retries: tag, 3
        0x10, 0x03,
        // rssi: tag, -67 sign-extended
        0x18, 0xBD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
        // channel: tag, 11
        0x20, 0x0B,
        // errors: tag, 0
        0x28, 0x00,
        // queued: tag, 42
        0x30, 0x2A,
        // dropped: tag, 7
        0x38, 0x07,
        // latency: tag, 12_000
        0x40, 0xE0, 0x5D,
    ];
}

/// A named device in `keylen` mode: a varint, then a string after its tag and its length
#[derive(Klv, Debug)]
#[klv(keylen(enc = proto::enc_tag), allow_unimplemented_decode)]
struct KeylenNamed {
    /// Device number, field 1, wire 0 VARINT
    #[klv(key = 0x08, enc = *varint::enc_i32)]
    id: i32,

    /// Device name, field 2, wire 2 LEN
    #[klv(key = 0x12, enc = &encs::from_string_utf8)]
    name: String,
}

/// [`KeylenNamed`] implementation
impl KeylenNamed {
    /// The `id` frame of every sample: tag, 5
    const ID_FRAME: &[u8] = &[0x08, 0x05];

    /// A 2-byte `name`, and its frame header: tag, length 2
    const SHORT: (usize, &[u8]) = (2, &[0x12, 0x02]);

    /// A 32-byte `name`, and its frame header: tag, length 32
    const MEDIUM: (usize, &[u8]) = (32, &[0x12, 0x20]);

    /// A 300-byte `name`, and its frame header: tag, length 300 in two varint bytes
    const LONG: (usize, &[u8]) = (300, &[0x12, 0xAC, 0x02]);

    /// The sample with a `name` of `len` bytes
    ///
    /// # Arguments
    ///
    /// * `len` - the byte length of `name`
    ///
    /// # Returns
    ///
    /// `id = 5` and `len` copies of `n`
    fn sample(len: usize) -> Self {
        Self {
            id: 5,
            name: "n".repeat(len),
        }
    }

    /// The bytes of [`Self::sample`]
    ///
    /// # Arguments
    ///
    /// * `(len, name_header)` - the byte length of `name`, and its frame header
    ///
    /// # Returns
    ///
    /// The `id` frame, then the `name` header and bytes
    fn wire((len, name_header): (usize, &[u8])) -> Vec<u8> {
        [Self::ID_FRAME, name_header, "n".repeat(len).as_bytes()].concat()
    }
}

/// A device report in `keylen` mode: a varint, then a nested [`KeylenNamed`] after its tag and its
/// length
#[derive(Klv, Debug)]
#[klv(keylen(enc = proto::enc_tag), allow_unimplemented_decode)]
struct KeylenNested {
    /// Report number, field 1, wire 0 VARINT
    #[klv(key = 0x08, enc = *varint::enc_i32)]
    report: i32,

    /// The reporting device, field 2, wire 2 LEN
    #[klv(key = 0x12, enc = KeylenNamed::encode_value)]
    device: KeylenNamed,
}

/// [`KeylenNested`] implementation
impl KeylenNested {
    /// The `report` frame, then the `device` frame header: tag, length 36
    const HEAD: &[u8] = &[0x08, 0x01, 0x12, 0x24];

    /// Report 1 of the [`KeylenNamed::MEDIUM`] device
    ///
    /// # Returns
    ///
    /// The report [`Self::wire`] carries
    fn sample() -> Self {
        Self {
            report: 1,
            device: KeylenNamed::sample(KeylenNamed::MEDIUM.0),
        }
    }

    /// The bytes of [`Self::sample`]
    ///
    /// # Returns
    ///
    /// [`Self::HEAD`], then the device's 36 bytes
    fn wire() -> Vec<u8> {
        [Self::HEAD, &KeylenNamed::wire(KeylenNamed::MEDIUM)].concat()
    }
}

/// Checks that `wire` decodes to `expected` with every byte read, then times the one-shot
/// [`DecodeValue::decode_value`] over it
///
/// The check runs once, outside the timed loop, so a wrong wire stops the run instead of timing a
/// decode that fails or stops short. Each timed decode reads a black-boxed copy of the slice, so
/// the optimizer cannot fold the known bytes into the decoder. The decoded value goes back to
/// criterion, which black-boxes it and drops it inside the timed loop, so a kept frame's
/// allocation and its free are both counted
///
/// # Arguments
///
/// * `group` - the criterion group to register under
/// * `id` - the benchmark id
/// * `wire` - the encoded value
/// * `expected` - the value `wire` decodes to
///
/// # Panics
///
/// When `wire` does not decode to `expected`, or leaves bytes unread
fn bench_decode<T>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    id: BenchmarkId,
    wire: &[u8],
    expected: &T,
) where
    T: for<'a> DecodeValue<&'a [u8]> + PartialEq + Debug,
{
    // --------------------------------------------------
    // the wire decodes to the expected value, every byte read
    // --------------------------------------------------
    let mut cursor = wire;
    assert_eq!(T::decode_value(&mut cursor).as_ref(), Ok(expected));
    assert!(
        cursor.is_empty(),
        "`{}` left {} bytes unread",
        type_name::<T>(),
        cursor.len(),
    );
    // --------------------------------------------------
    // time the decode
    // --------------------------------------------------
    group.bench_function(id, |b| {
        b.iter(|| T::decode_value(&mut black_box(wire)));
    });
}

/// Checks that `value` encodes to `wire`, then times [`EncodeValue::encode_value`] of it into a
/// buffer cleared before each encode
///
/// The check runs once, outside the timed loop, so a wrong wire stops the run instead of timing an
/// encode that writes other bytes. The buffer is reused, as a caller encoding record after record
/// reuses its own, so a timed encode allocates only what the generated code does. Each timed
/// encode reads a black-boxed reference to the value, so the optimizer cannot fold the known
/// fields into the encoder, and the encoded length goes back to criterion
///
/// # Arguments
///
/// * `group` - the criterion group to register under
/// * `id` - the benchmark id
/// * `value` - the value to encode
/// * `wire` - the bytes `value` encodes to
///
/// # Panics
///
/// When `value` does not encode to `wire`
fn bench_encode<T>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    id: BenchmarkId,
    value: &T,
    wire: &[u8],
) where
    T: EncodeValue + Debug,
{
    // --------------------------------------------------
    // the value encodes to the expected wire
    // --------------------------------------------------
    let mut out = Vec::new();
    value.encode_value(&mut out);
    assert_eq!(
        out,
        wire,
        "`{}` encodes {value:?} to another wire",
        type_name::<T>()
    );
    // --------------------------------------------------
    // time the encode
    // --------------------------------------------------
    group.bench_function(id, |b| {
        b.iter(|| {
            out.clear();
            black_box(value).encode_value(&mut out);
            out.len()
        });
    });
}

/// Times [`PrefixedVarints`] and [`ImplicitVarints`] decoding their wires
///
/// No throughput is set: the two wires carry the same value in a different number of bytes, so
/// the time per decode is the figure to compare
///
/// # Arguments
///
/// * `c` - the criterion driver to register the benchmark group under
fn varint_decode_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("varint_decode_value");
    group.sample_size(SAMPLE_SIZE);
    bench_decode(
        &mut group,
        BenchmarkId::from_parameter("prefixed"),
        PrefixedVarints::WIRE,
        &PrefixedVarints::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::from_parameter("implicit"),
        ImplicitVarints::WIRE,
        &ImplicitVarints::DECODED,
    );
    group.finish();
}

/// Times [`FixedWidths`], [`FixedWidthsDroppingUnknown`] and [`FixedWidthsKeepingUnknown`]
/// decoding [`FIXED_WIDTHS_WIRE`], alone and with [`UNKNOWN_FRAMES`] after it
///
/// No throughput is set, as the figure to compare is the time per decode of the same known fields
///
/// # Arguments
///
/// * `c` - the criterion driver to register the benchmark group under
fn unknown_decode_value(c: &mut Criterion) {
    let two_unknown = [FIXED_WIDTHS_WIRE, UNKNOWN_FRAMES].concat();
    let mut group = c.benchmark_group("unknown_decode_value");
    group.sample_size(SAMPLE_SIZE);
    // --------------------------------------------------
    // no unknown frame on the wire
    // --------------------------------------------------
    bench_decode(
        &mut group,
        BenchmarkId::new("no_unknown", KNOWN_ONLY),
        FIXED_WIDTHS_WIRE,
        &FixedWidths::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("dropping", KNOWN_ONLY),
        FIXED_WIDTHS_WIRE,
        &FixedWidthsDroppingUnknown::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("keeping", KNOWN_ONLY),
        FIXED_WIDTHS_WIRE,
        &FixedWidthsKeepingUnknown::DECODED,
    );
    // --------------------------------------------------
    // two unknown frames after the known ones
    // --------------------------------------------------
    bench_decode(
        &mut group,
        BenchmarkId::new("no_unknown", TWO_UNKNOWN),
        &two_unknown,
        &FixedWidths::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("dropping", TWO_UNKNOWN),
        &two_unknown,
        &FixedWidthsDroppingUnknown::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("keeping", TWO_UNKNOWN),
        &two_unknown,
        &FixedWidthsKeepingUnknown::with_unknown_frames(),
    );
    group.finish();
}

/// Times [`Contact`] and [`StrictContact`] decoding [`CONTACT_WIRE`], and [`Shape`] and
/// [`StrictShape`] decoding [`SHAPE_WIRE`]
///
/// No throughput is set, as the figure to compare is the time per decode of the same wire
///
/// # Arguments
///
/// * `c` - the criterion driver to register the benchmark group under
fn strict_decode_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("strict_decode_value");
    group.sample_size(SAMPLE_SIZE);
    // --------------------------------------------------
    // key + len
    // --------------------------------------------------
    bench_decode(
        &mut group,
        BenchmarkId::new("lenient", KEY_LEN),
        CONTACT_WIRE,
        &Contact::decoded(),
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("strict", KEY_LEN),
        CONTACT_WIRE,
        &StrictContact::decoded(),
    );
    // --------------------------------------------------
    // keylen
    // --------------------------------------------------
    bench_decode(
        &mut group,
        BenchmarkId::new("lenient", KEYLEN),
        SHAPE_WIRE,
        &Shape::decoded(),
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("strict", KEYLEN),
        SHAPE_WIRE,
        &StrictShape::decoded(),
    );
    group.finish();
}

/// Times [`BreakOnShape`] decoding [`SHAPE_WIRE`] and [`BreakOnGauges`] decoding its wire, every
/// item of each let proceed by the `break_on` fn
///
/// No throughput is set, as the figure to compare is the time per decode of the same wire on
/// another tree
///
/// # Arguments
///
/// * `c` - the criterion driver to register the benchmark group under
fn break_on_decode_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("break_on_decode_value");
    group.sample_size(SAMPLE_SIZE);
    bench_decode(
        &mut group,
        BenchmarkId::from_parameter("shape"),
        SHAPE_WIRE,
        &BreakOnShape::decoded(),
    );
    bench_decode(
        &mut group,
        BenchmarkId::from_parameter("gauges"),
        BreakOnGauges::WIRE,
        &BreakOnGauges::DECODED,
    );
    group.finish();
}

/// Times [`KeylenFixedWidths`], [`KeylenVarints`], [`KeylenNamed`] with a `name` of each benched
/// length, and [`KeylenNested`] encoding their samples
///
/// No throughput is set, as the figure to compare is the time per encode of the same message on
/// another tree
///
/// # Arguments
///
/// * `c` - the criterion driver to register the benchmark group under
fn keylen_encode_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("keylen_encode_value");
    group.sample_size(SAMPLE_SIZE);
    // --------------------------------------------------
    // fixed-width values, then varints
    // --------------------------------------------------
    bench_encode(
        &mut group,
        BenchmarkId::from_parameter("fixed_widths"),
        &KeylenFixedWidths::SAMPLE,
        KeylenFixedWidths::WIRE,
    );
    bench_encode(
        &mut group,
        BenchmarkId::from_parameter("varints"),
        &KeylenVarints::SAMPLE,
        KeylenVarints::WIRE,
    );
    // --------------------------------------------------
    // a string of each benched length, then a nested message
    // --------------------------------------------------
    for name in [KeylenNamed::SHORT, KeylenNamed::MEDIUM, KeylenNamed::LONG] {
        bench_encode(
            &mut group,
            BenchmarkId::new("named", name.0),
            &KeylenNamed::sample(name.0),
            &KeylenNamed::wire(name),
        );
    }
    bench_encode(
        &mut group,
        BenchmarkId::from_parameter("nested"),
        &KeylenNested::sample(),
        &KeylenNested::wire(),
    );
    group.finish();
}

criterion_group!(
    benches,
    varint_decode_value,
    unknown_decode_value,
    strict_decode_value,
    break_on_decode_value,
    keylen_encode_value
);
criterion_main!(benches);
