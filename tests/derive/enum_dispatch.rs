//! Manual enum dispatch pattern for multi-type KLV streams
//!
//! The derive macro does not support enums directly. This module demonstrates
//! the recommended pattern: define each packet type with `#[derive(Klv)]` and
//! a unique sentinel, then write a thin enum wrapper that peeks the sentinel
//! bytes and routes to the appropriate `::extract()` call.
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xBE\xEF",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct NavPacket {
    #[klv(
        key = 0x01,
        dec = Coordinate::decode_value,
        enc = Coordinate::encode_value,
    )]
    position: Coordinate,
    #[klv(
        key = 0x02,
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    velocity: Velocity,
}
impl Default for NavPacket {
    fn default() -> NavPacket {
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
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xCA\xFE",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct WeatherPacket {
    #[klv(
        key = 0x01,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    priority: Priority,
    #[klv(
        key = 0x02,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    sky_color: Color,
}
impl Default for WeatherPacket {
    fn default() -> WeatherPacket {
        WeatherPacket {
            priority: Priority::High,
            sky_color: Color::Blue,
        }
    }
}

#[derive(Debug, PartialEq)]
enum Packet {
    Nav(NavPacket),
    Weather(WeatherPacket),
}

/// Try to dispatch one packet from the stream.
///
/// Peeks the first two bytes to identify the sentinel, then calls the
/// appropriate `::decode_frame()`. Returns `None` if the sentinel is unrecognised
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

#[test]
/// Tests that a concatenated Nav+Weather stream routes each frame to its matching `Packet` variant via sentinel peeking.
fn dispatch_by_sentinel() {
    let nav = NavPacket::default();
    let weather = WeatherPacket::default();

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
/// Tests that interleaved packet types (Nav, Weather, Nav) are dispatched in order with variants preserved.
fn dispatch_nav_then_weather_then_nav() {
    let n1 = NavPacket::default();
    let w1 = WeatherPacket::default();
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
/// Tests that garbage leading bytes are skipped one at a time until a recognised sentinel is found and decoded.
fn dispatch_unknown_sentinel_skips_byte() {
    // Stream: 2 garbage bytes, then a valid NavPacket
    let nav = NavPacket::default();
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
/// Tests that a stream containing no recognised sentinels yields zero packets without panicking.
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
/// Tests that dispatching on an empty stream returns `None` without attempting to read.
fn dispatch_empty_stream() {
    let mut slice: &[u8] = &[];
    let result = dispatch_one(&mut slice);
    assert!(result.is_none());
}
