#![allow(clippy::unwrap_used)]
//! BER-length for the frame length prefix; encode/decode large and small
//! payloads to show variable-width length encoding in action.
//!
//! BER lengths below 128 bytes fit in a single octet; lengths 128–255 require
//! two bytes (marker 0x81 + length); lengths up to 65535 require three bytes.
//! Using BER for the outer frame length means a single codec handles all
//! payload sizes without changing the struct definition. This example encodes
//! a small payload (< 128 bytes) and a large payload (> 128 bytes string field)
//! and verifies that the length prefix byte count differs between the two
//! cases, confirming BER variable-width behaviour.

use tinyklv::prelude::*;
use tinyklv::Klv;

fn ber_key_enc(v: u64) -> Vec<u8> {
    tinyklv::enc::ber::ber_oid(&v)
}
fn ber_len_enc(v: usize) -> Vec<u8> {
    tinyklv::enc::ber::ber_length(&v)
}

/// Sensor log entry with a variable-length annotation string.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xBE\xEA",
    // BER-OID keys (single-byte for keys < 128)
    key(dec = tinyklv::dec::ber::ber_oid::<u64>, enc = ber_key_enc),
    // BER-length prefix: single byte for small payloads, multi-byte for large
    len(dec = tinyklv::dec::ber::ber_length, enc = ber_len_enc),
)]
struct SensorLog {
    // Timestamp field using a named encoder function (required by the macro)
    #[klv(key = 0x01_u64, dec = tinyklv::dec::binary::be_u32, enc = &tinyklv::enc::binary::be_u32)]
    timestamp_s: u32,

    // Variable-length UTF-8 annotation; length encoded by the field's own len
    #[klv(key = 0x02_u64, varlen = true,
          dec = tinyklv::dec::binary::to_string_utf8,
          enc = &tinyklv::enc::string::from_string_utf8)]
    annotation: String,
}

fn main() {
    // --- Small payload (< 128 bytes total) -----------------------------------
    let small = SensorLog {
        timestamp_s: 1_700_000_000,
        annotation: String::from("OK"), // 2-byte value
    };

    let small_frame = small.encode_frame();
    println!(
        "Small frame ({} bytes): {:02X?}",
        small_frame.len(),
        small_frame
    );

    // sentinel is 2 bytes; next byte is the outer BER length
    // For a small payload the length fits in 1 byte (< 128)
    let outer_len_byte = small_frame[2];
    assert!(
        outer_len_byte < 0x80,
        "small payload: BER length must be single-byte (< 0x80), got {:#04X}",
        outer_len_byte
    );

    let dec_small = SensorLog::decode_frame(&mut small_frame.as_slice()).unwrap();
    assert_eq!(dec_small, small);
    println!("Small roundtrip: OK");

    // --- Large payload (> 128 bytes annotation) ------------------------------
    let large_annotation = "X".repeat(200); // 200-byte string forces multi-byte BER length
    let large = SensorLog {
        timestamp_s: 1_700_000_001,
        annotation: large_annotation,
    };

    let large_frame = large.encode_frame();
    println!(
        "Large frame ({} bytes), outer-len prefix: {:02X?}",
        large_frame.len(),
        &large_frame[2..4]
    );

    // For a payload > 127 bytes the outer BER length byte has MSB set (0x81+)
    let large_len_marker = large_frame[2];
    assert!(
        large_len_marker >= 0x80,
        "large payload: BER length marker must have MSB set, got {:#04X}",
        large_len_marker
    );

    let dec_large = SensorLog::decode_frame(&mut large_frame.as_slice()).unwrap();
    assert_eq!(dec_large, large);
    println!("Large roundtrip: OK");

    println!("SUCCESS");
}
