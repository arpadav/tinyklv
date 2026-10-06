//! The canonical protobuf-shaped message shared by the fused-`keylen` test files
//!
//! [`Example`] mixes every wire shape: a wire-0 VARINT (`Inline`), a wire-5 I32
//! and wire-1 I64 (`Sliced` fixed widths, decoded by the crate's little-endian
//! codecs), a wire-2 LEN string, a nested LEN message decoded on its sub-slice, a
//! packed repeated, a custom-enum VARINT field, and a field whose tag and length
//! prefix are both two-byte varints. [`Example::wire`] is its hand-built encoding (the
//! concatenation of [`Example::frames`]), [`Example::decoded`] the value it decodes to. [`Reading`]
//! is the small message the framing and unknown-key tests share
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::pb;
use tinyklv::codecs::ber::BerLength;
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::enc::ber as encber;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

/// A protobuf-style `enum` field: a single VARINT value mapped to a unit variant
///
/// Unlike a oneof, a proto `enum` is one ordinary VARINT-valued field whose value
/// codec maps the decoded integer to a unit variant - decoded here by [`Status::decode_value`]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Status {
    /// Wire value `0`
    Idle,
    /// Wire value `1`
    Active,
    /// Wire value `2`
    Error,
}

/// [`Status`] implementation
impl Status {
    /// Every variant, in discriminant order
    pub(crate) const ALL: [Self; 3] = [Self::Idle, Self::Active, Self::Error];
}

/// [`Status`] implementation of [`TryFrom<u64>`]: a decoded VARINT to its variant
impl TryFrom<u64> for Status {
    /// The rejected discriminant
    type Error = u64;

    /// # Arguments
    ///
    /// * `value` - the decoded VARINT
    ///
    /// # Returns
    ///
    /// `Ok(Status)`, or `Err(value)` for an unknown discriminant
    fn try_from(value: u64) -> Result<Self, <Self as TryFrom<u64>>::Error> {
        match value {
            0 => Ok(Self::Idle),
            1 => Ok(Self::Active),
            2 => Ok(Self::Error),
            unknown => Err(unknown),
        }
    }
}

/// `u64` implementation of [`From<Status>`]: a variant to its VARINT, the inverse of [`TryFrom<u64>`]
impl From<Status> for u64 {
    /// # Arguments
    ///
    /// * `value` - the variant to encode
    ///
    /// # Returns
    ///
    /// The variant's wire discriminant
    fn from(value: Status) -> Self {
        match value {
            Status::Idle => 0,
            Status::Active => 1,
            Status::Error => 2,
        }
    }
}

/// [`Status`] implementation of [`DecodeValue`]: a wire-0 VARINT (inline-framed custom enum)
impl DecodeValue<&[u8]> for Status {
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at the value; advanced past the varint on success
    ///
    /// # Returns
    ///
    /// `Ok(Status)`, or an error on truncation or an unknown discriminant; an unknown
    /// discriminant rewinds the cursor to the start of the varint
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        pb::dec_varint_u64
            .verify_map(|value| Self::try_from(value).ok())
            .parse_next(input)
    }
}

/// [`Status`] implementation of [`EncodeValue`]: the variant's discriminant as a wire-0 VARINT
impl EncodeValue for Status {
    /// # Arguments
    ///
    /// * `out` - the buffer the varint bytes are appended to
    fn encode_value(&self, out: &mut Vec<u8>) {
        pb::enc_varint_u64(u64::from(*self), out);
    }
}

/// Nested proto3 message - decoded from the LEN sub-slice of `Example.inner`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen, enc = pb::enc_tag_keylen),
    default(typ = i32, dec = pb::dec_varint_i32, enc = *pb::enc_varint_i32),
)]
pub(crate) struct Inner {
    /// field 1, wire 0 VARINT - decoded `Inline`
    #[klv(key = 0x08)]
    pub(crate) code: i32,
}

/// proto3 message exercising every wire shape through the fused `keylen` codec
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen, enc = pb::enc_tag_keylen),
    default(typ = i32, dec = pb::dec_varint_i32, enc = *pb::enc_varint_i32),
    default(typ = i64, dec = decb::le_i64, enc = *encb::le_i64),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32),
    default(typ = String, dec = pb::dec_string, enc = &encs::from_string_utf8),
    default(typ = Vec<i32>, dec = pb::dec_packed_i32, enc = &pb::enc_packed_i32),
)]
pub(crate) struct Example {
    /// field 1, wire 0 VARINT - `Inline`, read without `take`
    #[klv(key = 0x08)]
    pub(crate) id: i32,

    /// field 2, wire 5 I32 - `Sliced(4)`, fixed 4 bytes, no length on the wire
    #[klv(key = 0x15)]
    pub(crate) ratio: f32,

    /// field 3, wire 2 LEN - `Sliced(n)`, the only shape that carries a length prefix
    #[klv(key = 0x1A)]
    pub(crate) name: String,

    /// field 4, wire 2 LEN - nested message; its own `decode_value` runs on the sub-slice
    #[klv(key = 0x22, dec = Inner::decode_value, enc = Inner::encode_value)]
    pub(crate) inner: Inner,

    /// field 5, wire 2 LEN - packed repeated; the `Vec<i32>` codec loops to sub-slice eof
    #[klv(key = 0x2A)]
    pub(crate) samples: Vec<i32>,

    /// field 6, wire 1 I64 - `Sliced(8)`, fixed 8 bytes, no length on the wire
    #[klv(key = 0x31)]
    pub(crate) created_at: i64,

    /// field 7, wire 0 VARINT - inline-framed custom enum
    #[klv(key = 0x38, dec = Status::decode_value, enc = Status::encode_value)]
    pub(crate) status: Status,

    /// field 16, wire 2 LEN - tag 130 and length 128, both two-byte varints
    #[klv(key = 0x82)]
    pub(crate) note: String,
}

/// [`Example`] implementation
impl Example {
    /// Byte length of `note`, the smallest that needs a two-byte LEN prefix
    const NOTE_LEN: usize = 128;

    /// The frames of [`Example::wire`], one per field in declaration order
    #[rustfmt::skip]
    pub(crate) fn frames() -> Vec<Vec<u8>> {
        vec![
            // id = 5 (field 1, wire 0 VARINT, Inline)
            vec![0x08, 0x05],
            // ratio = 1.5 (field 2, wire 5 I32, Sliced(4)) - 1.5f32 little-endian
            vec![0x15, 0x00, 0x00, 0xC0, 0x3F],
            // name = "hi" (field 3, wire 2 LEN, Sliced(2))
            vec![0x1A, 0x02, 0x68, 0x69],
            // inner = { code = 7 } (field 4, wire 2 LEN, Sliced(2)) - nested 0x08 0x07
            vec![0x22, 0x02, 0x08, 0x07],
            // samples = [1, 2, 3] (field 5, wire 2 LEN, Sliced(3)) - packed varints
            vec![0x2A, 0x03, 0x01, 0x02, 0x03],
            // created_at = 9 (field 6, wire 1 I64, Sliced(8)) - i64 little-endian
            vec![0x31, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            // status = Active (field 7, wire 0 VARINT, Inline)
            vec![0x38, 0x01],
            Self::note_frame(),
        ]
    }

    /// The full wire encoding of [`Example::decoded`]
    pub(crate) fn wire() -> Vec<u8> {
        Self::frames().concat()
    }

    /// The `note` frame that ends [`Example::frames`]
    #[rustfmt::skip]
    fn note_frame() -> Vec<u8> {
        let mut frame = vec![
            // tag 130 (field 16, wire 2 LEN)
            0x82, 0x01,
            // length 128
            0x80, 0x01,
        ];
        frame.extend(Self::note().bytes());
        frame
    }

    /// [`Example::wire`] with `id = 300`, a two-byte inline varint
    pub(crate) fn wire_id_300() -> Vec<u8> {
        let mut wire = Self::wire();
        // --------------------------------------------------
        // the id varint 0x05 becomes 0xAC 0x02
        // --------------------------------------------------
        wire.splice(1..2, [0xAC, 0x02]);
        wire
    }

    /// The canonical decoded [`Example`] matching [`Example::wire`]
    pub(crate) fn decoded() -> Self {
        Self {
            id: 5,
            ratio: 1.5,
            name: "hi".to_owned(),
            inner: Inner { code: 7 },
            samples: vec![1, 2, 3],
            created_at: 9,
            status: Status::Active,
            note: Self::note(),
        }
    }

    /// The decoded `note`: [`Example::NOTE_LEN`] bytes of text
    fn note() -> String {
        "n".repeat(Self::NOTE_LEN)
    }
}

/// Small message mixing a required `Inline` field with optional `Sliced` and `Inline` ones
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen, enc = pb::enc_tag_keylen),
    default(typ = i32, dec = pb::dec_varint_i32, enc = *pb::enc_varint_i32),
    default(typ = String, dec = pb::dec_string, enc = &encs::from_string_utf8),
)]
pub(crate) struct Reading {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    pub(crate) count: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x12)]
    pub(crate) label: Option<String>,

    /// field 3, wire 0 VARINT - `Inline` custom enum
    #[klv(key = 0x18, dec = Status::decode_value, enc = Status::encode_value)]
    pub(crate) status: Option<Status>,

    /// field 4, wire 0 - `Inline` BER-OID, malformed past `u8::MAX`
    #[klv(key = 0x20, dec = decber::ber_oid::<u8>, enc = *encber::ber_oid)]
    pub(crate) level: Option<u8>,

    /// field 5, wire 0 - `Inline` BER length, malformed past `u8::MAX`
    #[klv(key = 0x28, dec = BerLength::<u8>::decode_value, enc = EncodeValue::encode_value)]
    pub(crate) size: Option<BerLength<u8>>,
}

/// [`Reading`] implementation
impl Reading {
    /// `count = 3` and nothing else: what every cut or stopped test wire decodes to
    pub(crate) fn count_only() -> Self {
        Self {
            count: 3,
            label: None,
            status: None,
            level: None,
            size: None,
        }
    }
}
