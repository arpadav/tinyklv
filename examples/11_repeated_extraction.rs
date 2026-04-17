//! Buffer containing 5 concatenated Waypoint frames; loop decode_frame to
//! extract all; asserts vec length and content. In autonomous-vehicle and
//! robotics pipelines, waypoint packets are often packed back-to-back in a
//! single UDP datagram or file record. Because each frame is wrapped with a
//! sentinel and a length prefix, `decode_frame` can recover each waypoint
//! independently from the concatenated stream. This example encodes 5 distinct
//! waypoints, concatenates them, then loops until the stream is exhausted,
//! collecting the results into a `Vec<Waypoint>` and verifying every field.

use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_f64(v: &f64) -> Vec<u8> {
    tinyklv::enc::binary::be_f64(*v)
}
fn enc_u8(v: &u8) -> Vec<u8> {
    tinyklv::enc::binary::u8(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

/// Autonomous-navigation waypoint packet.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    // Sentinel "WP" - unique to this packet type
    sentinel = b"\x57\x50",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Waypoint {
    // Sequential waypoint index
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    index: u32,

    // Latitude in decimal degrees (f64)
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_f64, enc = enc_f64)]
    lat_deg: f64,

    // Longitude in decimal degrees (f64)
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_f64, enc = enc_f64)]
    lon_deg: f64,

    // Speed limit at this waypoint in 0.1 m/s units
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    speed_limit_dms: u8,
}

fn main() {
    // Define 5 mission waypoints along a survey route
    let waypoints = [
        Waypoint {
            index: 0,
            lat_deg: 37.7749,
            lon_deg: -122.4194,
            speed_limit_dms: 30,
        },
        Waypoint {
            index: 1,
            lat_deg: 37.7750,
            lon_deg: -122.4200,
            speed_limit_dms: 20,
        },
        Waypoint {
            index: 2,
            lat_deg: 37.7751,
            lon_deg: -122.4210,
            speed_limit_dms: 15,
        },
        Waypoint {
            index: 3,
            lat_deg: 37.7748,
            lon_deg: -122.4215,
            speed_limit_dms: 25,
        },
        Waypoint {
            index: 4,
            lat_deg: 37.7745,
            lon_deg: -122.4220,
            speed_limit_dms: 30,
        },
    ];

    // Concatenate all 5 framed waypoints into a single byte stream
    // encode_frame prepends the sentinel + length for each packet
    let stream: Vec<u8> = waypoints.iter().flat_map(|w| w.encode_frame()).collect();
    println!(
        "Stream total: {} bytes covering {} waypoints",
        stream.len(),
        waypoints.len()
    );

    // Loop decode_frame until the stream is exhausted
    let mut slice = stream.as_slice();
    let mut decoded: Vec<Waypoint> = Vec::new();
    while let Ok(wp) = Waypoint::decode_frame(&mut slice) {
        decoded.push(wp);
    }

    // Must recover exactly 5 waypoints
    assert_eq!(decoded.len(), 5, "expected 5 waypoints");

    // Verify field-level correctness for every waypoint
    for (i, (got, want)) in decoded.iter().zip(waypoints.iter()).enumerate() {
        println!(
            "  wp[{}]: index={}, lat={:.4}°, lon={:.4}°, speed_limit={}(x0.1m/s)",
            i, got.index, got.lat_deg, got.lon_deg, got.speed_limit_dms
        );
        assert_eq!(got.index, want.index);
        assert!((got.lat_deg - want.lat_deg).abs() < 1e-9);
        assert!((got.lon_deg - want.lon_deg).abs() < 1e-9);
        assert_eq!(got.speed_limit_dms, want.speed_limit_dms);
    }

    println!("SUCCESS");
}
