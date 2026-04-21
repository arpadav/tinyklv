#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 05 - encode/decode symmetry.
//!
//! `#[derive(Klv)]` generates matched pairs of traits on both sides of the
//! wire: `EncodeValue`/`DecodeValue` for the raw KLV triples and
//! `EncodeFrame`/`DecodeFrame` for the sentinel-wrapped envelope. This
//! example walks through both pairs on a single struct and asserts the
//! invariants that tie them together:
//!
//! * `decode_value(encode_value(x)) == x`
//! * `decode_frame(encode_frame(x)) == x`
//! * `encode_frame(x).len() > encode_value(x).len()` (the frame adds
//!   sentinel + outer-length bytes around the value region)
//!
//! Showcases:
//! * `encode_value` vs `encode_frame`
//! * `decode_value` vs `decode_frame`
//! * Sentinel + outer-length accounting
//!
//! See also: book Tutorial 05.
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],                                              // default, shown for clarity
    sentinel = b"ATMOSAMPLE",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Two-field atmospheric sample: pressure in Pascal + temperature in 0.01 C
struct AtmoSample {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    /// Barometric pressure in Pascal (big-endian u32)
    pressure_pa: u32,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Temperature in 0.01 C units (big-endian u16)
    temperature_centideg: u16,
}

fn main() {
    // build
    let original = AtmoSample {
        pressure_pa:          101_325, // standard atmosphere
        temperature_centideg: 2_050,   // 20.50 C
    };

    // encode - value bytes contain only the field KLV triples
    let value_bytes = original.encode_value();

    // encode - frame bytes wrap the value region in sentinel + outer length
    let frame_bytes = original.encode_frame();

    // the framed form must be strictly larger (sentinel + outer length)
    assert!(
        frame_bytes.len() > value_bytes.len(),
        "frame must wrap the value region with sentinel + length",
    );

    // the framed form must begin with the sentinel
    assert_eq!(
        &frame_bytes[0..b"ATMOSAMPLE".len()],
        b"ATMOSAMPLE",
        "frame must begin with the sentinel bytes",
    );

    // decode - value form round-trips
    let from_value = AtmoSample::decode_value(
        &mut value_bytes.as_slice(),
    ).unwrap();

    // decode - frame form round-trips
    let from_frame = AtmoSample::decode_frame(
        &mut frame_bytes.as_slice(),
    ).unwrap();

    // assert - both decode paths reconstruct the original
    assert_eq!(from_value, original);
    assert_eq!(from_frame, original);
}
