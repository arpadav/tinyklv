//! Corruption resilience and auto-generated stream tests for `#[derive(Klv)]`
//!
//! Covers unknown key skipping, corrupt length truncation, corrupt value
//! recovery via `.ok()`, and multi-packet encode/extract loops using
//! sentinel structs built from domain types in `types.rs`.
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    fallback_impls,
)]
struct SimplePosition {
    #[klv(key = 0x01)]
    coordinate: Coordinate,

    #[klv(key = 0x02)]
    color: Color,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    fallback_impls,
)]
struct PartialReading {
    #[klv(key = 0x01)]
    color: Option<Color>,

    #[klv(key = 0x02)]
    velocity: Option<Velocity>,

    #[klv(key = 0x03)]
    timestamp: Option<Timestamp>,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x57\x41",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    fallback_impls,
)]
struct Waypoint {
    #[klv(key = 0x01)]
    coordinate: Coordinate,

    #[klv(key = 0x02)]
    timestamp: Timestamp,

    #[klv(key = 0x03)]
    priority: Priority,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x41\x4C",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    fallback_impls,
)]
struct Alert {
    #[klv(key = 0x01)]
    color: Color,

    #[klv(key = 0x02)]
    flags: StatusFlags,
}

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
/// Tests that a declared length exceeding the remaining input surfaces
/// as recoverable truncation under the 2-arm `Progress` design:
/// `Ok(NeedMore(p))` carrying the bytes that DID land. The previous
/// "fail-loud-on-overrun" behaviour was a heuristic; the new contract
/// is "an entity never knows when the stream is complete unless the
/// user implements a `Done` break condition," so an overlong declared
/// length on the trailing key is just "more bytes coming."
///
/// Callers that want fail-loud-on-overrun explicitly should drive
/// finalisation via `Decoder::finish` (it surfaces missing-required
/// labels) or implement a `Done` break condition.
fn corrupt_length_surfaces_as_recoverable_needmore() {
    let color = Color::Blue;

    let mut stream: Vec<u8> = Vec::new();
    stream.extend(encode_color_tlv(0x01, &color));
    stream.extend_from_slice(&[0x02, 100, 0x00, 0x01]);

    let mut cursor: &[u8] = stream.as_slice();
    let p = match PartialReading::decode_partial(&mut cursor) {
        Ok(tinyklv::Progress::NeedMore(p)) => p,
        other => panic!(
            "expected NeedMore (recoverable truncation), got: {other:?}"
        ),
    };
    // color landed before the bad-length key; velocity/timestamp did
    // not arrive (the truncation rewound past key 0x02).
    assert_eq!(p.color, Some(Color::Blue));
    assert!(p.velocity.is_none());
    assert!(p.timestamp.is_none());
}

#[test]
/// Tests that a field-decoder failure on an optional (e.g. short velocity) leaves the field `None` and decoding continues for subsequent keys.
fn corrupt_value_recoverable() {
    // Stream: valid color(0x01), key 0x02 len=6 but garbage bytes (Velocity
    // decode fails -> .ok()->None, loop continues), then valid timestamp(0x03).
    let color = Color::Alpha;
    let ts = Timestamp {
        seconds: 1_000_000,
        nanos: 250,
    };

    let mut stream: Vec<u8> = Vec::new();
    stream.extend(encode_color_tlv(0x01, &color));
    // key 0x02, len=5 (Velocity needs 6 bytes for 3×i16) - decode_velocity
    // fails on the short subslice, .ok()->None, loop continues
    stream.extend_from_slice(&[0x02, 0x05, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
    stream.extend(encode_timestamp_tlv(0x03, &ts));

    let result = PartialReading::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(result.color, Some(color), "color should have decoded");
    assert_eq!(
        result.velocity, None,
        "velocity decode failed -> None kept via .ok()"
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
