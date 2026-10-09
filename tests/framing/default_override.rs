//! A container `default(typ = T, ..)` fills only what a field of `T` leaves unset
//!
//! A field with its own `dec`, `enc` or `size(exact = N)` keeps it and takes the rest from the
//! default: its own decoder reads the frame, its own encoder writes it, and its own width is the
//! one a frame's length is checked against, in both drivers and at every chunk split. A field's
//! own `reserve` replaces the default's `exact`, and a default written as two entries for `T`
//! fills as one
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::combinator::repeat;

/// A run of 16-bit samples, as many as the frame holds: little-endian by default, big-endian
/// where a field says so
#[derive(Debug, Clone, PartialEq)]
struct Samples(Vec<i16>);

/// [`Samples`] implementation
impl Samples {
    /// Decodes big-endian samples to the end of the input
    ///
    /// # Arguments
    ///
    /// * `input` - the frame's value bytes; consumed to their last whole sample
    ///
    /// # Returns
    ///
    /// The samples
    fn decode_be(input: &mut &[u8]) -> tinyklv::Result<Self> {
        repeat(0.., decb::be_i16).map(Self).parse_next(input)
    }

    /// Encodes the samples big-endian, the inverse of [`Samples::decode_be`]
    ///
    /// # Arguments
    ///
    /// * `out` - the buffer the samples are appended to
    fn encode_be(&self, out: &mut Vec<u8>) {
        self.0.iter().for_each(|&sample| encb::be_i16(sample, out));
    }
}

/// [`Samples`] implementation of [`DecodeValue`]: little-endian samples to the end of the input
impl DecodeValue<&[u8]> for Samples {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        repeat(0.., decb::le_i16).map(Self).parse_next(input)
    }
}

/// [`Samples`] implementation of [`EncodeValue`]: the samples little-endian
impl EncodeValue for Samples {
    fn encode_value(&self, out: &mut Vec<u8>) {
        self.0.iter().for_each(|&sample| encb::le_i16(sample, out));
    }
}

/// A seismograph's frame whose channels each override one setting of the `Samples` default, so a
/// test can tell whose decoder, encoder or width ran
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(
        typ = Samples,
        dec = Samples::decode_value,
        enc = Samples::encode_value,
        size(exact = 4),
    ),
)]
struct Seismogram {
    /// field 1 - everything from the default: two little-endian samples
    #[klv(key = 0x0A)]
    north: Samples,

    /// field 2 - its own `dec` reads big-endian; `enc` and `exact` from the default
    #[klv(key = 0x12, dec = Samples::decode_be)]
    east: Samples,

    /// field 3 - its own `enc` writes big-endian; `dec` and `exact` from the default
    #[klv(key = 0x1A, enc = Samples::encode_be)]
    up: Samples,

    /// field 4 - optional, its own `size(exact = 6)`: three samples; `dec` and `enc` from the
    /// default
    #[klv(key = 0x22, size(exact = 6))]
    burst: Option<Samples>,
}

/// [`Seismogram`] implementation
impl Seismogram {
    /// north = [1, -2] (field 1, 4 bytes) - little-endian
    const NORTH: &[u8] = &[0x0A, 0x04, 0x01, 0x00, 0xFE, 0xFF];

    /// east = [3, -4] (field 2, 4 bytes) - big-endian, as its own `dec` reads it
    const EAST: &[u8] = &[0x12, 0x04, 0x00, 0x03, 0xFF, 0xFC];

    /// up = [5, -6] (field 3, 4 bytes) - little-endian, as the default's `dec` reads it
    const UP: &[u8] = &[0x1A, 0x04, 0x05, 0x00, 0xFA, 0xFF];

    /// burst = [7, 8, -9] (field 4, 6 bytes) - little-endian
    #[rustfmt::skip]
    const BURST: &[u8] = &[
        0x22, 0x06,
        0x07, 0x00, 0x08, 0x00, 0xF7, 0xFF,
    ];

    /// The wire [`Seismogram::decoded`] decodes from
    fn wire() -> Vec<u8> {
        [Self::NORTH, Self::EAST, Self::UP, Self::BURST].concat()
    }

    /// The canonical decoded [`Seismogram`] matching [`Seismogram::wire`]
    fn decoded() -> Self {
        Self {
            north: Samples(vec![1, -2]),
            east: Samples(vec![3, -4]),
            up: Samples(vec![5, -6]),
            burst: Some(Samples(vec![7, 8, -9])),
        }
    }
}

/// A field's own `dec` reads its frame; a field with only its own `enc` or `exact` reads through
/// the default's
#[test]
fn each_field_decodes_with_its_own_dec_or_the_defaults() {
    Seismogram::decoded().assert_decodes_from(&Seismogram::wire());
}

/// A field's own `enc` writes its frame; a field with only its own `dec` or `exact` writes
/// through the default's
#[test]
fn each_field_encodes_with_its_own_enc_or_the_defaults() {
    #[rustfmt::skip]
    let expected = [
        Seismogram::NORTH,
        // east = [3, -4] (field 2, 4 bytes) - little-endian, as the default's `enc` writes it
        &[0x12, 0x04, 0x03, 0x00, 0xFC, 0xFF],
        // up = [5, -6] (field 3, 4 bytes) - big-endian, as its own `enc` writes it
        &[0x1A, 0x04, 0x00, 0x05, 0xFF, 0xFA],
        Seismogram::BURST,
    ]
    .concat();
    assert_eq!(Seismogram::decoded().encoded(), expected);
}

/// A field's own `exact` is the width its frame is checked against: the default's width is
/// rejected for it
#[test]
fn a_fields_own_exact_replaces_the_defaults() {
    #[rustfmt::skip]
    let wire = [
        Seismogram::NORTH,
        Seismogram::EAST,
        Seismogram::UP,
        // burst = [7, 8] (field 4, 4 bytes)
        &[0x22, 0x04, 0x07, 0x00, 0x08, 0x00],
    ]
    .concat();
    Seismogram::assert_rejects_naming(&wire, &["`Seismogram::burst`", "size(exact = 6)"]);
}

/// A field with its own `dec` or `enc` but no width of its own is still checked against the
/// default's
#[test]
fn a_field_without_its_own_exact_takes_the_defaults() {
    let width = "size(exact = 4)";
    #[rustfmt::skip]
    let east = [
        Seismogram::NORTH,
        // east = [3, -4, 5] (field 2, 6 bytes)
        &[0x12, 0x06, 0x00, 0x03, 0xFF, 0xFC, 0x00, 0x05],
    ]
    .concat();
    Seismogram::assert_rejects_naming(&east, &["`Seismogram::east`", width]);
    #[rustfmt::skip]
    let up = [
        Seismogram::NORTH,
        Seismogram::EAST,
        // up = [5] (field 3, 2 bytes)
        &[0x1A, 0x02, 0x05, 0x00],
    ]
    .concat();
    Seismogram::assert_rejects_naming(&up, &["`Seismogram::up`", width]);
}

/// A seismograph's frame whose one channel reserves its bytes under the `Samples` default's width
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(
        typ = Samples,
        dec = Samples::decode_value,
        enc = Samples::encode_value,
        size(exact = 4),
    ),
)]
struct ReservedSeismogram {
    /// field 1 - its own `size(reserve = 8)`: any number of samples
    #[klv(key = 0x0A, size(reserve = 8))]
    north: Samples,
}

/// A field's own `reserve` replaces the default's `exact`, so its frame is not checked against
/// the default's width
#[test]
fn a_fields_own_reserve_replaces_the_defaults_exact() {
    // north = [1, -2, 3] (field 1, 6 bytes)
    let wire = [0x0A, 0x06, 0x01, 0x00, 0xFE, 0xFF, 0x03, 0x00];
    ReservedSeismogram {
        north: Samples(vec![1, -2, 3]),
    }
    .assert_decodes_from(&wire);
}

/// A seismograph's frame whose `Samples` default is written as two entries, its width in the
/// second
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    default(typ = Samples, enc = Samples::encode_value),
    default(typ = Samples, dec = Samples::decode_value, size(exact = 4)),
)]
struct SplitSeismogram {
    /// field 1 - two little-endian samples
    #[klv(key = 0x0A)]
    north: Samples,
}

/// A second `default(..)` entry for a type adds its width to the first entry's settings
#[test]
fn a_second_default_entry_adds_its_width() {
    // north = [1, -2, 3] (field 1, 6 bytes)
    let north = [0x0A, 0x06, 0x01, 0x00, 0xFE, 0xFF, 0x03, 0x00];
    SplitSeismogram::assert_rejects_naming(
        &north,
        &["`SplitSeismogram::north`", "size(exact = 4)"],
    );
}
