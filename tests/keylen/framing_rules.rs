//! Truncation-vs-malformed tests for the fused `#[klv(keylen(..))]` codec, through both drivers
//!
//! Drives `decode_partial` / `resume_partial` across chunk boundaries and `decode_value`
//! over cut and corrupt buffers. Any fused-header failure is truncation: streaming returns
//! `NeedMore`, one-shot stops at the frame and finalizes. An `Inline` value that fails at
//! end of input is truncation too; one that fails mid-buffer is malformed (an `Err`
//! labelled `<Struct>::<field>`). Either driver decodes an `Inline` value in one read
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::scans::Scans;
use crate::common::varint;
use crate::support::example::{Example, Reading};
use crate::support::{header, value};
use tinyklv::codecs::ber::BerLength;
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;

/// Message whose `Inline` port uses a decoder the end-of-input rule reaches but misclassifies
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32),
)]
struct Limits {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    count: i32,

    /// field 4, wire 0 - `Inline`, a fixed two-byte value
    #[klv(key = 0x20, dec = decb::be_u16)]
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
    /// `Sliced(1)`
    #[klv(key = 0x0101, dec = decb::u8)]
    flag: u8,

    /// `Sliced(128)` - length in two-byte BER long form `0x82 0x00 0x80`: non-minimal, accepted
    #[klv(key = 0x0102, dec = value::dec_string)]
    text: String,
}

/// [`Wide`] implementation
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

/// Message whose `Inline` varints [`Scans`] counts every read of
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    default(typ = u64, dec = Scans::dec_u64),
)]
struct Counters {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    sent: u64,

    /// field 2, wire 0 VARINT - `Inline`
    #[klv(key = 0x10)]
    lost: u64,
}

/// Every two-chunk split of the canonical wire decodes to the same struct as one-shot,
/// including inside a two-byte tag, between a tag and its length, and inside a two-byte
/// length prefix
#[test]
fn split_at_every_offset() {
    Example::decoded().assert_decodes_from(&Example::wire());
}

/// A header built from crate codecs still pauses at every split and completes, whether the
/// codec fails short of the end of input (`be_u16` on one byte) or consumes to it (a BER
/// long-form length cut inside its length bytes)
#[test]
fn split_inside_crate_codec_header() {
    Wide::decoded().assert_decodes_from(&Wide::wire());
}

/// One-shot decode of an `Inline` or a `Sliced` value cut at end of input stops at the frame
/// and finalizes what landed before it
#[test]
fn oneshot_cut_value_finalizes() {
    #[rustfmt::skip]
    let cuts: [&[u8]; 2] = [
        // level, an `Inline` BER-OID cut after a continuation byte
        &[0x20, 0x81],
        // label = "hi", `Sliced(2)` cut after its first byte
        &[0x12, 0x02, 0x68],
    ];
    for cut in cuts {
        // count = 3, then the cut frame
        let wire = [&[0x08, 0x03][..], cut].concat();
        let mut cursor = &wire[..];
        assert_eq!(
            Reading::decode_value(&mut cursor),
            Ok(Reading::count_only())
        );
        assert_eq!(cursor, cut, "the cut frame must stay unconsumed");
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

/// A `ber_oid` value past its type's range, with more frames after it, is malformed in
/// both drivers; the one-shot error names the field, then keeps the decoder's label
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
    Reading::assert_rejects_naming(
        &wire,
        &["`Reading::level` value (its decoder failed before the end of the input)"],
    );
    Reading::assert_oneshot_label_chain(&wire, "Reading::level", "BER-OID value");
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
    Reading::assert_rejects_naming(
        &wire,
        &["`Reading::size` value (its decoder failed before the end of the input)"],
    );
    Reading::assert_oneshot_label_chain(&wire, "Reading::size", "BER length value");
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
    expected.assert_decodes_from(&wire);
}

/// An `Inline` varint value split anywhere, including between its two bytes, pauses, then
/// completes to the value one-shot decodes
#[test]
fn split_inside_varint_value() {
    #[rustfmt::skip]
    let wire = [
        // count = 300, a two-byte varint
        0x08, 0xAC, 0x02,
    ];
    let expected = Reading {
        count: 300,
        ..Reading::count_only()
    };
    expected.assert_decodes_from(&wire);
}

/// An `Inline` fixed-width value split anywhere, including between its two bytes, pauses, then
/// completes to the value one-shot decodes: a short read leaves the cursor at the end of input
#[test]
fn split_inside_fixed_inline_value() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // port = 0x0102
        0x20, 0x01, 0x02,
    ];
    let expected = Limits {
        count: 3,
        port: Some(0x0102),
    };
    expected.assert_decodes_from(&wire);
}

/// An `Inline` value is read once per frame, by one-shot and by streaming over the whole input
#[test]
fn inline_value_is_read_once_by_either_driver() {
    #[rustfmt::skip]
    let wire = [
        // sent = 300 (field 1, wire 0 VARINT, Inline)
        0x08, 0xAC, 0x02,
        // lost = 5 (field 2, wire 0 VARINT, Inline)
        0x10, 0x05,
    ];
    let expected = Counters { sent: 300, lost: 5 };
    let (decoded, scans) = Scans::count(|| Counters::decode_value(&mut &wire[..]));
    assert_eq!(decoded.as_ref(), Ok(&expected));
    assert_eq!(scans, 2, "one-shot: one read per frame");
    let ((), scans) = Scans::count(|| expected.assert_streams_to(&mut &wire[..]));
    assert_eq!(scans, 2, "streaming: one read per frame");
}
