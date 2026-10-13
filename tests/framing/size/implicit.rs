//! `size(implicit)` on a field of a key + len struct: no length prefix is written or read
//!
//! An implicit value follows its key directly. One that delimits itself is decoded straight from
//! the input in one read; one of `exact = N` is the `N` bytes after its key, and a `lengthed`
//! decoder is passed `N`. Encode writes the key, then the value, which debug builds check is `N`
//! bytes under `exact = N`. A field with a length prefix beside it keeps its length, and a frame
//! of an unknown key is skipped by its length. An implicit value or a key cut at the end of the
//! input is truncation: streaming waits for the rest, one-shot finalizes without it. A
//! `break_on`, a key literal or a fn, is checked on every frame right after its key, an implicit
//! field's too
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::scans::Scans;
use crate::common::varint;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::token::take;

/// The plan's worked example, one proto3 message in key + len mode: `int32 id = 1` (a varint),
/// `float ratio = 2` (four bytes), `string name = 3` (length-delimited) and `fixed64
/// created_at = 4` (eight bytes), each scalar's framing from its type's `default(..)`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8], // default, shown for clarity
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32, size(implicit)),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32, size(implicit, exact = 4)),
    default(typ = u64, dec = decb::le_u64, enc = *encb::le_u64, size(implicit, exact = 8)),
)]
struct Reading {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire type 5
    #[klv(key = 0x15)]
    ratio: f32,

    /// field 3, wire type 2
    #[klv(
        key = 0x1A,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    name: String,

    /// field 4, wire type 1; optional, so `None` writes no frame
    #[klv(key = 0x21)]
    created_at: Option<u64>,
}

/// [`Reading`] implementation
impl Reading {
    /// The plan's bytes for [`Reading::decoded`]
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // id: tag, varint 5, no length
        0x08, 0x05,
        // ratio: tag, 1.5f32 little-endian, no length
        0x15, 0x00, 0x00, 0xC0, 0x3F,
        // name: tag, length 2, "hi"
        0x1A, 0x02, b'h', b'i',
    ];

    /// The plan's value, `Reading { id: 5, ratio: 1.5, name: "hi", created_at: <none> }`
    fn decoded() -> Self {
        Self {
            id: 5,
            ratio: 1.5,
            name: "hi".to_owned(),
            created_at: None,
        }
    }
}

/// A beacon's uptime in seconds, a varint on the wire: it delimits itself
#[derive(Debug, Clone, Copy, PartialEq)]
struct Uptime(u64);

/// [`Uptime`] implementation of [`DecodeValue`]: one varint
impl DecodeValue<&[u8]> for Uptime {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        varint::dec_u64.map(Self).parse_next(input)
    }
}

/// [`Uptime`] implementation of [`EncodeValue`]: one varint
impl EncodeValue for Uptime {
    fn encode_value(&self, out: &mut Vec<u8>) {
        varint::enc_u64(self.0, out);
    }
}

/// A grid position, two big-endian `i16`s: four bytes on the wire
#[derive(Debug, Clone, Copy, PartialEq)]
struct Position {
    /// Grid row
    row: i16,

    /// Grid column
    col: i16,
}

/// [`Position`] implementation of [`DecodeValue`]: the row, then the column
impl DecodeValue<&[u8]> for Position {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        (decb::be_i16, decb::be_i16)
            .map(|(row, col)| Self { row, col })
            .parse_next(input)
    }
}

/// [`Position`] implementation of [`EncodeValue`]: the row, then the column
impl EncodeValue for Position {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_i16(self.row, out);
        encb::be_i16(self.col, out);
    }
}

/// [`Position`] implementation
impl Position {
    /// Encodes the row alone: two of the four bytes a position is wide
    ///
    /// # Arguments
    ///
    /// * `out` - the buffer the row is appended to
    fn encode_row(&self, out: &mut Vec<u8>) {
        encb::be_i16(self.row, out);
    }
}

/// A station callsign, as many ASCII characters as its decoder is handed
#[derive(Debug, Clone, PartialEq)]
struct Callsign(String);

/// [`Callsign`] implementation
impl Callsign {
    /// Decodes a callsign of `width` characters
    ///
    /// # Arguments
    ///
    /// * `width` - the characters to read, the length the derive hands a `lengthed` decoder
    ///
    /// # Returns
    ///
    /// A decoder reading `width` ASCII characters, or failing on fewer or on a non-ASCII byte
    fn decode(width: usize) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Self> {
        move |input| {
            take(width)
                .verify(<[u8]>::is_ascii)
                .map(|chars: &[u8]| Self(String::from_utf8_lossy(chars).into_owned()))
                .parse_next(input)
        }
    }
}

/// [`Callsign`] implementation of [`EncodeValue`]: the characters
impl EncodeValue for Callsign {
    fn encode_value(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.0.as_bytes());
    }
}

/// A beacon whose every field is implicit, one per framing of `size(implicit)`: a value that
/// delimits itself, a value of `exact = N`, and a `lengthed` value of `exact = N`
#[derive(Klv, Debug, Clone, Default, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
)]
struct Beacon {
    /// a varint, read until it ends
    #[klv(
        key = 0x01,
        dec = Uptime::decode_value,
        enc = Uptime::encode_value,
        size(implicit),
    )]
    uptime: Option<Uptime>,

    /// four bytes
    #[klv(
        key = 0x02,
        dec = Position::decode_value,
        enc = Position::encode_value,
        size(implicit, exact = 4),
    )]
    position: Option<Position>,

    /// six characters, their count passed to the decoder
    #[klv(
        key = 0x03,
        dec = Callsign::decode,
        enc = Callsign::encode_value,
        size(implicit, exact = 6, lengthed),
    )]
    callsign: Option<Callsign>,
}

/// [`Beacon`] implementation
impl Beacon {
    /// uptime = 300 s: key, varint `AC 02`
    const UPTIME: &[u8] = &[0x01, 0xAC, 0x02];

    /// position = (-2, 515): key, `FF FE`, `02 03`
    const POSITION: &[u8] = &[0x02, 0xFF, 0xFE, 0x02, 0x03];

    /// callsign = "KD9XYZ": key, six characters
    const CALLSIGN: &[u8] = &[0x03, b'K', b'D', b'9', b'X', b'Y', b'Z'];

    /// An unknown key's frame: key, length 2, two bytes
    const UNKNOWN: &[u8] = &[0x7F, 0x02, 0xAA, 0xBB];

    /// The beacon [`Beacon::UPTIME`] holds
    fn uptime() -> Self {
        Self {
            uptime: Some(Uptime(300)),
            ..Self::default()
        }
    }

    /// The beacon [`Beacon::POSITION`] holds
    fn position() -> Self {
        Self {
            position: Some(Position { row: -2, col: 515 }),
            ..Self::default()
        }
    }

    /// The beacon [`Beacon::CALLSIGN`] holds
    fn callsign() -> Self {
        Self {
            callsign: Some(Callsign("KD9XYZ".to_owned())),
            ..Self::default()
        }
    }
}

/// A beacon log ended by key `0x03`, which is also its implicit callsign's key
#[derive(Klv, Debug, Default, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    break_on = 0x03,
)]
struct BeaconLog {
    /// a varint, read until it ends
    #[klv(
        key = 0x01,
        dec = Uptime::decode_value,
        enc = Uptime::encode_value,
        size(implicit),
    )]
    uptime: Option<Uptime>,

    /// six characters, never reached: its key ends the log first
    #[klv(
        key = 0x03,
        dec = Callsign::decode,
        enc = Callsign::encode_value,
        size(implicit, exact = 6, lengthed),
    )]
    callsign: Option<Callsign>,
}

/// A beacon log signed off by a callsign, which its `break_on` fn reads after the callsign's key
#[derive(Klv, Debug, Default, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
    break_on = SignedLog::end_after_callsign,
)]
struct SignedLog {
    /// a varint, read until it ends
    #[klv(key = 0x01, dec = Uptime::decode_value, size(implicit))]
    uptime: Option<Uptime>,

    /// four bytes
    #[klv(key = 0x02, dec = Position::decode_value, size(implicit, exact = 4))]
    position: Option<Position>,
}

/// [`SignedLog`] implementation
impl SignedLog {
    /// Ends the log once the six characters after a callsign's key are read
    ///
    /// # Arguments
    ///
    /// * `key` - the frame's key
    /// * `input` - the input right after the key
    ///
    /// # Returns
    ///
    /// `Done` past the callsign, else `Proceed`, a callsign cut short included, which then
    /// decodes as a frame cut short
    fn end_after_callsign(key: &u32, input: &mut &[u8]) -> BreakType {
        if *key != 0x03 {
            return BreakType::Proceed;
        }
        match Callsign::decode(6).parse_next(input) {
            Ok(_) => BreakType::Done,
            Err(_) => BreakType::Proceed,
        }
    }
}

/// A tank gauge whose level is read by the crate's big-endian `u16` decoder, with no length
/// before it
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
)]
struct Gauge {
    /// a varint, read until it ends
    #[klv(key = 0x08, dec = varint::dec_i32, size(implicit))]
    channel: i32,

    /// two bytes
    #[klv(key = 0x20, dec = decb::be_u16, size(implicit))]
    level: u16,
}

/// A radio's settings, implicit varints, its channel keyed by field 16, the first field number
/// whose tag takes two varint bytes
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32, size(implicit)),
)]
struct Radio {
    /// field 1
    #[klv(key = 0x08)]
    id: i32,

    /// field 16, tag `80 01`
    #[klv(key = 0x80)]
    channel: i32,

    /// field 3
    #[klv(key = 0x18)]
    power: i32,
}

/// A link's packet counters, implicit varints whose every read [`Scans`] counts
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
    default(typ = u64, dec = Scans::dec_u64, size(implicit)),
)]
struct Counters {
    /// packets sent
    #[klv(key = 0x01)]
    sent: u64,

    /// packets lost
    #[klv(key = 0x02)]
    lost: u64,
}

/// A beacon whose position encoder writes two of the four bytes its `exact = 4` declares
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
)]
struct RowBeacon {
    /// four bytes declared, two written
    #[klv(
        key = 0x02,
        dec = Position::decode_value,
        enc = Position::encode_row,
        size(implicit, exact = 4),
    )]
    position: Position,
}

/// The worked example encodes to the plan's exact bytes, and decodes back from them
#[test]
fn the_worked_example_round_trips_to_the_plans_bytes() {
    assert_eq!(Reading::decoded().encoded(), Reading::WIRE);
    Reading::decoded().assert_decodes_from(Reading::WIRE);
}

/// D2 row 7: a `lengthed` decoder of an `exact = N` value is passed `N`
#[test]
fn an_implicit_lengthed_decoder_is_passed_its_width() {
    assert_eq!(Beacon::callsign().encoded(), Beacon::CALLSIGN);
    Beacon::callsign().assert_decodes_from(Beacon::CALLSIGN);
}

/// An unknown key's frame has a length, and is skipped by it
#[test]
fn an_unknown_key_is_skipped_by_its_length() {
    let wire = [Beacon::UPTIME, Beacon::UNKNOWN, Beacon::CALLSIGN].concat();
    let expected = Beacon {
        callsign: Beacon::callsign().callsign,
        ..Beacon::uptime()
    };
    expected.assert_decodes_from(&wire);
}

/// A key-literal `break_on` equal to an implicit field's key stops the decode right after that
/// key, in both drivers and at every chunk split, with what came before it
#[test]
fn a_break_on_literal_stops_at_an_implicit_fields_key() {
    let wire = [Beacon::UPTIME, Beacon::CALLSIGN].concat();
    let decoded = BeaconLog {
        uptime: Beacon::uptime().uptime,
        ..BeaconLog::default()
    };
    decoded.assert_stops_at(&wire, Beacon::UPTIME.len() + 1);
}

/// A `break_on` fn beside implicit fields runs right after each key too, and the decode stops
/// where it leaves the input: past the callsign it reads, before the position after it
#[test]
fn a_break_on_fn_beside_an_implicit_field_stops_where_it_leaves_the_input() {
    let wire = [Beacon::UPTIME, Beacon::CALLSIGN, Beacon::POSITION].concat();
    let decoded = SignedLog {
        uptime: Beacon::uptime().uptime,
        ..SignedLog::default()
    };
    decoded.assert_stops_at(&wire, Beacon::UPTIME.len() + Beacon::CALLSIGN.len());
}

/// An implicit value cut at the end of the input is truncation, whether it delimits itself or
/// is `exact = N`: streaming pauses at the cut frame's start and completes once the rest arrives,
/// and one-shot finalizes the frames before it, the cursor left at the cut frame's start
#[test]
fn an_implicit_value_cut_at_the_end_of_the_input_is_truncation() {
    let whole = Beacon {
        position: Beacon::position().position,
        ..Beacon::uptime()
    };
    // --------------------------------------------------
    // (the frame before the cut one, the cut one, what one-shot finalizes)
    // --------------------------------------------------
    let cases = [
        (Beacon::POSITION, Beacon::UPTIME, Beacon::position()),
        (Beacon::UPTIME, Beacon::POSITION, Beacon::uptime()),
    ];
    for (head, cut, before) in cases {
        let wire = [head, cut].concat();
        let held = cut.len() - 1;
        // --------------------------------------------------
        // streaming: pauses with the cut frame unconsumed, then the rest completes it, at every
        // chunk split
        // --------------------------------------------------
        whole.assert_decodes_from(&wire);
        // --------------------------------------------------
        // one-shot: the frame before the cut one, the cursor at the cut frame's start
        // --------------------------------------------------
        let mut cursor = &wire[..wire.len() - 1];
        assert_eq!(Beacon::decode_value(&mut cursor), Ok(before));
        assert_eq!(cursor.len(), held, "one-shot stopped at the cut frame");
    }
}

/// A value of the crate's fixed-width decoder cut at the end of the input is truncation too:
/// streamed as `08 03 20 01`, then `02`, the decode completes, as at every other split
#[test]
fn a_fixed_width_value_cut_at_the_end_of_the_input_is_truncation() {
    #[rustfmt::skip]
    let wire = [
        // channel = 3: key, varint `03`
        0x08, 0x03,
        // level = 0x0102: key, `01 02`
        0x20, 0x01, 0x02,
    ];
    Gauge {
        channel: 3,
        level: 0x0102,
    }
    .assert_decodes_from(&wire);
}

/// A key cut at the end of the input is truncation too: streamed as `08 05 80`, then `01 07 18
/// 09`, the decode waits inside the two-byte tag and completes, as at every other split
#[test]
fn a_key_cut_at_the_end_of_the_input_is_truncation() {
    #[rustfmt::skip]
    let wire = [
        // id = 5: key, varint `05`
        0x08, 0x05,
        // channel = 7: key `80 01`, varint `07`
        0x80, 0x01, 0x07,
        // power = 9: key, varint `09`
        0x18, 0x09,
    ];
    Radio {
        id: 5,
        channel: 7,
        power: 9,
    }
    .assert_decodes_from(&wire);
}

/// An implicit value that delimits itself is read once per frame, by one-shot and by streaming
/// over the whole input
#[test]
fn an_implicit_value_is_read_once_by_either_driver() {
    #[rustfmt::skip]
    let wire = [
        // sent = 300: key, varint `AC 02`
        0x01, 0xAC, 0x02,
        // lost = 5: key, varint `05`
        0x02, 0x05,
    ];
    let expected = Counters { sent: 300, lost: 5 };
    let (decoded, scans) = Scans::count(|| Counters::decode_value(&mut &wire[..]));
    assert_eq!(decoded.as_ref(), Ok(&expected));
    assert_eq!(scans, 2, "one-shot: one read per frame");
    let ((), scans) = Scans::count(|| expected.assert_streams_to(&mut &wire[..]));
    assert_eq!(scans, 2, "streaming: one read per frame");
}

/// An `exact = N` value whose encoder writes another width fails the debug check
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "wrote a different byte count than its fixed width")]
fn an_exact_value_of_another_width_fails_the_debug_check() {
    RowBeacon {
        position: Position { row: -2, col: 515 },
    }
    .encoded();
}
