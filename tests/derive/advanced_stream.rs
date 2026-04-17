//! Corruption resilience and auto-generated stream tests for `#[derive(Klv)]`
//!
//! Covers unknown key skipping, corrupt length truncation, corrupt value
//! recovery via `.ok()`, and multi-packet encode/extract loops using
//! sentinel structs built from domain types in `types.rs`.
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// SimplePosition - no sentinel, for corruption tests
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct SimplePosition {
    #[klv(key = 0x01, dec = Coordinate::decode_value, enc = Coordinate::encode_value)]
    coordinate: Coordinate,
    #[klv(key = 0x02, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
}

// --------------------------------------------------
// PartialReading - optional fields for corruption tests
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct PartialReading {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Option<Color>,
    #[klv(key = 0x02, dec = Velocity::decode_value, enc = Velocity::encode_value)]
    velocity: Option<Velocity>,
    #[klv(key = 0x03, dec = Timestamp::decode_value, enc = Timestamp::encode_value)]
    timestamp: Option<Timestamp>,
}

// --------------------------------------------------
// Waypoint - sentinel 0x5741, for auto-generate tests
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x57\x41",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Waypoint {
    #[klv(key = 0x01, dec = Coordinate::decode_value, enc = Coordinate::encode_value)]
    coordinate: Coordinate,
    #[klv(key = 0x02, dec = Timestamp::decode_value, enc = Timestamp::encode_value)]
    timestamp: Timestamp,
    #[klv(key = 0x03, dec = Priority::decode_value, enc = Priority::encode_value)]
    priority: Priority,
}

// --------------------------------------------------
// Alert - sentinel 0x414C, for auto-generate tests
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x41\x4C",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Alert {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
    #[klv(key = 0x02, dec = StatusFlags::decode_value, enc = StatusFlags::encode_value)]
    flags: StatusFlags,
}

// --------------------------------------------------
// helpers
// --------------------------------------------------

/// Encode a `SimplePosition` manually so we can inject arbitrary bytes between
/// its fields.  Layout: key(1) + len(1) + value for each TLV triple.
fn encode_coordinate_tlv(key: u8, coord: &Coordinate) -> Vec<u8> {
    let val = Coordinate::encode_value(coord);
    let mut out = vec![key, val.len() as u8];
    out.extend(val);
    out
}

fn encode_color_tlv(key: u8, color: &Color) -> Vec<u8> {
    let val = Color::encode_value(color);
    let mut out = vec![key, val.len() as u8];
    out.extend(val);
    out
}

fn encode_timestamp_tlv(key: u8, ts: &Timestamp) -> Vec<u8> {
    let val = Timestamp::encode_value(ts);
    let mut out = vec![key, val.len() as u8];
    out.extend(val);
    out
}

// --------------------------------------------------
// tests
// --------------------------------------------------

#[test]
/// Tests that unknown TLV triples inserted between valid keys are skipped without disrupting decode of known fields.
fn unknown_keys_between_valid() {
    // Build stream manually: valid coord(0x01), unknown 0xAA(len=3, garbage),
    // unknown 0xBB(len=2, garbage), valid color(0x02).
    let coord = Coordinate {
        lat: 48.8566,
        lon: 2.3522,
    };
    let color = Color::Green;

    let mut stream: Vec<u8> = Vec::new();
    stream.extend(encode_coordinate_tlv(0x01, &coord));
    stream.extend_from_slice(&[0xAA, 0x03, 0xDE, 0xAD, 0xFF]);
    stream.extend_from_slice(&[0xBB, 0x02, 0xCA, 0xFE]);
    stream.extend(encode_color_tlv(0x02, &color));

    let result = SimplePosition::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.coordinate, coord,
        "coordinate should decode despite unknown keys"
    );
    assert_eq!(
        result.color, color,
        "color should decode despite unknown keys"
    );
}

#[test]
/// Tests that a declared length exceeding the remaining input surfaces a truncation error rather than silently returning partial data.
fn corrupt_length_fails_loudly() {
    // Stream: valid color(0x01), then key 0x02 with declared len=100 but only
    // 2 bytes of body before end-of-input. The declared length overruns the
    // remaining input - this is a truncated/malformed frame, not a clean EOF,
    // and decode must surface it as an error (not silently return partial data).
    let color = Color::Blue;

    let mut stream: Vec<u8> = Vec::new();
    stream.extend(encode_color_tlv(0x01, &color));
    stream.extend_from_slice(&[0x02, 100, 0x00, 0x01]);

    let err = PartialReading::decode_value(&mut stream.as_slice())
        .expect_err("declared length 100 exceeds remaining 2 bytes - must error");
    let rendered = format!("{err:?}");
    assert!(
        rendered.contains("truncated"),
        "error context should mention truncation; got: {rendered}"
    );
}

#[test]
/// Tests that a field-decoder failure on an optional (e.g. short velocity) leaves the field `None` and decoding continues for subsequent keys.
fn corrupt_value_recoverable() {
    // Stream: valid color(0x01), key 0x02 len=6 but garbage bytes (Velocity
    // decode fails → .ok()→None, loop continues), then valid timestamp(0x03).
    let color = Color::Alpha;
    let ts = Timestamp {
        seconds: 1_000_000,
        nanos: 250,
    };

    let mut stream: Vec<u8> = Vec::new();
    stream.extend(encode_color_tlv(0x01, &color));
    // key 0x02, len=5 (Velocity needs 6 bytes for 3×i16) - decode_velocity
    // fails on the short subslice, .ok()→None, loop continues
    stream.extend_from_slice(&[0x02, 0x05, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    stream.extend(encode_timestamp_tlv(0x03, &ts));

    let result = PartialReading::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(result.color, Some(color), "color should have decoded");
    assert_eq!(
        result.velocity, None,
        "velocity decode failed → None kept via .ok()"
    );
    assert_eq!(
        result.timestamp,
        Some(ts),
        "timestamp should decode after corrupt velocity"
    );
}

#[test]
/// Tests that a concatenated stream of 10 sentinel-framed `Waypoint` packets decodes back to the original sequence in order.
fn auto_generate_10_packets() {
    // Build 10 distinct Waypoints, encode each, concatenate, then extract all.
    let waypoints: Vec<Waypoint> = (0..10)
        .map(|i| Waypoint {
            coordinate: Coordinate {
                lat: 10.0 * i as f64,
                lon: -5.0 * i as f64,
            },
            timestamp: Timestamp {
                seconds: 1_700_000_000 + i as u32 * 60,
                nanos: i as u16 * 100,
            },
            priority: match i % 4 {
                0 => Priority::Low,
                1 => Priority::Medium,
                2 => Priority::High,
                _ => Priority::Critical,
            },
        })
        .collect();

    let stream: Vec<u8> = waypoints.iter().flat_map(|w| w.encode_frame()).collect();

    let mut slice = stream.as_slice();
    let mut decoded: Vec<Waypoint> = Vec::new();
    for _ in 0..10 {
        decoded.push(Waypoint::decode_frame(&mut slice).unwrap());
    }

    assert_eq!(decoded.len(), 10);
    for (i, (got, expected)) in decoded.iter().zip(waypoints.iter()).enumerate() {
        assert_eq!(got, expected, "waypoint[{i}] mismatch");
    }
}

#[test]
/// Tests that interleaved `Waypoint` and `Alert` frames can each be extracted independently using separate cursors keyed on their sentinels.
fn auto_generate_mixed_types() {
    // 3 Waypoints + 3 Alerts interleaved, then extract each type independently.
    let waypoints: Vec<Waypoint> = vec![
        Waypoint {
            coordinate: Coordinate {
                lat: 37.7749,
                lon: -122.4194,
            },
            timestamp: Timestamp {
                seconds: 1_000,
                nanos: 0,
            },
            priority: Priority::High,
        },
        Waypoint {
            coordinate: Coordinate {
                lat: 40.7128,
                lon: -74.0060,
            },
            timestamp: Timestamp {
                seconds: 2_000,
                nanos: 500,
            },
            priority: Priority::Low,
        },
        Waypoint {
            coordinate: Coordinate {
                lat: 51.5074,
                lon: -0.1278,
            },
            timestamp: Timestamp {
                seconds: 3_000,
                nanos: 999,
            },
            priority: Priority::Critical,
        },
    ];
    let alerts: Vec<Alert> = vec![
        Alert {
            color: Color::Red,
            flags: StatusFlags {
                active: true,
                armed: false,
                locked: false,
                mode: 0x01,
            },
        },
        Alert {
            color: Color::Blue,
            flags: StatusFlags {
                active: false,
                armed: true,
                locked: true,
                mode: 0x0A,
            },
        },
        Alert {
            color: Color::Green,
            flags: StatusFlags {
                active: true,
                armed: true,
                locked: false,
                mode: 0x1F,
            },
        },
    ];

    // Interleave: W A W A W A
    let mut stream: Vec<u8> = Vec::new();
    for i in 0..3 {
        stream.extend(waypoints[i].encode_frame());
        stream.extend(alerts[i].encode_frame());
    }

    // Extract all 3 Waypoints via advancing cursor
    let mut wp_slice = stream.as_slice();
    let got_wp: Vec<Waypoint> = (0..3)
        .map(|_| Waypoint::decode_frame(&mut wp_slice).unwrap())
        .collect();
    assert_eq!(got_wp, waypoints);

    // Extract all 3 Alerts via a fresh cursor
    let mut al_slice = stream.as_slice();
    let got_al: Vec<Alert> = (0..3)
        .map(|_| Alert::decode_frame(&mut al_slice).unwrap())
        .collect();
    assert_eq!(got_al, alerts);
}
