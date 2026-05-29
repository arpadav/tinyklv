//! Sub-types shared across the benchmark records.
//!
//! [`Reading`] is the fixed-width sensor record packed into [`super::Compound`]'s sensor run; its
//! byte-packing is shared by serde_klv, tlv_parser, and the manual approach, so it lives here as
//! inherent methods rather than in any one approach. [`GpsCoord`] is the nested coordinate
//! sub-packet used by both [`super::Compound`] and [`super::Rich`].
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::{dec::binary as decb, enc::binary as encb, prelude::*};

#[derive(Debug, PartialEq, Clone, Copy)]
/// A fixed 5-byte raw record: a `kind` tag plus a big-endian `f32`. Packed back-to-back
/// into the length-delimited `sensors` field; the byte-packing is shared by serde_klv,
/// tlv_parser, and the manual approach.
pub(crate) struct Reading {
    pub(crate) kind: u8,
    pub(crate) value: f32,
}

/// [`Reading`] implementation
impl Reading {
    /// Fixed wire width of one reading: `kind` (1 byte) + `value` (4 bytes big-endian `f32`)
    pub(crate) const WIDTH: usize = 5;

    /// Serialises a slice of readings into a packed byte run
    ///
    /// Each reading occupies exactly [`Reading::WIDTH`] bytes: the `kind` tag
    /// followed by the `value` as a big-endian `f32`
    ///
    /// # Arguments
    ///
    /// * `readings` - the slice of readings to pack; may be empty
    ///
    /// # Returns
    ///
    /// A `Vec<u8>` of length `readings.len() * Reading::WIDTH`
    pub(crate) fn pack(readings: &[Reading]) -> Vec<u8> {
        // --------------------------------------------------
        // allocate output buffer
        // --------------------------------------------------
        let mut out = Vec::with_capacity(readings.len() * Reading::WIDTH);
        // --------------------------------------------------
        // pack each reading as kind + big-endian f32
        // --------------------------------------------------
        for r in readings {
            out.push(r.kind);
            out.extend_from_slice(&r.value.to_be_bytes());
        }
        out
    }

    /// Deserialises a packed byte run back into a `Vec<Reading>`
    ///
    /// Splits `bytes` into [`Reading::WIDTH`]-byte chunks and converts each one
    /// into a `Reading`. The `kind` is the first byte; `value` is the next four
    /// bytes interpreted as a big-endian `f32`
    ///
    /// # Arguments
    ///
    /// * `bytes` - the packed byte buffer to unpack
    ///
    /// # Returns
    ///
    /// `Some(Vec<Reading>)` when `bytes.len()` is an exact multiple of
    /// [`Reading::WIDTH`], or `None` if there is a trailing partial record
    pub(crate) fn unpack(bytes: &[u8]) -> Option<Vec<Reading>> {
        // --------------------------------------------------
        // split into fixed-width chunks and convert each to a reading
        // --------------------------------------------------
        let mut chunks = bytes.chunks_exact(Reading::WIDTH);
        let out = chunks
            .by_ref()
            .map(|c| Reading {
                kind: c[0],
                value: f32::from_be_bytes([c[1], c[2], c[3], c[4]]),
            })
            .collect();
        // --------------------------------------------------
        // reject trailing partial record
        // --------------------------------------------------
        chunks.remainder().is_empty().then_some(out)
    }
}

/// [`Reading`] implementation of [`tinyklv::DecodeValue`] for [`&[u8]`]
impl tinyklv::DecodeValue<&[u8]> for Reading {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        // --------------------------------------------------
        // read kind tag and value field
        // --------------------------------------------------
        let kind = decb::u8(input)?;
        let value = decb::be_f32(input)?;
        // --------------------------------------------------
        // construct reading
        // --------------------------------------------------
        Ok(Reading { kind, value })
    }
}

/// Nested coordinate sub-packet.
#[derive(tinyklv::Klv, Debug, PartialEq, Clone, Copy)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
pub(crate) struct GpsCoord {
    #[klv(key = 0x01)]
    pub(crate) lat: f64,

    #[klv(key = 0x02)]
    pub(crate) lon: f64,
}
