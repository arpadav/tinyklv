//! `unknown(dec, enc)` on a key + len struct: the frames whose key no field declares
//!
//! The unknown decoder is handed the key and reads everything after it, the frame's length
//! included when it has one, so a VARINT frame is never misread as length-prefixed. The
//! `#[klv(unknown)]` field keeps each unknown frame's key and value in the order they arrived;
//! encode writes them after the known fields, the key and then the unknown encoder, which writes
//! the length itself. Without the field the frames are decoded and dropped; a struct with no other
//! KLV field keeps every frame. A value its decoder rejects before the end of the input is
//! malformed: both drivers reject it with an error naming the `#[klv(unknown)]` field. A frame
//! whose key and unknown value together consume nothing could only repeat, so it is rejected
//! naming the struct's frame. A `break_on` key is checked right after it is read, before the
//! unknown decoder could read its frame. A struct with a length-checked field decodes one-shot
//! over a copy of the cursor, handed to the unknown decoder and written back after it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::frames;
use crate::common::unknown::{self, Unknown};
use crate::common::varint;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::cell::Cell;
use winnow::error::{ContextError, ParserError};

/// A proto3 message in key + len mode, keeping its unknown frames: `int32 id = 1` (a varint),
/// `float ratio = 2` (four bytes) and `string name = 3` (length-delimited)
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8], // default, shown for clarity
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32, size(implicit)),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32, size(implicit, exact = 4)),
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

    /// every other frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// [`Reading`] implementation
impl Reading {
    /// unknown field 4, wire type 6: a tag protobuf frames no value after
    const WIRE_TYPE_6: &[u8] = &[0x26];

    /// `id = 5`, `ratio = 1.5`, `name = "hi"` and no unknown frame
    fn known() -> Self {
        Self {
            id: 5,
            ratio: 1.5,
            name: "hi".to_owned(),
            extra: Vec::new(),
        }
    }
}

/// [`Reading`] without the `#[klv(unknown)]` field
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32, size(implicit)),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32, size(implicit, exact = 4)),
)]
struct LossyReading {
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
}

/// A message with no keyed field: every frame is an unknown one, kept in `extra`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
)]
struct Empty {
    /// every frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// Self-describing message: every frame leads with a type byte, which is its key and which its
/// value decoder reads again; a type no field declares carries no value
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = Peeked::kind),
    len(dec = varint::dec_usize),
    unknown(dec = Peeked::bare),
    allow_unimplemented_encode,
)]
struct Peeked {
    /// type byte `0x01`, then one value byte
    #[klv(key = 0x01, dec = Peeked::small, size(implicit))]
    small: Option<u8>,
}

/// [`Peeked`] implementation
impl Peeked {
    /// small = 5 (type `0x01`), the first frame of every wire, so the first key a decode reads is
    /// never on the input the last key of the decode before it was read on ([`Peeked::kind`])
    const SMALL: &[u8] = &[0x01, 0x05];

    /// Decodes a frame's key: its type byte, left in the input for the value decoder
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at a frame; never advanced
    ///
    /// # Returns
    ///
    /// The type byte, or an error on empty input
    ///
    /// # Panics
    ///
    /// When called on the input of the call before it on this thread: a decode loop that does not
    /// advance reads the same key again, so it fails the test instead of hanging it
    fn kind(input: &mut &[u8]) -> tinyklv::Result<u8> {
        thread_local! {
            /// The address and length of the input of the last call on this thread
            static LAST: Cell<Option<(usize, usize)>> = const { Cell::new(None) };
        }
        let this = Some((input.as_ptr().addr(), input.len()));
        assert_ne!(LAST.replace(this), this, "the decode loop is not advancing");
        input
            .first()
            .copied()
            .ok_or_else(|| ContextError::from_input(input))
    }

    /// Decodes the `small` value: its type byte, then one byte
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the type byte; advanced past the value on
    ///   success, and to the end of input when the value byte is missing, so a cut value is
    ///   truncation
    ///
    /// # Returns
    ///
    /// The value byte, or an error when the input ends first
    fn small(input: &mut &[u8]) -> tinyklv::Result<u8> {
        // the type byte the key decoder left in the input
        decb::u8(input)?;
        decb::u8(input)
    }

    /// Decodes the value of a type no field declares: there is none
    ///
    /// # Arguments
    ///
    /// * `_kind` - the frame's type byte
    ///
    /// # Returns
    ///
    /// A decoder that consumes nothing and returns `Ok(())`
    fn bare(_kind: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<()> {
        |_| Ok(())
    }
}

/// A message with no `implicit` field, ended by key `0x7A` (field 15, wire type 2), which no field
/// declares
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
    break_on = 0x7A,
)]
struct Terminated {
    /// field 3, wire type 2
    #[klv(
        key = 0x1A,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    name: String,
}

/// A proto3 message in key + len mode whose length-delimited field is always four bytes, keeping
/// its unknown frames: `int32 id = 1` (a varint) and `bytes addr = 2` (an IPv4 address)
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8], // default, shown for clarity
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32, size(implicit)),
)]
struct Host {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire type 2, its length checked
    #[klv(key = 0x12, dec = decb::be_u32, enc = *encb::be_u32, size(exact = 4))]
    addr: Option<u32>,

    /// every other frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// [`Host`] implementation
impl Host {
    /// addr: tag, length 4, 10.0.0.1
    const ADDR: &[u8] = &[0x12, 0x04, 0x0A, 0x00, 0x00, 0x01];
}

/// Unknown frames of every wire type after the known fields decode into `extra` in wire order,
/// and encode back to the same bytes. Streaming, split at every offset, waits at a frame cut
/// short, inside an unknown frame's length-delimited body included, completes it once the rest
/// arrives, and keeps an unknown frame decoded before the cut once
#[test]
fn unknown_frames_after_the_known_fields_round_trip_to_their_bytes() {
    let wire = [
        frames::ID,
        frames::RATIO,
        frames::NAME,
        frames::UNKNOWN_VARINT,
        frames::UNKNOWN_I64,
        frames::UNKNOWN_LEN,
        frames::UNKNOWN_I32,
    ]
    .concat();
    let decoded = Reading {
        extra: vec![
            frames::varint_entry(),
            frames::i64_entry(),
            frames::len_entry(),
            frames::i32_entry(),
        ],
        ..Reading::known()
    };
    decoded.assert_decodes_from(&wire);
    assert_eq!(decoded.encoded(), wire);
}

/// An unknown frame between known fields round-trips by value, and encodes after them
#[test]
fn an_interleaved_unknown_frame_is_encoded_after_the_known_fields() {
    let interleaved = [frames::ID, frames::UNKNOWN_LEN, frames::RATIO, frames::NAME].concat();
    let reencoded = [frames::ID, frames::RATIO, frames::NAME, frames::UNKNOWN_LEN].concat();
    let decoded = Reading {
        extra: vec![frames::len_entry()],
        ..Reading::known()
    };
    decoded.assert_decodes_from(&interleaved);
    assert_eq!(decoded.encoded(), reencoded);
    decoded.assert_decodes_from(&reencoded);
}

/// Without the `#[klv(unknown)]` field an unknown frame is decoded and dropped: a VARINT frame
/// before a known field, read as a length, would misframe it
#[test]
fn a_dropped_varint_unknown_frame_leaves_the_next_field_to_decode() {
    let wire = [
        frames::UNKNOWN_VARINT,
        frames::ID,
        frames::RATIO,
        frames::NAME,
    ]
    .concat();
    let decoded = LossyReading {
        id: 5,
        ratio: 1.5,
        name: "hi".to_owned(),
    };
    decoded.assert_decodes_from(&wire);
}

/// A struct whose only KLV field is the `#[klv(unknown)]` one decodes every frame into it, and
/// encodes them back to the same bytes
#[test]
fn a_struct_with_only_the_unknown_field_round_trips_its_frames_to_their_bytes() {
    let wire = [frames::UNKNOWN_VARINT, frames::UNKNOWN_LEN].concat();
    let decoded = Empty {
        extra: vec![frames::varint_entry(), frames::len_entry()],
    };
    decoded.assert_decodes_from(&wire);
    assert_eq!(decoded.encoded(), wire);
}

/// An unknown value its decoder rejects before the end of the input is malformed: both drivers
/// reject it at every chunk split, with an error naming the `#[klv(unknown)]` field, and leave
/// the cursor where the decoder stopped
#[test]
fn a_malformed_unknown_value_before_the_end_of_the_input_is_rejected() {
    let message =
        "`Reading::extra` value (its `unknown` decoder failed before the end of the input)";
    let wire = [
        frames::ID,
        frames::RATIO,
        frames::NAME,
        Reading::WIRE_TYPE_6,
        frames::UNKNOWN_VARINT,
    ]
    .concat();
    Reading::assert_rejects_at(&wire, wire.len() - frames::UNKNOWN_VARINT.len(), message);
}

/// A frame whose key and unknown value together consume nothing could only repeat: both drivers
/// reject it at every chunk split, with an error naming the struct's frame, and leave the cursor
/// at its start
#[test]
fn an_unknown_frame_that_consumed_nothing_is_rejected() {
    let message = "`Peeked` frame (its header and value together consumed no input)";
    let wire = [
        Peeked::SMALL,
        // type 0x07, no field's: its key is peeked and its value is empty
        &[0x07],
    ]
    .concat();
    Peeked::assert_rejects_at(&wire, Peeked::SMALL.len(), message);
}

/// A `break_on` key no field declares ends the decode right after it is read, before the unknown
/// decoder could read the length its wire type carries: one-shot returns what came before it and
/// leaves the bytes after the key unread
#[test]
fn a_break_on_key_ends_the_decode_before_the_unknown_decoder_reads_its_frame() {
    // what the unknown decoder would read as the break frame's length
    let unread: &[u8] = &[0x02];
    let wire = [
        frames::NAME,
        // the break key
        &[0x7A],
        unread,
    ]
    .concat();
    let mut cursor = &wire[..];
    let decoded = Terminated {
        name: "hi".to_owned(),
    };
    assert_eq!(Terminated::decode_value(&mut cursor), Ok(decoded));
    assert_eq!(cursor, unread);
}

/// [`Host`]'s length-checked field makes its one-shot decode run over a copy of the cursor, which
/// reads unknown frames as the caller's cursor does: known frames around an unknown one decode
/// with it kept and the cursor at the end, and an unknown value cut short rewinds the cursor to
/// its frame's start, keeping what came before it
#[test]
fn a_one_shot_decode_over_a_cursor_copy_reads_unknown_frames() {
    let wire = [frames::ID, frames::UNKNOWN_LEN, Host::ADDR].concat();
    // whole: every frame decoded, every byte consumed
    let mut cursor = &wire[..];
    let decoded = Host {
        id: 5,
        addr: Some(0x0A00_0001),
        extra: vec![frames::len_entry()],
    };
    assert_eq!(Host::decode_value(&mut cursor), Ok(decoded));
    assert!(cursor.is_empty(), "{} bytes left", cursor.len());
    // cut: the unknown frame's last value byte missing
    let (cut, _) = wire.split_at(frames::ID.len() + frames::UNKNOWN_LEN.len() - 1);
    let mut cursor = cut;
    let decoded = Host {
        id: 5,
        addr: None,
        extra: Vec::new(),
    };
    assert_eq!(Host::decode_value(&mut cursor), Ok(decoded));
    assert_eq!(cursor, &cut[frames::ID.len()..]);
}
