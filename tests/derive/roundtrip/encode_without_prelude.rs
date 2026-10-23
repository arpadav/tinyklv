//! Encode-only derives in a module that brings no tinyklv trait into scope
//!
//! The encode half names every trait it calls by its full path, so it compiles beside
//! `use tinyklv::Klv;` alone: a known-width, back-filled, skipped and `implicit` value, the
//! retained unknown frames and the sentinel frame under a fixed-width length slot; a staged value,
//! a trait fallback, a `oneof(..)` arm and a staged sentinel frame under a variable-width length;
//! and a fused `keylen` header with its retained unknown frames. This file failing to compile is
//! the regression
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::Klv;
use tinyklv::enc::binary as encb;

/// A short name, written as its UTF-8 bytes
#[derive(Debug, PartialEq)]
struct Label(String);

/// [`Label`] implementation
impl Label {
    /// Writes the name's bytes
    fn encode(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(self.0.as_bytes());
    }
}

/// A temperature, a big-endian `i16` on the wire
#[derive(Debug, PartialEq)]
struct Celsius(i16);

/// [`Celsius`] implementation of [`tinyklv::traits::EncodeValue`]: a big-endian `i16`
impl tinyklv::traits::EncodeValue for Celsius {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_i16(self.0, out);
    }
}

/// What a reading carries, whichever key holds it
#[derive(Debug, PartialEq)]
enum Payload {
    /// A count, key 0x02
    Count(u8),
    /// A name, key 0x03
    Name(Label),
}

/// Writes a retained frame's length, then its bytes
fn frame(value: &[u8], out: &mut Vec<u8>) {
    encb::u8_from_usize(value.len(), out);
    out.extend_from_slice(value);
}

/// Writes a retained frame's bytes, which the `keylen` header frames
fn body(value: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(value);
}

/// Writes a `keylen` header: the key, then the length
fn header((key, len): (u8, usize), out: &mut Vec<u8>) {
    encb::u8(key, out);
    encb::u8_from_usize(len, out);
}

/// A report under a one-byte length slot, framed by a sentinel
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(enc = encb::u8),
    len(enc = encb::u8_from_usize, size(exact = 1)),
    sentinel = b"RPT",
    unknown(enc = frame),
    allow_unimplemented_decode,
)]
struct Report {
    /// key 0x01, two bytes wide
    #[klv(key = 0x01, enc = *encb::be_u16)]
    level: u16,

    /// key 0x02, its length back-filled
    #[klv(key = 0x02, enc = Label::encode)]
    name: Option<Label>,

    /// key 0x03, left out at zero
    #[klv(key = 0x03, enc = *encb::be_u16, skip_if_default)]
    alarm: u16,

    /// key 0x04, with no length
    #[klv(key = 0x04, enc = *encb::u8, size(implicit))]
    flags: u8,

    /// every other frame
    #[klv(unknown)]
    extra: Vec<(u8, Vec<u8>)>,
}

/// A reading under a variable-width length, framed by a sentinel
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(enc = encb::u8),
    len(enc = encb::u8_from_usize),
    sentinel = b"RD",
    default(typ = u8, enc = *encb::u8),
    default(typ = Label, enc = Label::encode),
    allow_unimplemented_decode,
    trait_fallback,
)]
struct Reading {
    /// key 0x01, by its `EncodeValue` impl
    #[klv(key = 0x01)]
    temp: Celsius,

    /// key 0x02 or 0x03, by its arm
    #[klv(key = oneof(0x02 => Payload::Count(u8), 0x03 => Payload::Name(Label)))]
    payload: Option<Payload>,
}

/// An id under a fused `keylen` header
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(enc = header),
    unknown(enc = body),
    allow_unimplemented_decode,
)]
struct Fused {
    /// key 0x01
    #[klv(key = 0x01, enc = *encb::u8)]
    id: u8,

    /// every other frame
    #[klv(unknown)]
    extra: Vec<(u8, Vec<u8>)>,
}

/// Each struct encodes as it does beside the prelude
#[test]
fn the_encode_half_needs_no_trait_in_scope() {
    let report = Report {
        level: 42,
        name: Some(Label("hi".to_owned())),
        alarm: 0,
        flags: 7,
        extra: vec![(0x09, vec![0x05])],
    };
    #[rustfmt::skip]
    let report_frame = [
        // the sentinel, then the body's length
        0x52, 0x50, 0x54, 0x0D,
        // level = 42
        0x01, 0x02, 0x00, 0x2A,
        // name = "hi"
        0x02, 0x02, 0x68, 0x69,
        // flags = 7, with no length
        0x04, 0x07,
        // the kept frame
        0x09, 0x01, 0x05,
    ];
    let mut out = Vec::new();
    tinyklv::traits::EncodeFrame::encode_frame(&report, &mut out);
    assert_eq!(out, report_frame);
    let reading = Reading {
        temp: Celsius(-10),
        payload: Some(Payload::Name(Label("hi".to_owned()))),
    };
    #[rustfmt::skip]
    let reading_frame = [
        // the sentinel, then the body's length
        0x52, 0x44, 0x08,
        // temp = -10
        0x01, 0x02, 0xFF, 0xF6,
        // payload = "hi", under the name's key
        0x03, 0x02, 0x68, 0x69,
    ];
    let mut out = Vec::new();
    tinyklv::traits::EncodeFrame::encode_frame(&reading, &mut out);
    assert_eq!(out, reading_frame);
    let counted = Reading {
        payload: Some(Payload::Count(3)),
        ..reading
    };
    #[rustfmt::skip]
    let counted_value = [
        // temp = -10
        0x01, 0x02, 0xFF, 0xF6,
        // payload = 3, under the count's key
        0x02, 0x01, 0x03,
    ];
    let mut out = Vec::new();
    tinyklv::traits::EncodeValue::encode_value(&counted, &mut out);
    assert_eq!(out, counted_value);
    let fused = Fused {
        id: 7,
        extra: vec![(0x09, vec![0x05, 0x06])],
    };
    #[rustfmt::skip]
    let fused_value = [
        // id = 7
        0x01, 0x01, 0x07,
        // the kept frame, framed by its header
        0x09, 0x02, 0x05, 0x06,
    ];
    let mut out = Vec::new();
    tinyklv::traits::EncodeValue::encode_value(&fused, &mut out);
    assert_eq!(out, fused_value);
}
