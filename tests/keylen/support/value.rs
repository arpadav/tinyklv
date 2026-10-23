//! Hand-written value codecs for the fused-`keylen` tests: a UTF-8 string read to the end of its
//! sub-slice, and packed varint `int32`s
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::varint;
use tinyklv::Result;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{
    error::{ContextError, ParserError},
    stream::Stream,
};

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
    input.finish();
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
        out.push(varint::dec_i32(input)?);
    }
    Ok(out)
}

/// Encodes a wire-2 LEN packed-repeated field of varint `int32`s, the inverse of [`dec_packed_i32`]
///
/// # Arguments
///
/// * `input` - the elements to encode, back to back with no per-element framing
/// * `out` - the buffer the packed varints are appended to
pub(crate) fn enc_packed_i32(input: &[i32], out: &mut Vec<u8>) {
    for &value in input {
        varint::enc_i32(value, out);
    }
}
