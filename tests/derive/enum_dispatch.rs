//! Manual enum dispatch pattern for multi-type KLV streams
//!
//! The derive macro does not support enums directly. This module demonstrates
//! the recommended pattern: define each packet type with `#[derive(Klv)]` and
//! a unique sentinel, then write a thin enum wrapper that peeks the sentinel
//! bytes and routes to the appropriate `::extract()` call.
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// NavPacket - sentinel 0xBEEF
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xBE\xEF",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct NavPacket {
    #[klv(key = 0x01, dec = Coordinate::decode_value, enc = Coordinate::encode_value)]
    position: Coordinate,
    #[klv(key = 0x02, dec = Velocity::decode_value,   enc = Velocity::encode_value)]
    velocity: Velocity,
}

// --------------------------------------------------
// WeatherPacket - sentinel 0xCAFE
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xCA\xFE",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WeatherPacket {
    #[klv(key = 0x01, dec = Priority::decode_value, enc = Priority::encode_value)]
    priority: Priority,
    #[klv(key = 0x02, dec = Color::decode_value,    enc = Color::encode_value)]
    sky_color: Color,
}

// --------------------------------------------------
// Enum wrapper + manual dispatch
// --------------------------------------------------

#[derive(Debug, PartialEq)]
enum Packet {
    Nav(NavPacket),
    Weather(WeatherPacket),
}

/// Try to dispatch one packet from the stream.
///
/// Peeks the first two bytes to identify the sentinel, then calls the
/// appropriate `::extract()`. Returns `None` if the sentinel is unrecognised
/// or if the stream is empty. On an unrecognised sentinel the byte is
/// advanced past so that callers can keep scanning.
fn dispatch_one(input: &mut &[u8]) -> Option<Packet> {
    if input.len() < 2 {
        *input = &[];
        return None;
    }

    match &input[0..2] {
        b"\xBE\xEF" => NavPacket::decode_frame(input).ok().map(Packet::Nav),
        b"\xCA\xFE" => WeatherPacket::decode_frame(input).ok().map(Packet::Weather),
        _ => {
            // Advance one byte and signal unknown
            *input = &input[1..];
            None
        }
    }
}

// --------------------------------------------------
// helpers
// --------------------------------------------------

fn make_nav() -> NavPacket {
    NavPacket {
        position: Coordinate {
            lat: 37.7749,
            lon: -122.4194,
        },
        velocity: Velocity {
            dx: 10,
            dy: -5,
            dz: 0,
        },
    }
}

fn make_weather() -> WeatherPacket {
    WeatherPacket {
        priority: Priority::High,
        sky_color: Color::Blue,
    }
}

// --------------------------------------------------
// tests
// --------------------------------------------------

#[test]
fn dispatch_by_sentinel() {
    let nav = make_nav();
    let weather = make_weather();

    let mut stream: Vec<u8> = nav.encode_frame();
    stream.extend(weather.encode_frame());

    let mut slice = stream.as_slice();
    let mut packets: Vec<Packet> = Vec::new();

    // Drain the stream with the dispatch loop
    while !slice.is_empty() {
        if let Some(p) = dispatch_one(&mut slice) {
            packets.push(p);
        }
    }

    assert_eq!(packets.len(), 2);
    assert_eq!(packets[0], Packet::Nav(nav));
    assert_eq!(packets[1], Packet::Weather(weather));
}

#[test]
fn dispatch_nav_then_weather_then_nav() {
    let n1 = make_nav();
    let w1 = make_weather();
    let n2 = NavPacket {
        position: Coordinate {
            lat: 51.5074,
            lon: -0.1278,
        },
        velocity: Velocity {
            dx: 0,
            dy: 0,
            dz: 1,
        },
    };

    let mut stream: Vec<u8> = n1.encode_frame();
    stream.extend(w1.encode_frame());
    stream.extend(n2.encode_frame());

    let mut slice = stream.as_slice();
    let mut packets: Vec<Packet> = Vec::new();
    while !slice.is_empty() {
        if let Some(p) = dispatch_one(&mut slice) {
            packets.push(p);
        }
    }

    assert_eq!(packets.len(), 3);
    assert!(matches!(&packets[0], Packet::Nav(_)));
    assert!(matches!(&packets[1], Packet::Weather(_)));
    assert!(matches!(&packets[2], Packet::Nav(_)));
}

#[test]
fn dispatch_unknown_sentinel_skips_byte() {
    // Stream: 2 garbage bytes, then a valid NavPacket
    let nav = make_nav();
    let mut stream: Vec<u8> = vec![0xDE, 0xAD];
    stream.extend(nav.encode_frame());

    let mut slice = stream.as_slice();
    let mut packets: Vec<Packet> = Vec::new();
    while !slice.is_empty() {
        if let Some(p) = dispatch_one(&mut slice) {
            packets.push(p);
        }
    }

    assert_eq!(packets.len(), 1, "garbage bytes skipped, NavPacket decoded");
    assert_eq!(packets[0], Packet::Nav(nav));
}

#[test]
fn dispatch_all_unknown_returns_empty() {
    // Stream contains no recognised sentinels
    let stream: &[u8] = &[0x00, 0x11, 0x22, 0x33, 0x44];
    let mut slice = stream;
    let mut packets: Vec<Packet> = Vec::new();
    while !slice.is_empty() {
        if let Some(p) = dispatch_one(&mut slice) {
            packets.push(p);
        }
    }
    assert!(packets.is_empty(), "no recognised sentinels → no packets");
}

#[test]
fn dispatch_empty_stream() {
    let mut slice: &[u8] = &[];
    let result = dispatch_one(&mut slice);
    assert!(result.is_none());
}
