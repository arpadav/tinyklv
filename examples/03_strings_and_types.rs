#![allow(clippy::unwrap_used)]
//! Struct mixing UTF-8 string, u32, and UTF-8 string fields in a single packet.
//!
//! Real sensor metadata often contains both numeric readings and human-readable
//! labels. This example models a weather-station registration packet with a
//! station identifier (UTF-8), a geographic region name (UTF-8), and a u32
//! serial number. It shows how variable-length string fields use `var = true`
//! combined with the length-taking codec signature, while fixed-width fields
//! use the simple `fn(&mut &[u8]) -> Result<T>` form. A full encode → decode
//! roundtrip is verified with `assert_eq!`.

use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

/// Weather-station registration metadata transmitted over a telemetry bus.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x57\x53",  // "WS" - marks a station-registration frame
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct StationRegistration {
    // Key 0x01: 4-byte big-endian serial number (fixed-width decoder)
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    serial: u32,

    // Key 0x02: variable-length UTF-8 region name.
    // `varlen = true` tells the macro the decoder is length-parameterised:
    //   fn(len: usize) -> impl Fn(&mut &[u8]) -> Result<String>
    #[klv(key = 0x02, varlen = true,
          dec = tinyklv::dec::binary::to_string_utf8,
          enc = tinyklv::enc::string::from_string_utf8)]
    region_name: String,

    // Key 0x03: variable-length UTF-8 station identifier
    #[klv(key = 0x03, varlen = true,
          dec = tinyklv::dec::binary::to_string_utf8,
          enc = tinyklv::enc::string::from_string_utf8)]
    station_id: String,
}

fn main() {
    let original = StationRegistration {
        serial: 0x00_AB_CD_12,
        region_name: String::from("North-Atlantic"),
        station_id: String::from("WX-042"),
    };

    println!(
        "Original: serial={:#010X}, region={:?}, id={:?}",
        original.serial, original.region_name, original.station_id
    );

    // encode_value emits the three key-length-value triples (no outer sentinel)
    let encoded = original.encode_value();
    println!("Encoded ({} bytes): {:02X?}", encoded.len(), encoded);

    let decoded = StationRegistration::decode_value(&mut encoded.as_slice()).unwrap();
    println!(
        "Decoded: serial={:#010X}, region={:?}, id={:?}",
        decoded.serial, decoded.region_name, decoded.station_id
    );

    assert_eq!(decoded, original);
    println!("SUCCESS");
}
