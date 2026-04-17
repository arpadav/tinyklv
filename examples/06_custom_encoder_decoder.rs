#![allow(clippy::unwrap_used)]
//! Custom encoder/decoder for a scaled f64 field (fixed-point telemetry).
//!
//! Many protocols store floating-point measurements as scaled integers to save
//! bandwidth. This example encodes a `f64` longitude value as a big-endian i32
//! using the formula `wire = (value / SCALE) as i32`, and decodes back with
//! `value = wire as f64 * SCALE`. The custom `scale_lon_enc` and
//! `scale_lon_dec` functions are referenced via `enc =` / `dec =` field
//! attributes. A GPS fix struct combining this scaled longitude with a
//! similarly-scaled latitude demonstrates the pattern for multiple fields.

use tinyklv::prelude::*;
use tinyklv::Klv;

// Scale factor: i32 full-range covers ±180° with ~84 nano-degree resolution
const LON_SCALE: f64 = 180.0 / (i32::MAX as f64);
const LAT_SCALE: f64 = 90.0 / (i32::MAX as f64);

// --- Custom encoder: f64 → 4-byte big-endian i32 -------------------------
fn scale_lon_enc(v: &f64) -> Vec<u8> {
    // Divide by scale to get the wire integer, clamp to i32 range
    let wire = (*v / LON_SCALE) as i32;
    tinyklv::enc::binary::be_i32(wire)
}

fn scale_lat_enc(v: &f64) -> Vec<u8> {
    let wire = (*v / LAT_SCALE) as i32;
    tinyklv::enc::binary::be_i32(wire)
}

// --- Custom decoder: 4 bytes → i32 → f64 --------------------------------
fn scale_lon_dec(input: &mut &[u8]) -> tinyklv::Result<f64> {
    // Parse the raw i32 then multiply by the scale factor
    let wire = tinyklv::dec::binary::be_i32(input)?;
    Ok(wire as f64 * LON_SCALE)
}

fn scale_lat_dec(input: &mut &[u8]) -> tinyklv::Result<f64> {
    let wire = tinyklv::dec::binary::be_i32(input)?;
    Ok(wire as f64 * LAT_SCALE)
}

fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}

/// GPS fix with scaled-integer lat/lon and a u16 fix-quality indicator.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x47\x50",  // "GP" - GPS packet marker
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct GpsFix {
    // Key 0x01: latitude stored as scaled i32, decoded as f64 degrees
    #[klv(key = 0x01, dec = scale_lat_dec, enc = scale_lat_enc)]
    latitude_deg: f64,

    // Key 0x02: longitude stored as scaled i32, decoded as f64 degrees
    #[klv(key = 0x02, dec = scale_lon_dec, enc = scale_lon_enc)]
    longitude_deg: f64,

    // Key 0x03: HDOP * 100 as u16 (no custom codec needed)
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    hdop_centiunits: u16,
}

fn main() {
    let original = GpsFix {
        latitude_deg: 48.856_6, // Paris latitude
        longitude_deg: 2.352_2, // Paris longitude
        hdop_centiunits: 95,    // HDOP = 0.95 (excellent)
    };

    println!(
        "Original: lat={:.6}°, lon={:.6}°, hdop={}",
        original.latitude_deg, original.longitude_deg, original.hdop_centiunits
    );

    let frame = original.encode_frame();
    println!("Encoded frame ({} bytes): {:02X?}", frame.len(), frame);

    let decoded = GpsFix::decode_frame(&mut frame.as_slice()).unwrap();
    println!(
        "Decoded:  lat={:.6}°, lon={:.6}°, hdop={}",
        decoded.latitude_deg, decoded.longitude_deg, decoded.hdop_centiunits
    );

    // Floating-point roundtrip has quantisation error ~1e-7°, well below GPS accuracy
    assert!((decoded.latitude_deg - original.latitude_deg).abs() < 1e-6);
    assert!((decoded.longitude_deg - original.longitude_deg).abs() < 1e-6);
    assert_eq!(decoded.hdop_centiunits, original.hdop_centiunits);
    println!("SUCCESS");
}
