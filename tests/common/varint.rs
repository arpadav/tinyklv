//! LEB128 varint codecs: the integer encoding of protobuf's tags, lengths and `VARINT` values
//!
//! These are ordinary tinyklv codecs that live entirely test-side, passed to `key(..)`, `len(..)`
//! and `dec = ..`: tinyklv never learns the word "protobuf"
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::Result;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{
    Parser,
    error::{ContextError, ParserError},
};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The bit of a varint byte that says another byte follows
const CONTINUATION: u8 = 0x80;

/// The value bits of a varint byte
const PAYLOAD: u8 = 0x7F;

/// Value bits each varint byte carries
const PAYLOAD_BITS: u32 = 7;

/// The shift of a 10th varint byte, which holds only bit 63 of a `u64`
const LAST_SHIFT: u32 = 63;

/// Decodes one varint as a `u64`
///
/// A failure leaves the cursor where a decode loop tells truncated from malformed: a varint that
/// runs out of bytes consumes them all, so the cursor sits at the end of the input; a 10th byte
/// carrying bits past the 64th is not consumed, so the cursor sits on it, short of the end
///
/// # Arguments
///
/// * `input` - the byte stream to read from; advanced past the varint on success
///
/// # Returns
///
/// `Ok(u64)`, or an error on truncation or on a 10th byte carrying bits past the 64th
pub(crate) fn dec_u64(input: &mut &[u8]) -> Result<u64> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let Some((&byte, rest)) = input.split_first() else {
            return Err(ContextError::from_input(input));
        };
        if shift == LAST_SHIFT && byte > 1 {
            return Err(ContextError::from_input(input));
        }
        *input = rest;
        value |= u64::from(byte & PAYLOAD) << shift;
        if byte & CONTINUATION == 0 {
            return Ok(value);
        }
        shift += PAYLOAD_BITS;
    }
}

/// Decodes one varint as a `u32`, protobuf's tag width
///
/// # Arguments
///
/// * `input` - the byte stream to read from; advanced past the varint on success, left as
///   [`dec_u64`] leaves it on a failed read, and rewound to the varint's start when the value
///   does not fit a `u32`
///
/// # Returns
///
/// `Ok(u32)`, or the error of [`dec_u64`], or an error on a value past `u32::MAX`
pub(crate) fn dec_u32(input: &mut &[u8]) -> Result<u32> {
    dec_u64.try_map(u32::try_from).parse_next(input)
}

/// Decodes one varint as a `usize`, a value length
///
/// # Arguments
///
/// * `input` - the byte stream to read from; advanced past the varint on success, left as
///   [`dec_u64`] leaves it on a failed read, and rewound to the varint's start when the value
///   does not fit a `usize`
///
/// # Returns
///
/// `Ok(usize)`, or the error of [`dec_u64`], or an error on a value past `usize::MAX`
pub(crate) fn dec_usize(input: &mut &[u8]) -> Result<usize> {
    dec_u64.try_map(usize::try_from).parse_next(input)
}

/// Decodes one varint as an `i32`, protobuf's `int32`: the low 32 bits of the `u64`, so a
/// negative value's ten-byte encoding reads back
///
/// # Arguments
///
/// * `input` - the byte stream to read from; advanced past the varint on success, left as
///   [`dec_u64`] leaves it on a failed read
///
/// # Returns
///
/// `Ok(i32)`, or the error of [`dec_u64`]
pub(crate) fn dec_i32(input: &mut &[u8]) -> Result<i32> {
    dec_u64.map(|value| value as i32).parse_next(input)
}

/// Encodes one `u64` as a varint, the inverse of [`dec_u64`]
///
/// # Arguments
///
/// * `input` - the value to encode
/// * `out` - the buffer the varint bytes are appended to
pub(crate) fn enc_u64(mut input: u64, out: &mut Vec<u8>) {
    while input > u64::from(PAYLOAD) {
        out.push((input as u8 & PAYLOAD) | CONTINUATION);
        input >>= PAYLOAD_BITS;
    }
    out.push(input as u8);
}

/// Encodes one `u32` as a varint, the inverse of [`dec_u32`]
///
/// # Arguments
///
/// * `input` - the value to encode
/// * `out` - the buffer the varint bytes are appended to
pub(crate) fn enc_u32(input: u32, out: &mut Vec<u8>) {
    enc_u64(u64::from(input), out);
}

/// Encodes one `i32` as a varint, the inverse of [`dec_i32`]: a negative value sign-extends to
/// ten bytes
///
/// # Arguments
///
/// * `input` - the value to encode
/// * `out` - the buffer the varint bytes are appended to
pub(crate) fn enc_i32(input: i32, out: &mut Vec<u8>) {
    enc_u64(i64::from(input) as u64, out);
}

/// Encodes one `usize` as a varint, the inverse of [`dec_usize`]
///
/// # Arguments
///
/// * `input` - the value to encode
/// * `out` - the buffer the varint bytes are appended to
pub(crate) fn enc_usize(input: usize, out: &mut Vec<u8>) {
    enc_u64(input as u64, out);
}
