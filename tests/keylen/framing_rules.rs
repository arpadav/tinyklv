//! Truncation-vs-malformed tests for the fused `#[klv(keylen(..))]` codec, through both drivers
//!
//! Drives `decode_partial` / `resume_partial` across chunk boundaries and `decode_value`
//! over cut and corrupt buffers. Any fused-header failure is truncation: streaming returns
//! `NeedMore`, one-shot stops at the frame and finalizes. An `Inline` value that fails at
//! end of input is truncation too; one that fails mid-buffer is malformed (an `Err`
//! labelled `<Struct>::<field>`)
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::drive::Drive;
use crate::example::{Example, Reading, Status};
use crate::pb;
use tinyklv::codecs::ber::BerLength;
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;
use tinyklv::traits::ResumePartial;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::error::{ContextError, ParserError};

/// Decodes a wire-0 VARINT like [`Status::decode_value`], but rejects an unknown
/// discriminant without rewinding, leaving the cursor past the varint
///
/// # Arguments
///
/// * `input` - the byte stream positioned at the value; advanced past the varint
///
/// # Returns
///
/// `Ok(Status)`, or an error on truncation or an unknown discriminant
fn decode_consuming(input: &mut &[u8]) -> tinyklv::Result<Status> {
    let value = pb::dec_varint_u64(input)?;
    Status::try_from(value).map_err(|_| ContextError::from_input(input))
}

/// Message whose `Inline` values use decoders the end-of-input rule reaches but misclassifies
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen),
    allow_unimplemented_encode,
    default(typ = i32, dec = pb::dec_varint_i32),
)]
struct Limits {
    #[klv(key = 0x08)]
    /// field 1, wire 0 VARINT - `Inline`
    count: i32,

    #[klv(key = 0x18, dec = decode_consuming)]
    /// field 3, wire 0 VARINT - `Inline`, rejects after consuming the varint
    status: Option<Status>,

    #[klv(key = 0x20, dec = decb::be_u16)]
    /// field 4, wire 0 - `Inline`, refuses fewer than 2 bytes without consuming
    port: Option<u16>,
}

/// Message whose keylen header is built from crate codecs: a `be_u16` key, which fails short
/// of the end of input, and a BER length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = Wide::header),
    allow_unimplemented_encode,
)]
struct Wide {
    #[klv(key = 0x0101, dec = decb::u8)]
    /// `Sliced(1)`
    flag: u8,

    #[klv(key = 0x0102, dec = pb::dec_string)]
    /// `Sliced(128)` - length in two-byte BER long form `0x82 0x00 0x80`: non-minimal, accepted
    text: String,
}

impl Wide {
    /// Byte length of `text`
    const TEXT_LEN: usize = 128;

    /// Decodes a frame header: a `be_u16` key, then a BER length bounding the value
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at a frame header; advanced past it on success
    ///
    /// # Returns
    ///
    /// `Ok((key, Framing::Sliced(len)))`, or the error of whichever codec ran out of bytes
    fn header(input: &mut &[u8]) -> tinyklv::Result<(u16, Framing)> {
        let key = decb::be_u16(input)?;
        let len = decber::ber_length(input)?;
        Ok((key, Framing::Sliced(len)))
    }

    /// The wire encoding of [`Wide::decoded`]
    #[rustfmt::skip]
    fn wire() -> Vec<u8> {
        let mut wire = vec![
            // flag = 7 (key 0x0101, length 1)
            0x01, 0x01, 0x01, 0x07,
            // text (key 0x0102, length 128 in two-byte BER long form: non-minimal, accepted)
            0x01, 0x02, 0x82, 0x00, 0x80,
        ];
        wire.extend(Self::text().bytes());
        wire
    }

    /// The decoded [`Wide`] matching [`Wide::wire`]
    fn decoded() -> Self {
        Self {
            flag: 7,
            text: Self::text(),
        }
    }

    /// The decoded `text`: [`Wide::TEXT_LEN`] bytes
    fn text() -> String {
        "t".repeat(Self::TEXT_LEN)
    }
}

/// Every two-chunk split of the canonical wire decodes to the same struct as one-shot,
/// including inside a two-byte tag, between a tag and its length, and inside a two-byte
/// length prefix
#[test]
fn split_at_every_offset() {
    Example::decoded().assert_every_split_completes(&Example::wire());
}

/// A header built from crate codecs still pauses at every split and completes, whether the
/// codec fails short of the end of input (`be_u16` on one byte) or consumes to it (a BER
/// long-form length cut inside its length bytes)
#[test]
fn split_inside_crate_codec_header() {
    Wide::decoded().assert_every_split_completes(&Wide::wire());
}

/// An `Inline` varint cut between its two bytes pauses with the frame unconsumed,
/// then completes once the tail arrives
#[test]
fn inline_varint_cut_mid_value_needs_more() {
    let wire = Example::wire_id_300();
    // --------------------------------------------------
    // tag 0x08 and the first varint byte 0xAC only
    // --------------------------------------------------
    let (head, tail) = wire.split_at(2);
    let mut cursor = head;
    let partial = match Example::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(partial)) => partial,
        other => panic!("expected NeedMore, got {other:?}"),
    };
    assert_eq!(cursor, head, "the cut frame must stay unconsumed");
    // --------------------------------------------------
    // resume on the whole frame + tail
    // --------------------------------------------------
    let rest = [cursor, tail].concat();
    let mut cursor = &rest[..];
    let partial = match Example::resume_partial(&mut cursor, partial) {
        Ok(Packet::NeedMore(partial)) => partial,
        other => panic!("expected NeedMore at end of input, got {other:?}"),
    };
    assert_eq!(
        partial.finalize(),
        Ok(Example {
            id: 300,
            ..Example::decoded()
        })
    );
}

/// One-shot decode of an `Inline` varint cut at end of input stops at the frame and
/// finalizes what landed before it
#[test]
fn oneshot_inline_varint_cut_finalizes() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // status varint cut after a continuation byte
        0x18, 0xAC,
    ];
    let mut cursor = &wire[..];
    assert_eq!(
        Reading::decode_value(&mut cursor),
        Ok(Reading::count_only())
    );
    assert_eq!(cursor, &wire[2..], "the cut frame must stay unconsumed");
}

/// One-shot decode of an `Inline` BER-OID cut at end of input stops at the frame and
/// finalizes what landed before it
#[test]
fn oneshot_inline_ber_oid_cut_finalizes() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // level cut after a continuation byte
        0x20, 0x81,
    ];
    let mut cursor = &wire[..];
    assert_eq!(
        Reading::decode_value(&mut cursor),
        Ok(Reading::count_only())
    );
    assert_eq!(cursor, &wire[2..], "the cut frame must stay unconsumed");
}

/// One-shot decode of a header cut at end of input, after the tag and before its length,
/// stops at the frame and finalizes what landed before it
#[test]
fn oneshot_header_cut_finalizes() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // label tag (wire 2 LEN), its length cut off
        0x12,
    ];
    let mut cursor = &wire[..];
    assert_eq!(
        Reading::decode_value(&mut cursor),
        Ok(Reading::count_only())
    );
    assert_eq!(cursor, &wire[2..], "the cut frame must stay unconsumed");
}

/// One-shot decode of a `Sliced` value shorter than its declared length stops at the
/// frame and finalizes what landed before it
#[test]
fn oneshot_truncated_sliced_value_finalizes() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // label = "hi", cut after its first byte
        0x12, 0x02, 0x68,
    ];
    let mut cursor = &wire[..];
    assert_eq!(
        Reading::decode_value(&mut cursor),
        Ok(Reading::count_only())
    );
    assert_eq!(cursor, &wire[2..], "the cut frame must stay unconsumed");
}

/// One-shot truncation before a required field lands fails at finalize, with an error
/// naming the missing field
#[test]
fn oneshot_truncation_missing_required_is_labelled_error() {
    #[rustfmt::skip]
    let wire = [
        // label = "hi", cut after its first byte; count never arrives
        0x12, 0x02, 0x68,
    ];
    match Reading::decode_value(&mut &wire[..]) {
        Err(error) if error.to_string().contains("Reading::count") => {}
        other => panic!("expected an Err naming `Reading::count`, got {other:?}"),
    }
}

/// A header the keylen decoder rejects mid-buffer is still truncation: streaming pauses
/// at the frame, never `Ready`, and one-shot stops at the frame and finalizes
#[test]
fn malformed_header_is_truncation() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // field 1, wire type 3 - unsupported by the keylen decoder
        0x0B, 0x01,
        // label = "hi" - past the bad frame, never reached
        0x12, 0x02, 0x68, 0x69,
    ];
    let mut cursor = &wire[..];
    Reading::count_only().assert_streams_to(&mut cursor);
    assert_eq!(cursor, &wire[2..], "streaming stops at the bad frame");
    let mut cursor = &wire[..];
    assert_eq!(
        Reading::decode_value(&mut cursor),
        Ok(Reading::count_only())
    );
    assert_eq!(cursor, &wire[2..], "one-shot stops at the bad frame");
}

/// An over-long `Inline` varint followed by more frames is malformed: both drivers
/// return an `Err` naming the field, never the earlier fields with the rest dropped
#[test]
fn malformed_inline_varint_is_labelled_error() {
    #[rustfmt::skip]
    let malformed = [
        // id (field 1, wire 0 VARINT) - 11-byte varint, the 10th byte overflows 64 bits
        0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
        // name = "hi" (field 3, wire 2 LEN, Sliced(2))
        0x1A, 0x02, 0x68, 0x69,
    ];
    // --------------------------------------------------
    // every required field lands first, so dropping the rest would finalize Ok
    // --------------------------------------------------
    let wire = [Example::wire(), malformed.to_vec()].concat();
    Example::assert_rejects_naming(&wire, &["Example::id"]);
}

/// An `Inline` decoder that rewinds before rejecting fails away from the end of input,
/// so a bad discriminant is malformed in both drivers, whether more frames follow or not
#[test]
fn rejected_inline_value_is_labelled_error() {
    #[rustfmt::skip]
    let mid_buffer = [
        // count = 3
        0x08, 0x03,
        // status = 9, not a `Status` discriminant
        0x18, 0x09,
        // label = "hi"
        0x12, 0x02, 0x68, 0x69,
    ];
    #[rustfmt::skip]
    let last_frame = [
        // count = 3
        0x08, 0x03,
        // status = 9, not a `Status` discriminant - the final frame
        0x18, 0x09,
    ];
    for wire in [&mid_buffer[..], &last_frame[..]] {
        Reading::assert_rejects_naming(wire, &["Reading::status"]);
    }
}

/// A `ber_oid` value past its type's range, with more frames after it, is malformed in
/// both drivers; the one-shot error keeps the decoder's label, then adds the field's
#[test]
fn overflowing_ber_oid_mid_buffer_is_labelled_error() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // level = 256, past `u8::MAX`
        0x20, 0x82, 0x00,
        // label = "hi"
        0x12, 0x02, 0x68, 0x69,
    ];
    Reading::assert_rejects_naming(&wire, &["Reading::level"]);
    Reading::assert_oneshot_label_chain(&wire, "BER-OID value", "Reading::level");
}

/// A `ber_oid` value past its type's range as the final frame rewinds before rejecting, so
/// it is malformed in both drivers rather than dropped as truncation
#[test]
fn overflowing_ber_oid_last_frame_is_labelled_error() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // level = 256, past `u8::MAX` - the final frame
        0x20, 0x82, 0x00,
    ];
    Reading::assert_rejects_naming(&wire, &["Reading::level"]);
    Reading::assert_oneshot_label_chain(&wire, "BER-OID value", "Reading::level");
}

/// A `BerLength<u8>` value past `u8::MAX`, with more frames after it, is malformed in both
/// drivers; the one-shot error keeps the decoder's label, then adds the field's
#[test]
fn overflowing_ber_length_mid_buffer_is_labelled_error() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // size = 256 in two-byte long form, past `u8::MAX`
        0x28, 0x82, 0x01, 0x00,
        // label = "hi"
        0x12, 0x02, 0x68, 0x69,
    ];
    Reading::assert_rejects_naming(&wire, &["Reading::size"]);
    Reading::assert_oneshot_label_chain(&wire, "BER length value", "Reading::size");
}

/// A `BerLength<u8>` value past `u8::MAX` as the final frame rewinds before rejecting, so it
/// is malformed in both drivers rather than dropped as truncation
#[test]
fn overflowing_ber_length_last_frame_is_labelled_error() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // size = 256 in two-byte long form, past `u8::MAX` - the final frame
        0x28, 0x82, 0x01, 0x00,
    ];
    Reading::assert_rejects_naming(&wire, &["Reading::size"]);
    Reading::assert_oneshot_label_chain(&wire, "BER length value", "Reading::size");
}

/// A long-form `BerLength` `Inline` value split anywhere, including between its two length
/// bytes, pauses, then completes
#[test]
fn split_inside_long_form_ber_length_value() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // size = 200 in two-byte long form (non-minimal, accepted)
        0x28, 0x82, 0x00, 0xC8,
    ];
    let expected = Reading {
        size: Some(BerLength::new(200_u8)),
        ..Reading::count_only()
    };
    expected.assert_every_split_completes(&wire);
}

/// A two-byte BER-OID `Inline` value split anywhere, including between its continuation and
/// terminator bytes, pauses, then completes
#[test]
fn split_inside_ber_oid_value() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // level = 255 in two BER-OID bytes
        0x20, 0x81, 0x7F,
    ];
    let expected = Reading {
        level: Some(255),
        ..Reading::count_only()
    };
    expected.assert_every_split_completes(&wire);
}

/// An `Inline` decoder that consumes a complete final value and then rejects it without
/// rewinding fails at end of input, so the rule classes it as truncation: one-shot drops
/// the field, streaming pauses
#[test]
fn consumed_then_rejected_final_value_is_truncation() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // status = 9, consumed then rejected - the final frame
        0x18, 0x09,
    ];
    let expected = Limits {
        count: 3,
        status: None,
        port: None,
    };
    let mut cursor = &wire[..];
    expected.assert_streams_to(&mut cursor);
    assert_eq!(cursor, &wire[2..], "streaming stops at the rejected frame");
    let mut cursor = &wire[..];
    assert_eq!(Limits::decode_value(&mut cursor), Ok(expected));
    assert_eq!(cursor, &wire[2..], "one-shot stops at the rejected frame");
}

/// An `Inline` decoder that refuses short input without consuming fails away from the end
/// of input, so a cut fixed-width value is classed malformed, not truncation
#[test]
fn short_fixed_inline_value_is_malformed() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // port: one of its two bytes
        0x20, 0x01,
    ];
    Limits::assert_rejects_naming(&wire, &["Limits::port"]);
}

/// A `Sliced` value shorter than its declared length pauses with the frame unconsumed
#[test]
fn truncated_sliced_value_needs_more() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // label = "hi", cut after its first byte
        0x12, 0x02, 0x68,
    ];
    let mut cursor = &wire[..];
    let partial = match Reading::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(partial)) => partial,
        other => panic!("expected NeedMore, got {other:?}"),
    };
    assert_eq!(cursor, &wire[2..], "the cut frame must stay unconsumed");
    let rest = [cursor, &[0x69]].concat();
    match Reading::resume_partial(&mut &rest[..], partial) {
        Ok(Packet::NeedMore(partial)) => assert_eq!(
            partial.finalize(),
            Ok(Reading {
                label: Some("hi".to_owned()),
                ..Reading::count_only()
            })
        ),
        other => panic!("expected NeedMore at end of input, got {other:?}"),
    }
}
