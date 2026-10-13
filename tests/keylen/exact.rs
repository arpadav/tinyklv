//! `size(exact = N)` on a field of a fused `#[klv(keylen(..))]` struct
//!
//! The header decides each frame's framing. A `Sliced` frame carries a length, which must be the
//! field's `N`: any other is rejected in both drivers with an error naming the field and its
//! width, even when the frame runs past the end of the input, so a `lengthed` decoder only ever
//! reads a frame `N` bytes long. A rejected frame the input holds whole is consumed, so a decode
//! resumed after the error starts at the next frame; one that runs past the end is left whole.
//! Encode writes `N` as the length of a frame that carries one. An `Inline` frame carries no
//! length: an exact-width field's decoder reads the main input unchecked, and a `lengthed` one has
//! no length to read by, so its value is malformed; a malformed value leaves the cursor where its
//! decoder stopped
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::common::wire;
use crate::support::header;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::net::Ipv4Addr;

/// A fix's sequence number, a VARINT on the wire
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sequence(u64);

/// [`Sequence`] implementation of [`DecodeValue`]: a varint
impl DecodeValue<&[u8]> for Sequence {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        varint::dec_u64.map(Self).parse_next(input)
    }
}

/// [`Sequence`] implementation of [`EncodeValue`]: a varint
impl EncodeValue for Sequence {
    fn encode_value(&self, out: &mut Vec<u8>) {
        varint::enc_u64(self.0, out);
    }
}

/// Seconds since the Unix epoch, a little-endian `u64` on the wire
#[derive(Debug, Clone, Copy, PartialEq)]
struct Timestamp(u64);

/// [`Timestamp`] implementation of [`DecodeValue`]: a little-endian `u64`
impl DecodeValue<&[u8]> for Timestamp {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        decb::le_u64.map(Self).parse_next(input)
    }
}

/// [`Timestamp`] implementation of [`EncodeValue`]: a little-endian `u64`
impl EncodeValue for Timestamp {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::le_u64(self.0, out);
    }
}

/// The address of the relay that forwarded a fix
#[derive(Debug, Clone, Copy, PartialEq)]
struct RelayAddr(Ipv4Addr);

/// [`RelayAddr`] implementation
impl RelayAddr {
    /// Decodes an address from `width` big-endian bytes, through the crate's length-taking
    /// decoder
    ///
    /// # Arguments
    ///
    /// * `width` - the bytes to read, the length the derive hands a `lengthed` decoder
    ///
    /// # Returns
    ///
    /// A decoder reading `width` bytes, or failing on fewer
    fn decode_lengthed(width: usize) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Self> {
        move |input| {
            decb::be_u32_lengthed(width)
                .map(|bits| Self(Ipv4Addr::from(bits)))
                .parse_next(input)
        }
    }
}

/// [`RelayAddr`] implementation of [`EncodeValue`]: the four octets
impl EncodeValue for RelayAddr {
    fn encode_value(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0.octets());
    }
}

/// GPS fix keyed by field number whose fixed-width fields declare their width, so the wire type
/// a frame arrives with decides its length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_field, enc = Fix::enc_header),
    trait_fallback,
)]
struct Fix {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 1)]
    seq: Sequence,

    /// field 2, a fixed64: wire 1 I64 frames it `Sliced(8)`
    #[klv(key = 2, size(exact = 8))]
    stamp: Option<Timestamp>,

    /// field 3, four big-endian bytes: wire 2 LEN, read by the crate's length-taking decoder
    #[klv(
        key = 3,
        dec = RelayAddr::decode_lengthed,
        enc = RelayAddr::encode_value,
        size(lengthed, exact = 4),
    )]
    addr: Option<RelayAddr>,
}

/// [`Fix`] implementation
impl Fix {
    /// seq = 7 (field 1, wire 0 VARINT, Inline)
    const SEQ: &[u8] = &[0x08, 0x07];

    /// stamp = 1_700_000_000 (field 2, wire 1 I64, Sliced(8)) - u64 little-endian
    #[rustfmt::skip]
    const STAMP: &[u8] = &[
        0x11,
        0x00, 0xF1, 0x53, 0x65, 0x00, 0x00, 0x00, 0x00,
    ];

    /// addr = 10.0.0.1 (field 3, wire 2 LEN, Sliced(4))
    const ADDR: &[u8] = &[0x1A, 0x04, 0x0A, 0x00, 0x00, 0x01];

    /// The full wire encoding of [`Fix::decoded`]
    fn wire() -> Vec<u8> {
        [Self::SEQ, Self::STAMP, Self::ADDR].concat()
    }

    /// The canonical decoded [`Fix`] matching [`Fix::wire`]
    fn decoded() -> Self {
        Self {
            seq: Sequence(7),
            stamp: Some(Timestamp(1_700_000_000)),
            addr: Some(RelayAddr(Ipv4Addr::new(10, 0, 0, 1))),
        }
    }

    /// What [`Fix::SEQ`] alone decodes to
    fn seq_only() -> Self {
        Self {
            seq: Sequence(7),
            stamp: None,
            addr: None,
        }
    }

    /// Encodes a frame header keyed by field number, each field with the wire type it is sent as
    ///
    /// # Arguments
    ///
    /// * `(field, value_len)` - the field number and the byte length of the value that follows
    /// * `out` - the buffer the header bytes are appended to
    fn enc_header((field, value_len): (u32, usize), out: &mut Vec<u8>) {
        let wire_type = match field {
            // stamp
            2 => wire::I64,
            // addr
            3 => wire::LEN,
            // seq
            _ => wire::VARINT,
        };
        header::enc_field(wire_type, (field, value_len), out);
    }
}

/// A relay address declared six bytes wide, which its four octets never fill
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_field, enc = Fix::enc_header),
    allow_unimplemented_decode,
)]
struct WideRelay {
    /// field 3, wire 2 LEN
    #[klv(key = 3, enc = RelayAddr::encode_value, size(exact = 6))]
    addr: RelayAddr,
}

/// Encode writes each exact width where the header carries a length, and the bytes decode back
#[test]
fn encode_writes_each_exact_width_as_the_length() {
    let encoded = Fix::decoded().encoded();
    assert_eq!(encoded, Fix::wire());
    Fix::decoded().assert_decodes_from(&encoded);
}

/// A value its encoder writes in another byte count than its exact width fails the debug check:
/// four octets for a six-byte `addr`, staged ahead of the header that frames it
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "the value encoder of `addr` wrote a different byte count")]
fn a_value_encoded_in_another_width_fails_the_debug_check() {
    WideRelay {
        addr: RelayAddr(Ipv4Addr::new(10, 0, 0, 1)),
    }
    .encoded();
}

/// A mismatched length is rejected before the frame is classed as truncation, so a frame running
/// past the end of the input errors rather than waiting for the rest, leaving the cursor at the
/// frame start
#[test]
fn a_mismatched_length_is_rejected_even_past_the_end_of_the_input() {
    let message = "`Fix::stamp` value (its frame length does not match `size(exact = 8)`)";
    #[rustfmt::skip]
    let wire = [
        Fix::SEQ,
        // stamp (field 2, wire 2 LEN, Sliced(64)), 2 bytes of it sent
        &[0x12, 0x40, 0x09, 0x00],
    ]
    .concat();
    Fix::assert_rejects_at(&wire, Fix::SEQ.len(), message);
}

/// Both drivers word a mismatched length the same way and leave the cursor past the frame
#[test]
fn the_rejection_names_the_field_and_leaves_the_cursor_past_the_frame() {
    let message = "`Fix::stamp` value (its frame length does not match `size(exact = 8)`)";
    #[rustfmt::skip]
    let stamp: &[u8] = &[
        // stamp (field 2, wire 2 LEN, Sliced(9))
        0x12, 0x09,
        0x00, 0xF1, 0x53, 0x65, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let wire = [Fix::SEQ, stamp, Fix::ADDR].concat();
    Fix::assert_rejects_at(&wire, Fix::SEQ.len() + stamp.len(), message);
}

/// A malformed `Inline` value is rejected naming the field, and both drivers leave the cursor
/// where its decoder stopped: on the 10th byte of an overlong varint, short of the end of the
/// input
#[test]
fn a_malformed_inline_value_leaves_the_cursor_where_its_decoder_stopped() {
    let message = "`Fix::seq` value (its decoder failed before the end of the input)";
    #[rustfmt::skip]
    let seq: &[u8] = &[
        // seq (field 1, wire 0 VARINT, Inline), 9 continuation bytes
        0x08,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];
    let wire = [
        seq,
        // a 10th byte carrying bits past the 64th
        &[0x02],
        Fix::STAMP,
    ]
    .concat();
    Fix::assert_rejects_at(&wire, seq.len(), message);
}

/// An `Inline` frame carries no length: the exact-width field's decoder reads its value from
/// the main input, unchecked, and both drivers decode it alike at every chunk split, one inside
/// the value included
#[test]
fn an_inline_frame_has_no_length_to_check() {
    #[rustfmt::skip]
    let wire = [
        Fix::SEQ,
        // stamp (field 2, wire 0 VARINT, Inline)
        &[0x10],
        // 1_700_000_000, read by le_u64 straight off the input
        &[0x00, 0xF1, 0x53, 0x65, 0x00, 0x00, 0x00, 0x00],
    ]
    .concat();
    let expected = Fix {
        stamp: Some(Timestamp(1_700_000_000)),
        ..Fix::seq_only()
    };
    expected.assert_decodes_from(&wire);
}

/// An `Inline` frame for the `lengthed, exact` field has no length to hand its decoder: it is
/// rejected naming the field at every chunk split, the cursor left at the value start
#[test]
fn an_inline_frame_for_a_lengthed_exact_field_is_malformed() {
    let message =
        "`Fix::addr` value (an `Inline` frame has no length for its `size(lengthed)` decoder)";
    #[rustfmt::skip]
    let wire = [
        Fix::SEQ,
        // addr (field 3, wire 0 VARINT, Inline)
        &[0x18],
        // the bytes a LEN frame would carry, never read
        &[0x0A, 0x00, 0x00, 0x01],
    ]
    .concat();
    let value_start = Fix::SEQ.len() + 1;
    Fix::assert_rejects_at(&wire, value_start, message);
}
