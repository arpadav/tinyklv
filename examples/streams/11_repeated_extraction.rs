#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 11 - draining a concatenated stream of KLV frames
//!
//! Autonomous-vehicle and robotics pipelines frequently pack many framed
//! records back-to-back into one UDP datagram or log file. Because each
//! frame carries its own sentinel and outer length, `decode_frame` can be
//! called repeatedly against a single slice to recover every record
//!
//! The `DrainFrames::drain_frames` trait exists for the value-region form
//! (repeats `decode_value` until EOF), but for sentinel-framed streams the
//! idiomatic pattern is a small `while let Ok(...) = T::decode_frame(...)`
//! loop - shown here
//!
//! Showcases:
//! * Concatenating N `encode_frame` outputs into one byte stream
//! * Draining that stream with a `while let Ok` loop on `decode_frame`
//! * Asserting count and per-field correctness against the source data
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq, Clone, Copy)]
#[klv(
    stream = &[u8],
    sentinel = b"WAYPOINT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Autonomous-navigation waypoint record
struct Waypoint {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    /// Monotonic waypoint index
    index: u32,

    #[klv(
        key = 0x02,
        dec = decb::be_f64,
        enc = *encb::be_f64,
    )]
    /// Latitude in decimal degrees (f64)
    lat_deg: f64,

    #[klv(
        key = 0x03,
        dec = decb::be_f64,
        enc = *encb::be_f64,
    )]
    /// Longitude in decimal degrees (f64)
    lon_deg: f64,

    #[klv(
        key = 0x04,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Speed limit at this waypoint, in 0.1 m/s units
    speed_limit_dms: u8,
}

fn main() {
    // build - five distinct survey waypoints
    let waypoints = [
        Waypoint { index: 0, lat_deg: 37.7749, lon_deg: -122.4194, speed_limit_dms: 30 },
        Waypoint { index: 1, lat_deg: 37.7750, lon_deg: -122.4200, speed_limit_dms: 20 },
        Waypoint { index: 2, lat_deg: 37.7751, lon_deg: -122.4210, speed_limit_dms: 15 },
        Waypoint { index: 3, lat_deg: 37.7748, lon_deg: -122.4215, speed_limit_dms: 25 },
        Waypoint { index: 4, lat_deg: 37.7745, lon_deg: -122.4220, speed_limit_dms: 30 },
    ];

    // encode - every waypoint becomes its own sentinel-prefixed frame,
    // and the results are concatenated into one byte stream
    let mut stream: Vec<u8> = Vec::new();
    for wp in &waypoints {
        tinyklv::EncodeFrame::encode_frame(wp, &mut stream);
    }

    // decode - drain the stream one frame at a time until EOF
    let mut slice = stream.as_slice();
    let mut decoded: Vec<Waypoint> = Vec::new();
    while let Ok(wp) = Waypoint::decode_frame(&mut slice) {
        decoded.push(wp);
    }

    // assert - count and field-level equality for every waypoint
    assert_eq!(decoded.len(), waypoints.len());
    for (got, want) in decoded.iter().zip(waypoints.iter()) {
        assert_eq!(got.index,           want.index);
        assert!((got.lat_deg - want.lat_deg).abs() < 1e-9);
        assert!((got.lon_deg - want.lon_deg).abs() < 1e-9);
        assert_eq!(got.speed_limit_dms, want.speed_limit_dms);
    }
}
