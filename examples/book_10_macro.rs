#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 10 - `scale!`, `cast!`, and their encode counterparts
//! See `book/tutorial/10-macros.md` for the full narrative
//!
//! Demonstrates `tinyklv::scale!`, `tinyklv::cast!`, and their encode
//! counterparts used inline in `#[klv(...)]` attributes. Any macro
//! that expands to a closure matching the decoder / encoder contract
//! works in `dec = ...` / `enc = ...` - including container-level
//! `default(typ = T, dec = ...)`. The example maps a `u16` wire range to
//! floating-point heading and altitude values, and a `u16` wire type to a
//! `u32` in-memory field via `cast!`
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

// u16 [0, 65535] -> degrees [0.0, 360.0)
const HEADING_SCALE: f64 = 360.0 / 65535.0;

// u16 [0, 65535] -> metres [-900.0, 19000.0]
const ALTITUDE_SCALE: f64 = 19900.0 / 65535.0;
const ALTITUDE_OFFSET: f64 = -900.0;

fn decode_altitude(
    input: &mut &[u8],
) -> tinyklv::Result<f64> {
    let raw = decb::be_u16.parse_next(input)?;
    Ok(raw as f64 * ALTITUDE_SCALE + ALTITUDE_OFFSET)
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8], // default, shown for clarity
    sentinel = b"TELEMETRY",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Telemetry packet using `scale!`, `cast!`, and offset macros for field codecs
struct TelemetryPacket {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Sensor node identifier
    node_id: u8,

    // scale! inline: be_u16 -> f64 heading in degrees
    #[klv(
        key = 0x02,
        dec = tinyklv::scale!(decb::be_u16, f64, HEADING_SCALE),
        enc = tinyklv::scale_enc!(
            encb::be_u16, f64, u16, HEADING_SCALE,
        ),
    )]
    /// Heading in decimal degrees, mapped from a `u16` wire range via `scale!`
    heading_deg: f64,

    // named function for decode (scale + offset), scale_offset_enc! for encode
    #[klv(
        key = 0x03,
        dec = decode_altitude,
        enc = tinyklv::scale_offset_enc!(
            encb::be_u16, f64, u16,
            ALTITUDE_SCALE, ALTITUDE_OFFSET,
        ),
    )]
    /// Altitude in metres, mapped from a `u16` wire range with scale and offset
    altitude_m: f64,

    // cast! inline: be_u16 -> field u32
    #[klv(
        key = 0x04,
        dec = tinyklv::cast!(decb::be_u16, u32),
        enc = tinyklv::cast_enc!(encb::be_u16, u32, u16),
    )]
    /// Sample counter; wire type is `u16`, in-memory type is `u32` via `cast!`
    sample_count: u32,

    // plain u8 for comparison - no macro needed
    #[klv(
        key = 0x05,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Decode quality score (0..=100)
    quality: u8,
}

fn main() {
    let pkt = TelemetryPacket {
        node_id:      0x0A,
        heading_deg:  180.0,
        altitude_m:   5000.0,
        sample_count: 1024,
        quality:      99,
    };

    // encode -> decode roundtrip
    let mut frame = Vec::new();
    pkt.encode_frame(&mut frame);

    // junk before sentinel, then the frame
    let mut data: Vec<u8> = vec![
        0x00, 0x00, 0xDE, 0xAD, 0x00,
    ];
    data.extend(&frame);

    let decoded = TelemetryPacket::decode_frame(
        &mut data.as_slice(),
    ).unwrap();

    // heading: u16 roundtrip loses sub-scale precision
    let heading_tolerance = HEADING_SCALE;
    assert!(
        (decoded.heading_deg - pkt.heading_deg).abs()
            < heading_tolerance,
        "heading: {} vs {}",
        decoded.heading_deg, pkt.heading_deg,
    );

    // altitude: same precision loss from u16 quantisation
    let altitude_tolerance = ALTITUDE_SCALE;
    assert!(
        (decoded.altitude_m - pkt.altitude_m).abs()
            < altitude_tolerance,
        "altitude: {} vs {}",
        decoded.altitude_m, pkt.altitude_m,
    );

    assert_eq!(decoded.sample_count, pkt.sample_count);
    assert_eq!(decoded.quality, pkt.quality);
    assert_eq!(decoded.node_id, pkt.node_id);

    println!(
        "roundtrip OK: heading={:.2}°, alt={:.1}m, \
         samples={}, quality={}",
        decoded.heading_deg,
        decoded.altitude_m,
        decoded.sample_count,
        decoded.quality,
    );
}
