#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 18 - the fused `keylen` codec, framing each value by its key
//! See `book/tutorial/18-keylen.md` for the full narrative
//!
//! The struct of tutorial 17 with one `keylen(dec, enc)` header codec in place
//! of `key(..)` and `len(..)`, and no `size(implicit)` anywhere: the low 3 bits
//! of each key say how its value is framed, so `dec_header` returns
//! `Framing::Inline` for the varint and `Framing::Sliced(n)` for the rest. The
//! wire is byte for byte the one of tutorial 17
use tinyklv::prelude::*;            // Klv proc-macro + traits + Framing
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::dec::string as decs;   // string decoders
use tinyklv::enc::binary as encb;   // binary encoders
use tinyklv::enc::string as encs;   // string encoders

/// The bit of a varint byte that says another byte follows
const MORE: u8 = 0x80;

/// The low 3 bits of a key, its wire type: how the value after it is framed
const WIRE_TYPE: u32 = 0b111;

/// Wire type of a varint, which stops itself
const VARINT: u32 = 0;

/// Wire type of eight bytes
const FIXED64: u32 = 1;

/// Wire type of a varint length, then that many bytes
const PREFIXED: u32 = 2;

/// Wire type of four bytes
const FIXED32: u32 = 5;

/// Reads one LEB128 varint: seven value bits a byte, low bits first, `MORE`
/// set on every byte but the last. Running out of bytes leaves the cursor at
/// the end of the input
fn dec_varint(input: &mut &[u8]) -> tinyklv::Result<u64> {
    let mut value = 0;
    for shift in (0..u64::BITS).step_by(7) {
        let byte = decb::u8(input)?;
        value |= u64::from(byte & !MORE) << shift;
        if byte & MORE == 0 {
            return Ok(value);
        }
    }
    // an eleventh byte: too long for a `u64`
    Err(winnow::error::ContextError::from_input(input))
}

/// Writes one LEB128 varint
fn enc_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= u64::from(MORE) {
        out.push(value as u8 | MORE);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Reads a frame's key, a varint
fn dec_tag(input: &mut &[u8]) -> tinyklv::Result<u32> {
    dec_varint.try_map(u32::try_from).parse_next(input)
}

/// Writes a frame's key
fn enc_tag(tag: u32, out: &mut Vec<u8>) {
    enc_varint(tag.into(), out);
}

/// Reads a value's length, a varint
fn dec_len(input: &mut &[u8]) -> tinyklv::Result<usize> {
    dec_varint.try_map(usize::try_from).parse_next(input)
}

/// Writes a value's length
fn enc_len(len: usize, out: &mut Vec<u8>) {
    enc_varint(len as u64, out);
}

/// Reads a frame header: the key, then the framing its wire type names
fn dec_header(input: &mut &[u8]) -> tinyklv::Result<(u32, Framing)> {
    let key = dec_tag(input)?;
    let framing = match key & WIRE_TYPE {
        VARINT   => Framing::Inline,
        FIXED64  => Framing::Sliced(8),
        PREFIXED => Framing::Sliced(dec_len(input)?),
        FIXED32  => Framing::Sliced(4),
        _        => return Err(winnow::error::ContextError::from_input(input)),
    };
    Ok((key, framing))
}

/// Writes a frame header: the key, and the value's length when its wire type
/// has one
fn enc_header((key, len): (u32, usize), out: &mut Vec<u8>) {
    enc_tag(key, out);
    if key & WIRE_TYPE == PREFIXED {
        enc_len(len, out);
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = dec_header, enc = enc_header),
    default(typ = u64, dec = dec_varint, enc = *enc_varint),
)]
/// The reading of tutorial 17, framed by its keys
struct Reading {
    #[klv(key = 0x08)]
    /// `VARINT`: `Inline`, the varint read from the main stream
    count: u64,

    #[klv(key = 0x15, dec = decb::le_f32, enc = *encb::le_f32)]
    /// `FIXED32`: `Sliced(4)`
    ratio: f32,

    #[klv(
        key = 0x1A,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    /// `PREFIXED`: `Sliced(len)`, the length passed to the decoder
    name: String,
}

fn main() {
    // manually construct the stream
    let stream = [
        // count: key, varint 300, no length
        0x08, 0xAC, 0x02,
        // ratio: key, 1.5 as four little-endian bytes, no length
        0x15, 0x00, 0x00, 0xC0, 0x3F,
        // name: key, length 2, "hi"
        0x1A, 0x02, 0x68, 0x69,
    ];

    // manually construct the expected value
    let expected = Reading {
        count: 300,
        ratio: 1.5,
        name:  String::from("hi"),
    };

    // encode: `enc_header` writes a length for the name only
    let mut encoded = Vec::new();
    expected.encode_value(&mut encoded);
    assert_eq!(encoded, stream);

    // decode: `dec_header` frames each value by its key
    let decoded = Reading::decode_value(
        &mut stream.as_slice(),
    ).unwrap();
    assert_eq!(decoded, expected);
}
