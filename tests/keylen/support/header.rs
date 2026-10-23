//! Hand-written frame header codecs for the fused-`keylen` tests: protobuf's, and a bare `u8`-key
//! header
//!
//! These are ordinary tinyklv codecs that live entirely test-side: tinyklv never
//! learns the word "protobuf". They mirror the protobuf wire format - a tag
//! varint whose low 3 bits carry the wire type - so the fused `keylen` codec can
//! return `(tag, Framing)` and drive both `Sliced` and `Inline` value framing;
//! [`dec_field`] returns the field number in place of the tag. The
//! `enc_*` encoders are the inverses the fused encode path writes with. [`dec_u8`] and
//! [`enc_u8`] are the bare header of the tests that need no protobuf: a `u8` key, every
//! value `Inline`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::{varint, wire};
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

/// Decodes one protobuf frame header: the tag varint (the key), then the framing
///
/// The framing is [`wire::framing`] of the wire type in the tag's low 3 bits. This is the
/// entire protobuf-specific delta - the rest is ordinary value decoding
///
/// # Arguments
///
/// * `input` - the byte stream positioned at a frame header; advanced past the header on success
///
/// # Returns
///
/// `Ok((tag, framing))`, or an error on truncation or an unsupported wire type
pub(crate) fn dec_tag(input: &mut &[u8]) -> Result<(u32, Framing)> {
    let start = input.checkpoint();
    let tag = varint::dec_u32(input)?;
    let Some(framing) = wire::framing(tag, input)? else {
        input.reset(&start);
        return Err(ContextError::from_input(input));
    };
    Ok((tag, framing))
}

/// Decodes one protobuf frame header keyed by field number: [`dec_tag`] with the wire type
/// shifted off the tag, so one key frames `Sliced` or `Inline` by the wire type it arrives with
///
/// # Arguments
///
/// * `input` - the byte stream positioned at a frame header; advanced past the header on success
///
/// # Returns
///
/// `Ok((field_number, framing))`, or the error of [`dec_tag`]
pub(crate) fn dec_field(input: &mut &[u8]) -> Result<(u32, Framing)> {
    dec_tag(input).map(|(tag, framing)| (tag >> wire::TYPE_BITS, framing))
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
pub(crate) fn dec_u8(input: &mut &[u8]) -> Result<(u8, Framing)> {
    Ok((decb::u8(input)?, Framing::Inline))
}

/// Encodes a frame header, the inverse of [`dec_u8`]: the `u8` key alone, since every
/// value is `Inline` and its length never reaches the wire
///
/// # Arguments
///
/// * `(key, _)` - the field key and the (unwritten) byte length of the value that follows
/// * `out` - the buffer the key byte is appended to
pub(crate) fn enc_u8((key, _): (u8, usize), out: &mut Vec<u8>) {
    out.push(key);
}

/// Encodes one protobuf frame header, the inverse of [`dec_tag`] for wire types 0/1/2/5
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
pub(crate) fn enc_tag((tag, value_len): (u32, usize), out: &mut Vec<u8>) {
    let wire_type = tag & wire::TYPE_MASK;
    debug_assert!(
        matches!(wire_type, wire::VARINT | wire::I64 | wire::LEN | wire::I32),
        "unsupported wire type {wire_type} in tag {tag}"
    );
    varint::enc_u32(tag, out);
    if wire_type == wire::LEN {
        varint::enc_usize(value_len, out);
    }
}

/// Encodes one protobuf frame header keyed by field number, the inverse of
/// [`dec_field`]: [`enc_tag`] with the tag built from the field number and the wire
/// type the field is sent as
///
/// # Arguments
///
/// * `wire_type` - the wire type the field is sent as
/// * `(field, value_len)` - the field number and the byte length of the value that follows
/// * `out` - the buffer the header bytes are appended to
pub(crate) fn enc_field(wire_type: u32, (field, value_len): (u32, usize), out: &mut Vec<u8>) {
    enc_tag(((field << wire::TYPE_BITS) | wire_type, value_len), out);
}
