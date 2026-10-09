//! [`Unknown`], the value of a protobuf frame no field declares, and the `unknown(dec, enc)`
//! codecs that keep it
//!
//! A value is read by the wire type in its tag's low 3 bits. Each framing mode has its own pair.
//! In key + len mode the decoder reads everything after the key from the main stream, a LEN
//! frame's length included, and the encoder writes that length back. In keylen mode the header
//! has framed the value, so the decoder reads the body it was left, the bounded slice of a
//! `Sliced` frame or the main stream of an `Inline` one, and the encoder writes the body alone
//!
//! On the main stream, a value cut short consumes the input to its end, which a decode loop reads
//! as truncation. A wire type protobuf does not frame (3, 4, 6 or 7) is rejected with nothing
//! consumed, which a decode loop reads as malformed when bytes follow the key, but as truncation
//! when the key ends the input
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::{varint, wire};
use tinyklv::Result;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::framing::Framing;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::{
    error::{ContextError, ParserError},
    stream::Stream,
};

/// The value of a frame no field declares, one variant per wire type protobuf frames
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unknown {
    /// Wire type 0 VARINT
    Varint(u64),
    /// Wire type 1 I64, little-endian
    I64(u64),
    /// Wire type 2 LEN, the bytes after the length
    Len(Vec<u8>),
    /// Wire type 5 I32, little-endian
    I32(u32),
}

/// Decodes the value of a key + len frame: everything after its key
///
/// The value is framed by [`wire::framing`], the table a keylen header frames by: a VARINT is read
/// from the main stream, a fixed-width or LEN value from the bytes its width or length prefix
/// takes, each as [`unknown_body_dec`] reads that body
///
/// # Arguments
///
/// * `tag` - the frame's key, whose low 3 bits are its wire type
///
/// # Returns
///
/// A decoder reading the value from the main stream, failing as [`unknown_body_dec`] does, on a
/// failed length read, or with the input consumed to its end on a value cut short
pub(crate) fn unknown_dec(tag: u32) -> impl Fn(&mut &[u8]) -> Result<Unknown> {
    move |input| match wire::framing(tag, input)? {
        Some(Framing::Inline) => unknown_body_dec(tag)(input),
        Some(Framing::Sliced(width)) => unknown_body_dec(tag)(&mut take_or_drain(width, input)?),
        None => Err(ContextError::from_input(input)),
    }
}

/// Decodes the body of a keylen frame, as its header left it
///
/// # Arguments
///
/// * `tag` - the frame's key, whose low 3 bits are its wire type
///
/// # Returns
///
/// A decoder reading a VARINT from the main stream of an `Inline` frame, or a fixed-width or LEN
/// value from the bounded slice of a `Sliced` one, a LEN value being the whole slice; it fails on
/// a VARINT that [`varint::dec_u64`] rejects, or with nothing consumed on a wire type protobuf does
/// not frame
pub(crate) fn unknown_body_dec(tag: u32) -> impl Fn(&mut &[u8]) -> Result<Unknown> {
    move |input| match tag & wire::TYPE_MASK {
        wire::VARINT => varint::dec_u64(input).map(Unknown::Varint),
        wire::I64 => decb::le_u64(input).map(Unknown::I64),
        wire::LEN => Ok(Unknown::Len(input.finish().to_vec())),
        wire::I32 => decb::le_u32(input).map(Unknown::I32),
        _ => Err(ContextError::from_input(input)),
    }
}

/// Encodes the value of a key + len frame, the inverse of [`unknown_dec`]: a LEN value's length,
/// then the body [`unknown_body_enc`] writes
///
/// # Arguments
///
/// * `value` - the value to encode
/// * `out` - the buffer the value bytes are appended to
pub(crate) fn unknown_enc(value: &Unknown, out: &mut Vec<u8>) {
    if let Unknown::Len(bytes) = value {
        varint::enc_usize(bytes.len(), out);
    }
    unknown_body_enc(value, out);
}

/// Encodes the body of a keylen frame, the inverse of [`unknown_body_dec`]: the keylen encoder
/// writes the header, a LEN value's length included
///
/// # Arguments
///
/// * `value` - the value to encode
/// * `out` - the buffer the body bytes are appended to
pub(crate) fn unknown_body_enc(value: &Unknown, out: &mut Vec<u8>) {
    match value {
        Unknown::Varint(value) => varint::enc_u64(*value, out),
        Unknown::I64(value) => encb::le_u64(*value, out),
        Unknown::Len(bytes) => out.extend_from_slice(bytes),
        Unknown::I32(value) => encb::le_u32(*value, out),
    }
}

/// Takes the next `width` bytes of the main stream
///
/// # Arguments
///
/// * `width` - the bytes to take
/// * `input` - the main stream; advanced past them, or consumed to its end when fewer remain, so
///   a value cut short leaves the cursor where a decode loop reads truncation
///
/// # Returns
///
/// The bytes, or an error when fewer than `width` remain
fn take_or_drain<'i>(width: usize, input: &mut &'i [u8]) -> Result<&'i [u8]> {
    let Some((taken, rest)) = input.split_at_checked(width) else {
        input.finish();
        return Err(ContextError::from_input(input));
    };
    *input = rest;
    Ok(taken)
}
