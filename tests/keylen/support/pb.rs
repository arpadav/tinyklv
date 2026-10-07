//! Hand-written wire codecs for the fused-`keylen` tests: protobuf's, and a bare `u8`-key header
//!
//! These are ordinary tinyklv codecs that live entirely test-side: tinyklv never
//! learns the word "protobuf". They mirror the protobuf wire format - a tag
//! varint whose low 3 bits carry the wire type - so the fused `keylen` codec can
//! return `(tag, Framing)` and drive both `Sliced` and `Inline` value framing;
//! [`dec_field_keylen`] returns the field number in place of the tag. The
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
use crate::common::varint;
use tinyklv::Result;
use tinyklv::dec::binary as decb;
use tinyklv::framing::Framing;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{
    error::{ContextError, ParserError},
    stream::Stream,
};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Mask selecting the wire type from a tag's low 3 bits
const WIRE_TYPE_MASK: u32 = 0x07;

/// Width of the wire type in a tag's low bits; the field number sits above it
const WIRE_TYPE_BITS: u32 = 3;

/// Wire type 0 VARINT: self-delimiting
pub(crate) const WIRE_VARINT: u32 = 0;

/// Wire type 1 I64: fixed 8 bytes
pub(crate) const WIRE_I64: u32 = 1;

/// Wire type 2 LEN: a varint length prefix, then that many bytes
pub(crate) const WIRE_LEN: u32 = 2;

/// Wire type 5 I32: fixed 4 bytes
const WIRE_I32: u32 = 5;

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
    let tag = varint::dec_u32(input)?;
    let framing = match tag & WIRE_TYPE_MASK {
        WIRE_VARINT => Framing::Inline,
        WIRE_I64 => Framing::Sliced(8),
        WIRE_LEN => Framing::Sliced(varint::dec_usize(input)?),
        WIRE_I32 => Framing::Sliced(4),
        _ => {
            input.reset(&start);
            return Err(ContextError::from_input(input));
        }
    };
    Ok((tag, framing))
}

/// Decodes one protobuf frame header keyed by field number: [`dec_tag_keylen`] with the wire type
/// shifted off the tag, so one key frames `Sliced` or `Inline` by the wire type it arrives with
///
/// # Arguments
///
/// * `input` - the byte stream positioned at a frame header; advanced past the header on success
///
/// # Returns
///
/// `Ok((field_number, framing))`, or the error of [`dec_tag_keylen`]
pub(crate) fn dec_field_keylen(input: &mut &[u8]) -> Result<(u32, Framing)> {
    dec_tag_keylen(input).map(|(tag, framing)| (tag >> WIRE_TYPE_BITS, framing))
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
    varint::dec_u64(input).map(|value| value as u32 as i32)
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
    varint::enc_u32(tag, out);
    if wire_type == WIRE_LEN {
        varint::enc_usize(value_len, out);
    }
}

/// Encodes one protobuf frame header keyed by field number, the inverse of
/// [`dec_field_keylen`]: [`enc_tag_keylen`] with the tag built from the field number and the wire
/// type the field is sent as
///
/// # Arguments
///
/// * `wire_type` - the wire type the field is sent as
/// * `(field, value_len)` - the field number and the byte length of the value that follows
/// * `out` - the buffer the header bytes are appended to
pub(crate) fn enc_field_keylen(
    wire_type: u32,
    (field, value_len): (u32, usize),
    out: &mut Vec<u8>,
) {
    enc_tag_keylen(((field << WIRE_TYPE_BITS) | wire_type, value_len), out);
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
    varint::enc_u64(i64::from(input) as u64, out);
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
