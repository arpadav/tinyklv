//! `DrainFrames` and repeated sentinel-extraction tests for `#[derive(Klv)]`
//!
//! Covers sentinel-framed extraction via `drain_frames` and manual
//! `while let Ok(...) = T::decode_frame(...)` loops across two structs:
//! `Waypoint` (lat/lon coordinate + priority) and `FramedPacket` (color +
//! timestamp).  Tests include single-packet roundtrip, three-packet loop,
//! empty-stream error, and float-precision verification across a batch of
//! polar-extreme coordinates
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x57\x41",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
struct Waypoint {
    #[klv(key = 0x01)]
    coordinate: Coordinate,

    #[klv(key = 0x02)]
    priority: Priority,
}
impl Waypoint {
    /// Construct a [`Waypoint`] from raw lat/lon degrees and a priority level
    fn new(lat: f64, lon: f64, prio: Priority) -> Self {
        Self {
            coordinate: Coordinate { lat, lon },
            priority: prio,
        }
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"UF",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
struct FramedPacket {
    #[klv(key = 0x01)]
    color: Color,

    #[klv(key = 0x02)]
    timestamp: Timestamp,
}

#[test]
/// Tests that sentinel framing lets `decode_frame` extract three independent back-to-back packets from one stream
fn repeated_sentinel_extract_loop() {
    let w1 = Waypoint::new(48.8566, 2.3522, Priority::Low);
    let w2 = Waypoint::new(51.5074, -0.1278, Priority::Medium);
    let w3 = Waypoint::new(40.7128, -74.0060, Priority::High);

    let mut stream: Vec<u8> = w1.encode_frame();
    stream.extend(w2.encode_frame());
    stream.extend(w3.encode_frame());

    let mut results: Vec<Waypoint> = Vec::new();
    let mut slice = stream.as_slice();
    while let Ok(w) = Waypoint::decode_frame(&mut slice) {
        results.push(w);
    }

    assert_eq!(results.len(), 3);
    assert_eq!(results[0], w1);
    assert_eq!(results[1], w2);
    assert_eq!(results[2], w3);
}

#[test]
/// Tests that `decode_frame` on an empty stream errors because the sentinel cannot be found
fn repeated_sentinel_extract_empty_stream() {
    let mut slice: &[u8] = &[];
    let result = Waypoint::decode_frame(&mut slice);
    assert!(result.is_err(), "empty stream must fail extract");
}

#[test]
/// Verifies encode/decode roundtrip for a single sentinel-framed `Waypoint` via `encode_frame`/`decode_frame`
fn repeated_sentinel_extract_single() {
    let w = Waypoint::new(35.6762, 139.6503, Priority::Critical);
    let encoded = w.encode_frame();
    let decoded = Waypoint::decode_frame(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, w);
}

#[test]
/// Tests that `drain_frames` extracts two independently framed packets
fn drain_frames_two_framed_packets() {
    let p1 = FramedPacket {
        color: Color::Red,
        timestamp: Timestamp {
            seconds: 1,
            nanos: 0,
        },
    };
    let p2 = FramedPacket {
        color: Color::Blue,
        timestamp: Timestamp {
            seconds: 2,
            nanos: 500,
        },
    };

    let mut stream = p1.encode_frame();
    stream.extend(p2.encode_frame());

    let results = FramedPacket::drain_frames(&mut stream.as_slice()).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].color, p1.color);
    assert_eq!(results[0].timestamp, p1.timestamp);
    assert_eq!(results[1].color, p2.color);
    assert_eq!(results[1].timestamp, p2.timestamp);
}

#[test]
/// Verifies that `drain_frames` returns an empty `Vec` for an empty input
fn drain_frames_empty_returns_empty() {
    let results = FramedPacket::drain_frames(&mut [].as_slice()).unwrap();
    assert!(results.is_empty());
}

#[test]
/// Tests that field values survive the encode -> extract-loop roundtrip without numerical drift across three framed waypoints
fn repeated_sentinel_three_roundtrip_values() {
    let waypoints = [
        Waypoint::new(0.0, 0.0, Priority::Low),
        Waypoint::new(-90.0, 180.0, Priority::Critical),
        Waypoint::new(90.0, -180.0, Priority::High),
    ];

    let stream: Vec<u8> = waypoints.iter().flat_map(tinyklv::EncodeFrame::encode_frame).collect();
    let mut slice = stream.as_slice();
    let mut decoded: Vec<Waypoint> = Vec::new();
    while let Ok(w) = Waypoint::decode_frame(&mut slice) {
        decoded.push(w);
    }

    assert_eq!(decoded.len(), waypoints.len());
    for (got, want) in decoded.iter().zip(waypoints.iter()) {
        assert_eq!(got.priority, want.priority);
        assert!((got.coordinate.lat - want.coordinate.lat).abs() < 1e-9);
        assert!((got.coordinate.lon - want.coordinate.lon).abs() < 1e-9);
    }
}
