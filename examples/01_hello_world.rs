#![allow(clippy::unwrap_used)]
//! Minimal KLV encode/decode roundtrip for a 2-field telemetry struct.
//!
//! This example shows the simplest possible use of `#[derive(Klv)]`: a struct
//! with a `u8` sequence-number field and a `u16` temperature reading. It
//! demonstrates `encode_frame` (sentinel + length + value bytes) and the
//! complementary `decode_frame` that seeks the sentinel and recovers the
//! struct. After running you should see the raw bytes printed alongside the
//! decoded values, confirming a perfect roundtrip.
use tinyklv::prelude::*;
use tinyklv::Klv;

/// A minimal sensor-heartbeat packet: one byte sequence number, one u16
/// temperature reading in 0.01 °C units.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    // sentinel = the magic bytes that mark the start of this packet type
    sentinel = b"\x47\x48",
    // key codec: 1-byte unsigned key
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    // len codec: 1-byte unsigned length
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct HeartbeatPacket {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = &tinyklv::enc::binary::u8)]
    /// Tag 0x01 carries the sequence counter (u8)
    sequence: u8,

    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    /// Tag 0x02 carries temperature as big-endian u16
    temperature_centideg: u16,
}

fn main() {
    let original = HeartbeatPacket {
        sequence: 42,
        temperature_centideg: 2350, // 23.50 °C
    };

    // encode_frame prepends the sentinel 0x47 0x48 and a 1-byte length,
    // then emits key-length-value triples for each field.
    let frame = original.encode_frame();
    println!("Encoded frame ({} bytes): {:02X?}", frame.len(), frame);

    // decode_frame seeks the sentinel in the buffer, reads the length,
    // subslices, and then runs decode_value on the inner bytes.
    let decoded = HeartbeatPacket::decode_frame(&mut frame.as_slice()).unwrap();
    println!(
        "Decoded: sequence={}, temperature={}(x0.01°C)",
        decoded.sequence, decoded.temperature_centideg
    );

    assert_eq!(decoded, original);
    println!("SUCCESS");
}
