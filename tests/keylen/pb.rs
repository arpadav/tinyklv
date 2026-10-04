//! Hand-written protobuf-shaped wire codecs for the fused-`keylen` tests
//!
//! These are ordinary tinyklv codecs that live entirely test-side: tinyklv never
//! learns the word "protobuf". They mirror the protobuf wire format - a tag
//! varint whose low 3 bits carry the wire type - so the fused `keylen` codec can
//! return `(tag, Framing)` and drive both `Sliced` and `Inline` value framing
//!
//! Every decoder used as an `Inline` value that rejects a complete value fails short
//! of the end of input, so the decode loop classes the failure as malformed rather
//! than truncated
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::Framing;
use tinyklv::Result;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{
    Parser,
    error::{ContextError, ParserError},
    stream::Stream,
};

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
pub(crate) fn dec_varint_u32(input: &mut &[u8]) -> Result<u32> {
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
pub(crate) fn dec_varint_usize(input: &mut &[u8]) -> Result<usize> {
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
    let framing = match tag & 0x07 {
        0 => Framing::Inline,
        1 => Framing::Sliced(8),
        2 => Framing::Sliced(dec_varint_usize(input)?),
        5 => Framing::Sliced(4),
        _ => {
            input.reset(&start);
            return Err(ContextError::from_input(input));
        }
    };
    Ok((tag, framing))
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
