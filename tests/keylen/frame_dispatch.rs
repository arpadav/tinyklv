//! Unknown-key and duplicate-key tests for the fused `#[klv(keylen(..))]` codec, through both
//! drivers
//!
//! An unknown `Sliced` frame is skipped by its length; an unknown `Inline` frame ends the
//! loop at its header; `deny_unknown_keys` rejects either. A repeated key keeps its last
//! value, or the one before it when a later `Sliced` value fails to decode
//!
//! Under `unknown(dec, enc)` the header frames an unknown frame as it frames any other, and the
//! unknown decoder reads the body it left: the bounded slice of a `Sliced` frame, the main stream
//! of an `Inline` one, so the loop reads on past either. The `#[klv(unknown)]` field keeps each
//! frame's key and value in the order they arrived; encode writes them after the known fields,
//! the header through the keylen encoder and the body through the unknown encoder. Without the
//! field an `Inline` frame is decoded and dropped, a `Sliced` one dropped with its bytes; a struct
//! with no other KLV field keeps every frame. A value its decoder rejects is malformed, an
//! `Inline` one before the end of the input and a `Sliced` one the field keeps: both drivers
//! reject it with an error naming the `#[klv(unknown)]` field, or the struct's frame when no field
//! keeps it
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::frames;
use crate::common::unknown::{self, Unknown};
use crate::common::varint;
use crate::support::example::Reading;
use crate::support::header_budget::HeaderBudget;
use crate::support::{header, value};
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

/// [`Reading`] under `deny_unknown_keys`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    deny_unknown_keys,
    default(typ = i32, dec = varint::dec_i32),
    default(typ = String, dec = value::dec_string),
)]
struct StrictReading {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    count: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x12)]
    label: Option<String>,
}

/// Message with no required field, so a decode that reads no frame still finalizes
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = Sparse::header),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32),
)]
struct Sparse {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    count: Option<i32>,
}

/// [`Sparse`] implementation
impl Sparse {
    /// [`header::dec_tag`] under the [`HeaderBudget`], so a decode loop that never advances
    /// panics instead of hanging the suite
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at a frame header; advanced past it on success
    ///
    /// # Returns
    ///
    /// The result of [`header::dec_tag`]
    ///
    /// # Panics
    ///
    /// On the read past the [`HeaderBudget`]
    fn header(input: &mut &[u8]) -> tinyklv::Result<(u32, Framing)> {
        HeaderBudget::spend();
        header::dec_tag(input)
    }
}

/// A proto3 message in keylen mode, keeping its unknown frames: `int32 id = 1` (a varint),
/// `float ratio = 2` (four bytes) and `string name = 3` (length-delimited)
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8], // default, shown for clarity
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    unknown(dec = unknown::unknown_body_dec, enc = unknown::unknown_body_enc),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32),
    default(typ = String, dec = value::dec_string, enc = &encs::from_string_utf8),
)]
struct Sensor {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire 5 I32 - `Sliced(4)`
    #[klv(key = 0x15)]
    ratio: f32,

    /// field 3, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x1A)]
    name: String,

    /// every other frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// [`Sensor`] without the `#[klv(unknown)]` field
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    unknown(dec = unknown::unknown_body_dec, enc = unknown::unknown_body_enc),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32),
    default(typ = String, dec = value::dec_string, enc = &encs::from_string_utf8),
)]
struct LossySensor {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire 5 I32 - `Sliced(4)`
    #[klv(key = 0x15)]
    ratio: f32,

    /// field 3, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x1A)]
    name: String,
}

/// A message with no keyed field: every frame is an unknown one, kept in `extra`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    unknown(dec = unknown::unknown_body_dec, enc = unknown::unknown_body_enc),
)]
struct Empty {
    /// every frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// A message keeping the value of every frame it does not declare as UTF-8 text
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag),
    unknown(dec = Annotated::note),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32),
)]
struct Annotated {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// every other frame's value, keyed by its tag
    #[klv(unknown)]
    notes: Vec<(u32, String)>,
}

/// [`Annotated`] implementation
impl Annotated {
    /// Decodes the value of a frame no field declares as UTF-8 text
    ///
    /// # Arguments
    ///
    /// * `_tag` - the frame's key; every such value is text
    ///
    /// # Returns
    ///
    /// [`value::dec_string`], which reads the whole body and rejects bytes that are not UTF-8
    fn note(_tag: u32) -> impl Fn(&mut &[u8]) -> tinyklv::Result<String> {
        value::dec_string
    }
}

/// `deny_unknown_keys` rejects an unknown key before a `Sliced` value is taken, with a label
/// naming the struct
#[test]
fn deny_unknown_keys_rejects_before_the_value_is_taken() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // unknown field 9, wire 2 LEN, Sliced(5) - 1 of its 5 bytes
        0x4A, 0x05, 0x00,
    ];
    StrictReading::assert_rejects_naming(&wire, &["key (expected", "StrictReading"]);
}

/// Without `deny_unknown_keys` an unknown `Inline` frame has nothing to skip it by:
/// the loop stops at its header and both drivers finalize what landed before it
#[test]
fn inline_unknown_ends_loop() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // unknown field 9, wire 0 VARINT, Inline
        0x48, 0x01,
        // label = "hi" - past the unknown frame, never reached
        0x12, 0x02, 0x68, 0x69,
    ];
    let mut cursor = &wire[..];
    match Reading::decode_partial(&mut cursor) {
        Ok(Packet::Ready(decoded)) => assert_eq!(decoded, Reading::count_only()),
        other => panic!("streaming: expected Ready, got {other:?}"),
    }
    assert_eq!(cursor, &wire[2..], "streaming stops at the unknown frame");
    let mut cursor = &wire[..];
    assert_eq!(
        Reading::decode_value(&mut cursor),
        Ok(Reading::count_only())
    );
    assert_eq!(cursor, &wire[2..], "one-shot stops at the unknown frame");
}

/// An unknown `Inline` frame first on the wire is a `Ready` that consumed nothing, so a
/// `Vec` of such messages must stop on it rather than collect empty elements forever
#[test]
fn inline_unknown_first_frame_is_ready_unmoved() {
    #[rustfmt::skip]
    let wire = [
        // unknown field 9, wire 0 VARINT, Inline
        0x48, 0x01,
        // count = 3 - past the unknown frame, never reached
        0x08, 0x03,
    ];
    let mut cursor = &wire[..];
    match Sparse::decode_partial(&mut cursor) {
        Ok(Packet::Ready(decoded)) => assert_eq!(decoded, Sparse { count: None }),
        other => panic!("single: expected Ready, got {other:?}"),
    }
    assert_eq!(cursor, &wire[..], "single decode must not move the cursor");
    let mut cursor = &wire[..];
    match Vec::<Sparse>::decode_partial(&mut cursor) {
        Ok(Packet::Ready(decoded)) => assert_eq!(decoded, []),
        other => panic!("vec: expected Ready, got {other:?}"),
    }
    assert_eq!(cursor, &wire[..], "vec decode must not move the cursor");
    let mut cursor = &wire[..];
    assert_eq!(Vec::<Sparse>::decode_value(&mut cursor), Ok(vec![]));
    assert_eq!(
        cursor,
        &wire[..],
        "one-shot vec decode must not move the cursor"
    );
}

/// Without `deny_unknown_keys` an unknown `Sliced` frame is skipped by its length and
/// the frames after it decode
#[test]
fn sliced_unknown_is_skipped() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // unknown field 9, wire 2 LEN, Sliced(1)
        0x4A, 0x01, 0x00,
        // label = "hi"
        0x12, 0x02, 0x68, 0x69,
    ];
    let expected = Reading {
        label: Some("hi".to_owned()),
        ..Reading::count_only()
    };
    assert_eq!(
        Reading::decode_value(&mut &wire[..]).as_ref(),
        Ok(&expected)
    );
    expected.assert_streams_to(&mut &wire[..]);
}

/// A key seen twice keeps its last value, for an `Inline` and a `Sliced` field alike
#[test]
fn duplicate_key_last_wins() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // label = "a"
        0x12, 0x01, 0x61,
        // count = 4
        0x08, 0x04,
        // label = "b"
        0x12, 0x01, 0x62,
    ];
    let expected = Reading {
        count: 4,
        label: Some("b".to_owned()),
        ..Reading::count_only()
    };
    assert_eq!(
        Reading::decode_value(&mut &wire[..]).as_ref(),
        Ok(&expected)
    );
    expected.assert_streams_to(&mut &wire[..]);
}

/// A key seen twice whose later `Sliced` value its decoder rejects keeps the earlier value, in
/// both drivers and at every chunk split
#[test]
fn a_repeated_key_whose_later_value_fails_keeps_the_earlier_one() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // label = "a"
        0x12, 0x01, 0x61,
        // label = `FF`, not UTF-8
        0x12, 0x01, 0xFF,
    ];
    let expected = Reading {
        label: Some("a".to_owned()),
        ..Reading::count_only()
    };
    expected.assert_decodes_from(&wire);
}

/// Unknown frames of every wire type after the known fields, the `Inline` one first, decode into
/// `extra` in wire order, in both drivers and at every chunk split, and encode back to the same
/// bytes
#[test]
fn unknown_frames_after_the_known_fields_round_trip_to_their_bytes() {
    let wire = [
        frames::ID,
        frames::RATIO,
        frames::NAME,
        // Inline
        frames::UNKNOWN_VARINT,
        // Sliced(8)
        frames::UNKNOWN_I64,
        // Sliced(3)
        frames::UNKNOWN_LEN,
        // Sliced(4)
        frames::UNKNOWN_I32,
    ]
    .concat();
    let decoded = Sensor {
        id: 5,
        ratio: 1.5,
        name: "hi".to_owned(),
        extra: vec![
            frames::varint_entry(),
            frames::i64_entry(),
            frames::len_entry(),
            frames::i32_entry(),
        ],
    };
    decoded.assert_decodes_from(&wire);
    assert_eq!(decoded.encoded(), wire);
}

/// Without the `#[klv(unknown)]` field an unknown `Inline` frame is decoded and dropped, so the
/// known fields after it decode
#[test]
fn a_dropped_inline_unknown_frame_leaves_the_next_field_to_decode() {
    let wire = [
        frames::UNKNOWN_VARINT,
        frames::ID,
        frames::RATIO,
        frames::NAME,
    ]
    .concat();
    let decoded = LossySensor {
        id: 5,
        ratio: 1.5,
        name: "hi".to_owned(),
    };
    decoded.assert_decodes_from(&wire);
}

/// A struct whose only KLV field is the `#[klv(unknown)]` one decodes every frame into it, an
/// `Inline` and a `Sliced` one alike, and encodes them back to the same bytes
#[test]
fn a_struct_with_only_the_unknown_field_round_trips_its_frames_to_their_bytes() {
    let wire = [
        // Inline
        frames::UNKNOWN_VARINT,
        // Sliced(3)
        frames::UNKNOWN_LEN,
    ]
    .concat();
    let decoded = Empty {
        extra: vec![frames::varint_entry(), frames::len_entry()],
    };
    decoded.assert_decodes_from(&wire);
    assert_eq!(decoded.encoded(), wire);
}

/// An unknown `Inline` value its decoder rejects before the end of the input is malformed when no
/// field keeps it too: both drivers reject it at every chunk split, with an error naming the
/// struct's frame, and leave the cursor where the decoder stopped, on the 10th byte of an
/// overlong varint
#[test]
fn a_malformed_inline_unknown_value_is_rejected_naming_the_frame() {
    let message = "`LossySensor` item (its `unknown` decoder failed before the end of the input)";
    #[rustfmt::skip]
    let overlong: &[u8] = &[
        // unknown field 4, wire 0 VARINT, Inline, 9 continuation bytes
        0x20,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];
    let head = [frames::ID, frames::RATIO, frames::NAME, overlong].concat();
    let wire = [
        &head[..],
        // a 10th byte carrying bits past the 64th
        &[0x02],
        frames::UNKNOWN_I32,
    ]
    .concat();
    LossySensor::assert_rejects_at(&wire, head.len(), message);
}

/// An unknown `Sliced` value the `#[klv(unknown)]` field keeps is malformed when its decoder
/// rejects it: both drivers reject it at every chunk split, with an error naming the field, and
/// leave the cursor past the frame
#[test]
fn a_kept_sliced_unknown_value_its_decoder_rejects_is_malformed() {
    let message = "`Annotated::notes` value (its `unknown` decoder rejected the frame's value)";
    #[rustfmt::skip]
    let note: &[u8] = &[
        // unknown field 6, wire 2 LEN, Sliced(2)
        0x32, 0x02,
        // bytes that are not UTF-8
        0xFF, 0x01,
    ];
    let wire = [note, frames::ID].concat();
    Annotated::assert_rejects_at(&wire, note.len(), message);
}
