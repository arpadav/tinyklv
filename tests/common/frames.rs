//! The frames of one proto3 message, `int32 id = 1`, `float ratio = 2` and `string name = 3`, at
//! `id = 5`, `ratio = 1.5` and `name = "hi"`, and of the fields 4 to 7 it does not declare, one
//! per wire type protobuf frames
//!
//! Key + len and keylen mode frame the message to the same bytes, so the framing and keylen
//! suites both build their wires by concatenating these
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::unknown::Unknown;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// id: tag, varint 5, no length
pub(crate) const ID: &[u8] = &[0x08, 0x05];

/// ratio: tag, 1.5f32 little-endian, no length
pub(crate) const RATIO: &[u8] = &[0x15, 0x00, 0x00, 0xC0, 0x3F];

/// name: tag, length 2, "hi"
pub(crate) const NAME: &[u8] = &[0x1A, 0x02, b'h', b'i'];

/// unknown field 4, wire type 0 VARINT: tag, varint 300
pub(crate) const UNKNOWN_VARINT: &[u8] = &[0x20, 0xAC, 0x02];

/// unknown field 5, wire type 1 I64: tag, 9u64 little-endian
pub(crate) const UNKNOWN_I64: &[u8] = &[0x29, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

/// unknown field 6, wire type 2 LEN: tag, length 3, three bytes
pub(crate) const UNKNOWN_LEN: &[u8] = &[0x32, 0x03, 0xAA, 0xBB, 0xCC];

/// unknown field 7, wire type 5 I32: tag, 7u32 little-endian
pub(crate) const UNKNOWN_I32: &[u8] = &[0x3D, 0x07, 0x00, 0x00, 0x00];

/// The `#[klv(unknown)]` entry [`UNKNOWN_VARINT`] decodes to
pub(crate) fn varint_entry() -> (u32, Unknown) {
    (0x20, Unknown::Varint(300))
}

/// The `#[klv(unknown)]` entry [`UNKNOWN_I64`] decodes to
pub(crate) fn i64_entry() -> (u32, Unknown) {
    (0x29, Unknown::I64(9))
}

/// The `#[klv(unknown)]` entry [`UNKNOWN_LEN`] decodes to
pub(crate) fn len_entry() -> (u32, Unknown) {
    (0x32, Unknown::Len(vec![0xAA, 0xBB, 0xCC]))
}

/// The `#[klv(unknown)]` entry [`UNKNOWN_I32`] decodes to
pub(crate) fn i32_entry() -> (u32, Unknown) {
    (0x3D, Unknown::I32(7))
}
