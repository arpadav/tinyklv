//! `RepeatedDecode` trait tests
//!
//! Covers `extract()`-based looping for sentinel structs (proper framing)
//! and `repeated()`/`decode()` behavior for unframed structs (last-wins
//! merge semantics documented inline).
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// Waypoint - sentinel 0x5741 ("WA"), framed
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x57\x41",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Waypoint {
    #[klv(
        key = 0x01,
        dec = Coordinate::decode_value,
        enc = Coordinate::encode_value,
    )]
    coordinate: Coordinate,
    #[klv(
        key = 0x02,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    priority: Priority,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct UnframedPacket {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
    #[klv(
        key = 0x02,
        dec = Timestamp::decode_value,
        enc = Timestamp::encode_value,
    )]
    timestamp: Timestamp,
}

fn make_waypoint(lat: f64, lon: f64, prio: Priority) -> Waypoint {
    Waypoint {
        coordinate: Coordinate { lat, lon },
        priority: prio,
    }
}

#[test]
/// Tests that sentinel framing lets `decode_frame` extract three independent back-to-back packets from one stream.
fn repeated_sentinel_extract_loop() {
    let w1 = make_waypoint(48.8566, 2.3522, Priority::Low);
    let w2 = make_waypoint(51.5074, -0.1278, Priority::Medium);
    let w3 = make_waypoint(40.7128, -74.0060, Priority::High);

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
/// Tests that `decode_frame` on an empty stream errors because the sentinel cannot be found.
fn repeated_sentinel_extract_empty_stream() {
    let mut slice: &[u8] = &[];
    let result = Waypoint::decode_frame(&mut slice);
    assert!(result.is_err(), "empty stream must fail extract");
}

#[test]
/// Verifies encode/decode roundtrip for a single sentinel-framed `Waypoint` via `encode_frame`/`decode_frame`.
fn repeated_sentinel_extract_single() {
    let w = make_waypoint(35.6762, 139.6503, Priority::Critical);
    let encoded = w.encode_frame();
    let decoded = Waypoint::decode_frame(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, w);
}

/// Without sentinel framing there is no boundary between logical packets.
///
/// `decode()` runs a single key-value loop consuming the entire stream.
/// When both packets share the same key set (0x01, 0x02), all four triples
/// are processed in one pass and the last-seen value for each key wins.
/// `repeated()` calls `decode()` repeatedly until failure; the first call
#[test]
/// consumes everything, so `repeated()` returns a Vec of length 1.
fn repeated_decode_unframed_last_wins_merge() {
    let p1 = UnframedPacket {
        color: Color::Red,
        timestamp: Timestamp {
            seconds: 1,
            nanos: 0,
        },
    };
    let p2 = UnframedPacket {
        color: Color::Blue,
        timestamp: Timestamp {
            seconds: 2,
            nanos: 500,
        },
    };

    let mut stream = p1.encode_value();
    stream.extend(p2.encode_value());

    // repeated() wraps decode() in a loop; the single decode() call
    // consumes all bytes, merging both packets (last-wins per key).
    let results = UnframedPacket::repeated(&mut stream.as_slice()).unwrap();
    assert_eq!(
        results.len(),
        1,
        "unframed stream: one decode() call consumes all bytes → repeated() returns 1 element"
    );

    // p2 values win for both keys
    assert_eq!(results[0].color, p2.color);
    assert_eq!(results[0].timestamp, p2.timestamp);
}

#[test]
/// Verifies that `repeated` returns an empty `Vec` for an unframed struct fed an empty input.
fn repeated_decode_unframed_empty_returns_empty() {
    let results = UnframedPacket::repeated(&mut [].as_slice()).unwrap();
    assert!(results.is_empty());
}

#[test]
/// Tests that field values survive the encode -> extract-loop roundtrip without numerical drift across three framed waypoints.
fn repeated_sentinel_three_roundtrip_values() {
    let waypoints = [
        make_waypoint(0.0, 0.0, Priority::Low),
        make_waypoint(-90.0, 180.0, Priority::Critical),
        make_waypoint(90.0, -180.0, Priority::High),
    ];

    let stream: Vec<u8> = waypoints.iter().flat_map(|w| w.encode_frame()).collect();
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
