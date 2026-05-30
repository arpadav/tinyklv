#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 04 - optional fields.
//!
//! `Option<T>` on a struct field makes the corresponding KLV triple optional
//! in the stream. On encode, `None` emits nothing; on decode, missing keys
//! leave the field as `None` without raising an error. This lets a single
//! struct describe heterogeneous telemetry from units with different sensor
//! sets.
//!
//! Showcases:
//! * `Option<T>` field wrapping around the same codecs as `T`
//! * Frame shrinkage when optional fields are absent
//! * Decode tolerating missing keys
//!
//! See also: book Tutorial 04.
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"DRONETELEM",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Drone telemetry frame. Lat/lon are always present; altitude, battery, and
/// heading are optional (they may not be fitted on every airframe)
struct DroneTelemetry {
    #[klv(
        key = 0x01,
        dec = decb::be_i32,
        enc = *encb::be_i32,
    )]
    /// Mandatory latitude in micro-degrees (i32)
    lat_udeg: i32,

    #[klv(
        key = 0x02,
        dec = decb::be_i32,
        enc = *encb::be_i32,
    )]
    /// Mandatory longitude in micro-degrees (i32)
    lon_udeg: i32,

    #[klv(
        key = 0x03,
        dec = decb::be_i32,
        enc = *encb::be_i32,
    )]
    /// Optional altitude above sea level in centimetres
    altitude_cm: Option<i32>,

    #[klv(
        key = 0x04,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Optional battery percentage, 0..=100
    battery_pct: Option<u8>,

    #[klv(
        key = 0x05,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Optional heading in 0.01 degree units (u16)
    heading_centideg: Option<u16>,
}

fn main() {
    // build - a fully-instrumented airframe with every optional present
    let full = DroneTelemetry {
        lat_udeg:         51_477_200,     // ~51.477 N (London)
        lon_udeg:         -126_772,       // ~0.127 W
        altitude_cm:      Some(10_200),   // 102 m
        battery_pct:      Some(87),
        heading_centideg: Some(27_000),   // 270.00 deg
    };

    // encode + decode - every optional appears in the frame
    let mut enc_full = Vec::new();
    full.encode_frame(&mut enc_full);
    let dec_full = DroneTelemetry::decode_frame(
        &mut enc_full.as_slice(),
    ).unwrap();
    assert_eq!(dec_full, full);

    // build - a stripped-down airframe with some sensors absent
    let partial = DroneTelemetry {
        lat_udeg:         37_774_900,     // ~37.775 N (San Francisco)
        lon_udeg:         -122_419_400,
        altitude_cm:      None,           // altimeter not fitted
        battery_pct:      Some(42),
        heading_centideg: None,           // compass not fitted
    };

    // encode - absent fields emit zero bytes, so the partial frame is shorter
    let mut enc_partial = Vec::new();
    partial.encode_frame(&mut enc_partial);
    assert!(
        enc_partial.len() < enc_full.len(),
        "absent fields must shrink the frame",
    );

    // decode - missing keys stay as None without error
    let dec_partial = DroneTelemetry::decode_frame(
        &mut enc_partial.as_slice(),
    ).unwrap();

    // assert - partial frame round-trips
    assert_eq!(dec_partial, partial);
}
