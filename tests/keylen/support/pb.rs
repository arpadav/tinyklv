//! Hand-written wire codecs for the fused-`keylen` tests: protobuf's, and a bare `u8`-key header
//!
//! These are ordinary tinyklv codecs that live entirely test-side: tinyklv never
//! learns the word "protobuf". They mirror the protobuf wire format - a tag
//! varint whose low 3 bits carry the wire type - so the fused `keylen` codec can
//! return `(tag, Framing)` and drive both `Sliced` and `Inline` value framing. The
//! `enc_*` encoders are the inverses the fused encode path writes with. [`dec_u8_keylen`] and
//! [`enc_u8_keylen`] are the bare header of the tests that need no protobuf: a `u8` key, every
//! value `Inline`
//!
//! Every decoder used as an `Inline` value that rejects a complete value fails short
//! of the end of input, so the decode loop classes the failure as malformed rather
//! than truncated
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::Result;
use tinyklv::dec::binary as decb;
use tinyklv::framing::Framing;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{
    Parser,
    error::{ContextError, ParserError},
    stream::Stream,
};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Mask selecting the wire type from a tag's low 3 bits
const WIRE_TYPE_MASK: u32 = 0x07;

/// Wire type 0 VARINT: self-delimiting
const WIRE_VARINT: u32 = 0;

/// Wire type 1 I64: fixed 8 bytes
const WIRE_I64: u32 = 1;

/// Wire type 2 LEN: a varint length prefix, then that many bytes
const WIRE_LEN: u32 = 2;

/// Wire type 5 I32: fixed 4 bytes
const WIRE_I32: u32 = 5;

/// Decodes one LEB128 varint as a `u64`, consuming continuation bytes
///
/// # Arguments
///
/// * `input` - the byte stream to read from; advanced past the varint on success
///
/// # Returns
///
/// `Ok(u64)` with the decoded value, or an error on truncation or on a 10th byte
/// carrying bits past the 64th
pub(crate) fn dec_varint_u64(input: &mut &[u8]) -> Result<u64> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let Some((&byte, rest)) = input.split_first() else {
            return Err(ContextError::from_input(input));
        };
        // --------------------------------------------------
        // the 10th byte holds only bit 63: anything above 1 overflows
        // rejected before advancing, so the cursor sits on the bad byte
        // --------------------------------------------------
        if shift == 63 && byte > 1 {
            return Err(ContextError::from_input(input));
        }
        *input = rest;
        result |= u64::from(byte & 0x7F) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
        shift += 7;
    }
}

/// Decodes one LEB128 varint as a `u32` (the protobuf tag width)
///
/// # Arguments
///
/// * `input` - the byte stream to read from; advanced past the varint on success
///
/// # Returns
///
/// `Ok(u32)`, or an error on truncation or if the value does not fit in a `u32`
fn dec_varint_u32(input: &mut &[u8]) -> Result<u32> {
    dec_varint_u64.try_map(u32::try_from).parse_next(input)
}

/// Decodes one LEB128 varint as a `usize` length prefix
///
/// # Arguments
///
/// * `input` - the byte stream to read from; advanced past the varint on success
///
/// # Returns
///
/// `Ok(usize)`, or an error on truncation or if the value does not fit in a `usize`
fn dec_varint_usize(input: &mut &[u8]) -> Result<usize> {
    dec_varint_u64.try_map(usize::try_from).parse_next(input)
}

/// Decodes one protobuf frame header: the tag varint (the key), then the framing
///
/// The framing is derived from the wire type in the tag's low 3 bits. This is the
/// entire protobuf-specific delta - the rest is ordinary value decoding:
///
/// * wire 0 VARINT -> [`Framing::Inline`] (self-delimiting, read without `take`)
/// * wire 1 I64    -> [`Framing::Sliced(8)`](Framing::Sliced)
/// * wire 2 LEN    -> [`Framing::Sliced(n)`](Framing::Sliced) after consuming the length prefix
/// * wire 5 I32    -> [`Framing::Sliced(4)`](Framing::Sliced)
///
/// # Arguments
///
/// * `input` - the byte stream positioned at a frame header; advanced past the header on success
///
/// # Returns
///
/// `Ok((tag, framing))`, or an error on truncation or an unsupported wire type
pub(crate) fn dec_tag_keylen(input: &mut &[u8]) -> Result<(u32, Framing)> {
    let start = input.checkpoint();
    let tag = dec_varint_u32(input)?;
    let framing = match tag & WIRE_TYPE_MASK {
        WIRE_VARINT => Framing::Inline,
        WIRE_I64 => Framing::Sliced(8),
        WIRE_LEN => Framing::Sliced(dec_varint_usize(input)?),
        WIRE_I32 => Framing::Sliced(4),
        _ => {
            input.reset(&start);
            return Err(ContextError::from_input(input));
        }
    };
    Ok((tag, framing))
}

/// Decodes a frame header: a `u8` key, every value `Inline`
///
/// # Arguments
///
/// * `input` - the byte stream positioned at a frame header; advanced past the key
///
/// # Returns
///
/// `Ok((key, Framing::Inline))`, or an error on empty input
pub(crate) fn dec_u8_keylen(input: &mut &[u8]) -> Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Inline))
}

/// Encodes a frame header, the inverse of [`dec_u8_keylen`]: the `u8` key alone, since every
/// value is `Inline` and its length never reaches the wire
///
/// # Arguments
///
/// * `(key, _)` - the field key and the (unwritten) byte length of the value that follows
/// * `out` - the buffer the key byte is appended to
pub(crate) fn enc_u8_keylen((key, _): (u8, usize), out: &mut Vec<u8>) {
    out.push(key);
}

/// Decodes a wire-0 VARINT value as a protobuf `int32`
///
/// Reads the varint and reinterprets its low 32 bits as a signed `i32`, matching
/// protobuf `int32` semantics
///
/// # Arguments
///
/// * `input` - the byte stream positioned at the value; advanced past the varint on success
///
/// # Returns
///
/// `Ok(i32)`, or an error on truncation
pub(crate) fn dec_varint_i32(input: &mut &[u8]) -> Result<i32> {
    dec_varint_u64(input).map(|value| value as u32 as i32)
}

/// Decodes a wire-2 LEN value as a UTF-8 `String`, consuming the whole sub-slice
///
/// Under `Sliced` framing the decode loop hands this the exact value bytes, so it
/// reads everything remaining in `input`
///
/// # Arguments
///
/// * `input` - the bounded value sub-slice; fully consumed on success
///
/// # Returns
///
/// `Ok(String)`, or an error if the bytes are not valid UTF-8
pub(crate) fn dec_string(input: &mut &[u8]) -> Result<String> {
    let decoded = String::from_utf8(input.to_vec()).map_err(|_| ContextError::from_input(input))?;
    *input = &[];
    Ok(decoded)
}

/// Decodes a wire-2 LEN packed-repeated field of varint `int32`s, looping to sub-slice EOF
///
/// # Arguments
///
/// * `input` - the bounded value sub-slice; fully consumed on success
///
/// # Returns
///
/// `Ok(Vec<i32>)`, or an error if a packed element is truncated
pub(crate) fn dec_packed_i32(input: &mut &[u8]) -> Result<Vec<i32>> {
    let mut out = Vec::new();
    while !input.is_empty() {
        out.push(dec_varint_i32(input)?);
    }
    Ok(out)
}

/// Encodes one `u64` as a LEB128 varint, the inverse of [`dec_varint_u64`]
///
/// # Arguments
///
/// * `input` - the value to encode
/// * `out` - the buffer the varint bytes are appended to
pub(crate) fn enc_varint_u64(mut input: u64, out: &mut Vec<u8>) {
    while input >= 0x80 {
        out.push((input as u8) | 0x80);
        input >>= 7;
    }
    out.push(input as u8);
}

/// Encodes one `u32` as a LEB128 varint (the protobuf tag width), the inverse of [`dec_varint_u32`]
///
/// # Arguments
///
/// * `input` - the value to encode
/// * `out` - the buffer the varint bytes are appended to
fn enc_varint_u32(input: u32, out: &mut Vec<u8>) {
    enc_varint_u64(u64::from(input), out);
}

/// Encodes one `usize` length prefix as a LEB128 varint, the inverse of [`dec_varint_usize`]
///
/// # Arguments
///
/// * `input` - the length to encode
/// * `out` - the buffer the varint bytes are appended to
fn enc_varint_usize(input: usize, out: &mut Vec<u8>) {
    enc_varint_u64(input as u64, out);
}

/// Encodes one protobuf frame header, the inverse of [`dec_tag_keylen`] for wire types 0/1/2/5
///
/// Writes the tag varint, then the value length as a varint only for wire 2 LEN: every
/// other wire type is either self-delimiting (wire 0) or fixed width (wires 1 and 5), so
/// its length never reaches the wire. Any other wire type is a test bug, caught by a
/// `debug_assert!`
///
/// # Arguments
///
/// * `(tag, value_len)` - the field tag and the byte length of the value that follows
/// * `out` - the buffer the header bytes are appended to
pub(crate) fn enc_tag_keylen((tag, value_len): (u32, usize), out: &mut Vec<u8>) {
    let wire_type = tag & WIRE_TYPE_MASK;
    debug_assert!(
        matches!(wire_type, WIRE_VARINT | WIRE_I64 | WIRE_LEN | WIRE_I32),
        "unsupported wire type {wire_type} in tag {tag}"
    );
    enc_varint_u32(tag, out);
    if wire_type == WIRE_LEN {
        enc_varint_usize(value_len, out);
    }
}

/// Encodes a protobuf `int32` as a wire-0 VARINT, the inverse of [`dec_varint_i32`]
///
/// A negative value is sign-extended to 64 bits, so it always takes ten bytes, matching
/// protobuf `int32` semantics
///
/// # Arguments
///
/// * `input` - the value to encode
/// * `out` - the buffer the varint bytes are appended to
pub(crate) fn enc_varint_i32(input: i32, out: &mut Vec<u8>) {
    enc_varint_u64(i64::from(input) as u64, out);
}

/// Encodes a wire-2 LEN packed-repeated field of varint `int32`s, the inverse of [`dec_packed_i32`]
///
/// # Arguments
///
/// * `input` - the elements to encode, back to back with no per-element framing
/// * `out` - the buffer the packed varints are appended to
pub(crate) fn enc_packed_i32(input: &[i32], out: &mut Vec<u8>) {
    for &value in input {
        enc_varint_i32(value, out);
    }
}
