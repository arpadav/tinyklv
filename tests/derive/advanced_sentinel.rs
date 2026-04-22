//! Advanced sentinel seek tests for `#[derive(Klv)]`
//!
//! Tests sentinel-based seek, encode, and extract across complex domain
//! types from `types.rs`. Covers roundtrip, prefix verification, multi-type
//! streams, garbage tolerance, not-found errors, and interleaved extraction.
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
// NavPacket - sentinel 0xBEEF
// --------------------------------------------------

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
        dec = Timestamp::decode_value,
        enc = Timestamp::encode_value,
    )]
    timestamp: Timestamp,
    #[klv(
        key = 0x02,
        dec = Coordinate::decode_value,
        enc = Coordinate::encode_value,
    )]
    coordinate: Coordinate,
    #[klv(
        key = 0x03,
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    velocity: Velocity,
    #[klv(
        key = 0x04,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
}

// --------------------------------------------------
// WeatherPacket - sentinel 0xCAFE
// --------------------------------------------------

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
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    velocity: Velocity,
}

// --------------------------------------------------
// StatusPacket - sentinel 0xFACE
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xFA\xCE",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct StatusPacket {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
    #[klv(
        key = 0x02,
        dec = StatusFlags::decode_value,
        enc = StatusFlags::encode_value,
    )]
    flags: StatusFlags,
}

// --------------------------------------------------
// helpers
// --------------------------------------------------

fn make_nav() -> NavPacket {
    NavPacket {
        timestamp: Timestamp {
            seconds: 1_700_000_000,
            nanos: 500,
        },
        coordinate: Coordinate {
            lat: 37.7749,
            lon: -122.4194,
        },
        velocity: Velocity {
            dx: 10,
            dy: -5,
            dz: 2,
        },
        color: Color::Blue,
    }
}

fn make_weather() -> WeatherPacket {
    WeatherPacket {
        priority: Priority::High,
        velocity: Velocity {
            dx: 3,
            dy: 7,
            dz: -1,
        },
    }
}

fn make_status() -> StatusPacket {
    StatusPacket {
        color: Color::Red,
        flags: StatusFlags {
            active: true,
            armed: false,
            locked: true,
            mode: 0x0A,
        },
    }
}

// --------------------------------------------------
// tests
// --------------------------------------------------

#[test]
/// Tests that a sentinel-framed `NavPacket` carrying four domain types roundtrips through `encode_frame`/`decode_frame`.
fn sentinel_complex_roundtrip() {
    let original = make_nav();
    let encoded = original.encode_frame();
    let decoded = NavPacket::decode_frame(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Verifies that `encode_frame` prepends the configured sentinel bytes (`0xBEEF`) at the start of the output.
fn sentinel_encode_prefix() {
    let encoded = make_nav().encode_frame();
    assert_eq!(
        &encoded[0..2],
        b"\xBE\xEF",
        "encode() must prepend sentinel 0xBEEF"
    );
}

#[test]
/// Tests that three concatenated frames with distinct sentinels can each be decoded sequentially from a shared cursor.
fn multi_type_extract() {
    let nav = make_nav();
    let weather = make_weather();
    let status = make_status();

    let mut stream: Vec<u8> = nav.encode_frame();
    stream.extend(weather.encode_frame());
    stream.extend(status.encode_frame());

    let decoded_nav = NavPacket::decode_frame(&mut stream.as_slice()).unwrap();
    let decoded_weather = WeatherPacket::decode_frame(&mut stream.as_slice()).unwrap();
    let decoded_status = StatusPacket::decode_frame(&mut stream.as_slice()).unwrap();

    assert_eq!(decoded_nav, nav);
    assert_eq!(decoded_weather, weather);
    assert_eq!(decoded_status, status);
}

#[test]
/// Tests that sentinel seek tolerates leading garbage and a near-miss prefix (`0xBEEE`) between two valid `NavPacket` frames.
fn multi_packet_garbage() {
    let nav1 = make_nav();
    let nav2 = NavPacket {
        timestamp: Timestamp {
            seconds: 1_700_000_001,
            nanos: 0,
        },
        coordinate: Coordinate {
            lat: 40.7128,
            lon: -74.0060,
        },
        velocity: Velocity {
            dx: 0,
            dy: 0,
            dz: 0,
        },
        color: Color::Green,
    };

    let mut stream: Vec<u8> = vec![0xDE, 0xAD, 0xFF, 0xFF];
    stream.extend(nav1.encode_frame());
    // partial near-miss: 0xBE 0xEE is not the sentinel
    stream.extend_from_slice(&[0xBE, 0xEE, 0x00]);
    stream.extend(nav2.encode_frame());

    let mut slice = stream.as_slice();
    let first = NavPacket::decode_frame(&mut slice).unwrap();
    let second = NavPacket::decode_frame(&mut slice).unwrap();

    assert_eq!(first, nav1);
    assert_eq!(second, nav2);
}

#[test]
/// Tests that `decode_frame` errors when the expected sentinel is absent from the stream or the stream is empty.
fn sentinel_not_found() {
    // Stream contains only a WeatherPacket - NavPacket sentinel 0xBEEF absent
    let weather_bytes = make_weather().encode_frame();
    assert!(
        NavPacket::decode_frame(&mut weather_bytes.as_slice()).is_err(),
        "should fail: 0xBEEF sentinel not present in WeatherPacket stream"
    );

    // Empty stream
    assert!(
        NavPacket::decode_frame(&mut [].as_ref()).is_err(),
        "should fail: empty stream"
    );
}

#[test]
/// Tests that interleaved `NavPacket`/`WeatherPacket` frames can be extracted independently by type using separate cursors.
fn interleaved_extract() {
    let nav1 = make_nav();
    let nav2 = NavPacket {
        timestamp: Timestamp {
            seconds: 42,
            nanos: 7,
        },
        coordinate: Coordinate {
            lat: 51.5074,
            lon: -0.1278,
        },
        velocity: Velocity {
            dx: -3,
            dy: 2,
            dz: 0,
        },
        color: Color::Alpha,
    };
    let weather1 = make_weather();
    let weather2 = WeatherPacket {
        priority: Priority::Critical,
        velocity: Velocity {
            dx: 100,
            dy: 200,
            dz: 50,
        },
    };

    let mut stream: Vec<u8> = nav1.encode_frame();
    stream.extend(weather1.encode_frame());
    stream.extend(nav2.encode_frame());
    stream.extend(weather2.encode_frame());

    // Extract both NavPackets from one cursor
    let mut nav_slice = stream.as_slice();
    let got_nav1 = NavPacket::decode_frame(&mut nav_slice).unwrap();
    let got_nav2 = NavPacket::decode_frame(&mut nav_slice).unwrap();
    assert_eq!(got_nav1, nav1);
    assert_eq!(got_nav2, nav2);

    // Extract both WeatherPackets from a fresh cursor
    let mut weather_slice = stream.as_slice();
    let got_weather1 = WeatherPacket::decode_frame(&mut weather_slice).unwrap();
    let got_weather2 = WeatherPacket::decode_frame(&mut weather_slice).unwrap();
    assert_eq!(got_weather1, weather1);
    assert_eq!(got_weather2, weather2);
}
