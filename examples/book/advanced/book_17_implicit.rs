//! Book tutorial 17 - `size(implicit)` values with no length prefix
//! See `book/src/tutorial/advanced/17-implicit.md` for the full narrative
//!
//! A key + len struct whose `count` is a varint written with no length after
//! its key, and whose `ratio` is four bytes with no length, beside a `name`
//! that keeps its length prefix. The keys, lengths and the varint value are
//! LEB128 varints, read and written by the codecs below
use tinyklv::dec::binary as decb; // binary decoders
use tinyklv::dec::string as decs; // string decoders
use tinyklv::enc::binary as encb; // binary encoders
use tinyklv::enc::string as encs; // string encoders
use tinyklv::prelude::*; // Klv proc-macro + traits

/// The bit of a varint byte that says another byte follows
const MORE: u8 = 0x80;

/// Reads one LEB128 varint: seven value bits a byte, low bits first, `MORE`
/// set on every byte but the last. Running out of bytes leaves the cursor at
/// the end of the input, and a varint too long for a `u64` rewinds it
fn dec_varint(input: &mut &[u8]) -> tinyklv::Result<u64> {
    let start = input.checkpoint();
    let mut value = 0;
    for shift in (0..u64::BITS).step_by(7) {
        let byte = decb::u8(input)?;
        value |= u64::from(byte & !MORE) << shift;
        if byte & MORE == 0 && u64::from(byte) <= u64::MAX >> shift {
            return Ok(value);
        }
    }
    // an eleventh byte, or a tenth past the 64th bit: too long for a `u64`,
    // so the cursor goes back to the varint, which a stream can never finish
    input.reset(&start);
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

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = dec_tag, enc = enc_tag),
    len(dec = dec_len, enc = enc_len),
    default(
        typ = u64,
        dec = dec_varint,
        enc = *enc_varint,
        size(implicit),
    ),
)]
/// A reading with two values that carry no length and one that does
struct Reading {
    #[klv(key = 0x08)]
    /// A varint, which stops itself: no length on the wire
    count: u64,

    #[klv(
        key = 0x15,
        dec = decb::le_f32,
        enc = *encb::le_f32,
        size(implicit, exact = 4),
    )]
    /// Four little-endian bytes: no length on the wire, `exact = 4` takes them
    ratio: f32,

    #[klv(
        key = 0x1A,
        dec = decs::to_string_utf8_strict,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    /// A string: a length prefix, then that many bytes
    name: String,
}

fn main() {
    // manually construct the stream
    #[rustfmt::skip]
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
        name: String::from("hi"),
    };

    // encode: no length after the `implicit` keys
    let mut encoded = Vec::new();
    expected.encode_value(&mut encoded);
    assert_eq!(encoded, stream);

    // decode: the varint stops itself, the ratio takes 4 bytes
    let decoded = Reading::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(decoded, expected);
}
