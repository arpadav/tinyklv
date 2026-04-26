#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 06 - hand-written encoder / decoder functions.
//!
//! The built-in `dec::binary` / `enc::binary` helpers cover integers and
//! floats in big/little endian. Whenever a field needs a domain-specific
//! representation - fixed-point quantisation, unit conversion, bit-packing -
//! you write a matched pair of free functions with the conventional
//! signatures:
//!
//! * Decoder: `fn(&mut Stream) -> tinyklv::Result<T>`
//! * Encoder: `fn(&T) -> Vec<u8>` (or `fn(T) -> Vec<u8>` for Copy types)
//!
//! The `dec =` and `enc =` field attributes then reference them by path,
//! exactly as they would reference a built-in. This example encodes GPS
//! lat/lon as scaled `i32` values to save bandwidth while keeping the
//! in-memory type as `f64`.
//!
//! Showcases:
//! * Custom decode/encode fns with free-function signatures
//! * Mixing hand-written codecs with built-in `be_u16`
//! * Fixed-point (scaled integer) quantisation
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

/// Scale factor: i32 full range covers +/- 180 degrees with ~84 nano-degree steps
const LON_SCALE: f64 = 180.0 / (i32::MAX as f64);
/// Scale factor: i32 full range covers +/- 90 degrees with ~42 nano-degree steps
const LAT_SCALE: f64 = 90.0 / (i32::MAX as f64);

/// Encode a longitude in degrees as a 4-byte big-endian scaled i32
fn scale_lon_enc(v: &f64) -> Vec<u8> {
    let data = (*v / LON_SCALE) as i32;
    encb::be_i32(data)
}

/// Encode a latitude in degrees as a 4-byte big-endian scaled i32
fn scale_lat_enc(v: &f64) -> Vec<u8> {
    let data = (*v / LAT_SCALE) as i32;
    encb::be_i32(data)
}

/// Decode a 4-byte big-endian scaled i32 back to an f64 longitude in degrees
fn scale_lon_dec(input: &mut &[u8]) -> tinyklv::Result<f64> {
    let data = decb::be_i32(input)?;
    Ok(data as f64 * LON_SCALE)
}

/// Decode a 4-byte big-endian scaled i32 back to an f64 latitude in degrees
fn scale_lat_dec(input: &mut &[u8]) -> tinyklv::Result<f64> {
    let data = decb::be_i32(input)?;
    Ok(data as f64 * LAT_SCALE)
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"GPSFIX",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// GPS fix with scaled-integer lat/lon and an HDOP quality indicator
struct GpsFix {
    #[klv(
        key = 0x01,
        dec = scale_lat_dec,
        enc = scale_lat_enc,
    )]
    /// Latitude in degrees (stored as scaled i32)
    latitude_deg: f64,

    #[klv(
        key = 0x02,
        dec = scale_lon_dec,
        enc = scale_lon_enc,
    )]
    /// Longitude in degrees (stored as scaled i32)
    longitude_deg: f64,

    #[klv(
        key = 0x03,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// HDOP multiplied by 100 (u16, built-in codec)
    hdop_centiunits: u16,
}

fn main() {
    // build - a Paris fix with a sharp HDOP
    let original = GpsFix {
        latitude_deg:    48.856_6,
        longitude_deg:    2.352_2,
        hdop_centiunits:     95, // HDOP = 0.95
    };

    // encode - the custom fns produce 4-byte scaled i32 values for lat/lon
    let frame = original.encode_frame();

    // decode - the hand-written decoders invert the scaling
    let decoded = GpsFix::decode_frame(
        &mut frame.as_slice(),
    ).unwrap();

    // assert - scaled integers introduce ~1e-7 degree quantisation;
    // tolerate that but require exact equality on the integer field
    assert!((decoded.latitude_deg  - original.latitude_deg ).abs() < 1e-6);
    assert!((decoded.longitude_deg - original.longitude_deg).abs() < 1e-6);
    assert_eq!(decoded.hdop_centiunits, original.hdop_centiunits);
}
