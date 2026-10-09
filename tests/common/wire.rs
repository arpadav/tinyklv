//! Protobuf's wire types: the low 3 bits of a tag, which say how the value after it is framed,
//! and [`framing`], which decodes that framing
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::varint;
use tinyklv::Result;
use tinyklv::framing::Framing;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Width of the wire type in a tag's low bits; the field number sits above it
pub(crate) const TYPE_BITS: u32 = 3;

/// Mask selecting the wire type from a tag's low [`TYPE_BITS`] bits
pub(crate) const TYPE_MASK: u32 = (1 << TYPE_BITS) - 1;

/// Wire type 0 VARINT: self-delimiting
pub(crate) const VARINT: u32 = 0;

/// Wire type 1 I64: fixed 8 bytes
pub(crate) const I64: u32 = 1;

/// Wire type 2 LEN: a varint length prefix, then that many bytes
pub(crate) const LEN: u32 = 2;

/// Wire type 5 I32: fixed 4 bytes
pub(crate) const I32: u32 = 5;

/// Decodes the framing of the value after a tag, from the wire type in its low 3 bits
///
/// * wire 0 VARINT -> [`Framing::Inline`] (self-delimiting, read without `take`)
/// * wire 1 I64    -> [`Framing::Sliced(8)`](Framing::Sliced)
/// * wire 2 LEN    -> [`Framing::Sliced(n)`](Framing::Sliced) after consuming the length prefix
/// * wire 5 I32    -> [`Framing::Sliced(4)`](Framing::Sliced)
///
/// # Arguments
///
/// * `tag` - the frame's key, whose low 3 bits are its wire type
/// * `input` - the byte stream positioned after the tag; advanced past a LEN value's length
///   prefix, left as [`varint::dec_usize`] leaves it on a failed one, and untouched otherwise
///
/// # Returns
///
/// `Ok(Some(framing))`, `Ok(None)` on a wire type protobuf does not frame (3, 4, 6 or 7), or the
/// error of [`varint::dec_usize`] on a LEN value's length prefix
pub(crate) fn framing(tag: u32, input: &mut &[u8]) -> Result<Option<Framing>> {
    let framing = match tag & TYPE_MASK {
        VARINT => Framing::Inline,
        I64 => Framing::Sliced(size_of::<u64>()),
        LEN => Framing::Sliced(varint::dec_usize(input)?),
        I32 => Framing::Sliced(size_of::<u32>()),
        _ => return Ok(None),
    };
    Ok(Some(framing))
}
