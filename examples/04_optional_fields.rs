#![allow(clippy::unwrap_used)]
//! Struct with Option<T> fields: one test with all fields present, one with
//! some omitted. Optional fields in KLV are modelled as `Option<T>` - when
//! `None`, no key-length-value triple is emitted on the wire, so the encoded
//! form is shorter. On decode, absent keys leave the field as `None`. This
//! example uses a drone telemetry struct with a mandatory GPS fix and optional
//! altitude and battery readings, asserting both the full and partial cases.

use tinyklv::prelude::*;
use tinyklv::Klv;

/// Drone telemetry frame. Latitude and longitude are always present;
/// altitude and battery level are optional (may not be fitted on all units).
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xD7\xE1",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct DroneTelemetry {
    // Mandatory: latitude in micro-degrees (i32)
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_i32, enc = &tinyklv::enc::binary::be_i32)]
    lat_udeg: i32,

    // Mandatory: longitude in micro-degrees (i32)
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_i32, enc = &tinyklv::enc::binary::be_i32)]
    lon_udeg: i32,

    // Optional: altitude above sea level in centimetres
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_i32, enc = &tinyklv::enc::binary::be_i32)]
    altitude_cm: Option<i32>,

    // Optional: battery percentage 0-100
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_u8, enc = &tinyklv::enc::binary::u8)]
    battery_pct: Option<u8>,

    // Optional: heading in 0.01° units (u16)
    #[klv(key = 0x05, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    heading_centideg: Option<u16>,
}

fn main() {
    // --- Case 1: all fields present ----------------------------------------
    let full = DroneTelemetry {
        lat_udeg: 51_477_200,      // ~51.477° N (London)
        lon_udeg: -126_772,        // ~0.127° W
        altitude_cm: Some(10_200), // 102 m
        battery_pct: Some(87),
        heading_centideg: Some(27_000), // 270.00°
    };

    let enc_full = full.encode_frame();
    println!("Full frame  ({} bytes): {:02X?}", enc_full.len(), enc_full);

    let dec_full = DroneTelemetry::decode_frame(&mut enc_full.as_slice()).unwrap();
    println!(
        "Full decoded: lat={}, lon={}, alt={:?}, bat={:?}, hdg={:?}",
        dec_full.lat_udeg,
        dec_full.lon_udeg,
        dec_full.altitude_cm,
        dec_full.battery_pct,
        dec_full.heading_centideg
    );

    // Every field must survive the roundtrip
    assert_eq!(dec_full, full);

    // --- Case 2: optional fields omitted ------------------------------------
    let partial = DroneTelemetry {
        lat_udeg: 37_774_900, // ~37.775° N (San Francisco)
        lon_udeg: -122_419_400,
        altitude_cm: None, // sensor not fitted
        battery_pct: Some(42),
        heading_centideg: None,
    };

    let enc_partial = partial.encode_frame();
    println!(
        "Partial frame ({} bytes): {:02X?}",
        enc_partial.len(),
        enc_partial
    );

    // Partial frame must be shorter than the full frame
    assert!(
        enc_partial.len() < enc_full.len(),
        "absent fields should shrink the frame"
    );

    let dec_partial = DroneTelemetry::decode_frame(&mut enc_partial.as_slice()).unwrap();
    println!(
        "Partial decoded: lat={}, lon={}, alt={:?}, bat={:?}",
        dec_partial.lat_udeg,
        dec_partial.lon_udeg,
        dec_partial.altitude_cm,
        dec_partial.battery_pct
    );

    assert_eq!(dec_partial, partial);
    println!("SUCCESS");
}
