//! ASCII-encoded field tests for `#[derive(Klv)]`
//!
//! Tests the `dec::string` / `enc::string` ASCII codec families wired up
//! through the derive macro on a realistic `AsciiTelemetry` struct that
//! mixes a fixed binary `u16` field with `varlen = true` decimal-integer,
//! signed-integer, float, hex, and alpha-run fields.  Covers a full
//! roundtrip, hand-built packet decoding, and rejection of garbage numerics
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as deca;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as enca;
use tinyklv::prelude::*;

/// A telemetry packet mixing a fixed binary field with several ASCII-text-encoded
/// value fields, exercising the ASCII value decoders/encoders in `codecs::string` end-to-end
/// through the derive macro
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct AsciiTelemetry {
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    id: u16,

    #[klv(key = 0x02, varlen = true, dec = deca::u32, enc = *enca::u32)]
    sequence: u32,

    #[klv(key = 0x03, varlen = true, dec = deca::i32, enc = *enca::i32)]
    offset: i32,

    #[klv(key = 0x04, varlen = true, dec = deca::f64, enc = *enca::f64)]
    altitude: f64,

    #[klv(key = 0x05, varlen = true, dec = deca::hex_u16, enc = *enca::hex_u16)]
    flags: u16,

    #[klv(key = 0x06, varlen = true, dec = deca::alpha, enc = enca::from_string_ascii)]
    callsign: String,
}

#[test]
/// Verifies a full encode/decode roundtrip over the mixed-codec telemetry struct
fn ascii_telemetry_roundtrip() {
    let original = AsciiTelemetry {
        id: 4242,
        sequence: 1_000_000,
        offset: -512,
        altitude: 123.5,
        flags: 0xABCD,
        callsign: String::from("FALCON"),
    };
    let encoded = original.encode_value();
    let decoded = AsciiTelemetry::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Verifies decoding a hand-built packet. Decimal fields must be canonical
/// (winnow's `dec_uint`/`dec_int` reject leading zeros), while the hex field
/// legitimately tolerates leading zeros
fn ascii_telemetry_decode_hand_built() {
    let mut data: Vec<u8> = Vec::new();
    // id = 0x0007 (fixed be_u16)
    data.extend_from_slice(&[0x01, 0x02, 0x00, 0x07]);
    // sequence = "42" -> 42
    data.extend_from_slice(&[0x02, 0x02]);
    data.extend_from_slice(b"42");
    // offset = "-7" -> -7
    data.extend_from_slice(&[0x03, 0x02]);
    data.extend_from_slice(b"-7");
    // altitude = "3.5" -> 3.5
    data.extend_from_slice(&[0x04, 0x03]);
    data.extend_from_slice(b"3.5");
    // flags = "00ff" -> 0x00FF
    data.extend_from_slice(&[0x05, 0x04]);
    data.extend_from_slice(b"00ff");
    // callsign = "ABC"
    data.extend_from_slice(&[0x06, 0x03]);
    data.extend_from_slice(b"ABC");

    let decoded = AsciiTelemetry::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(
        decoded,
        AsciiTelemetry {
            id: 7,
            sequence: 42,
            offset: -7,
            altitude: 3.5,
            flags: 0x00FF,
            callsign: String::from("ABC"),
        }
    );
}

#[test]
/// Verifies a malformed ASCII numeric field (non-digit byte) fails to decode
fn ascii_telemetry_rejects_garbage_number() {
    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&[0x01, 0x02, 0x00, 0x07]);
    // sequence field contains a non-digit byte
    data.extend_from_slice(&[0x02, 0x03]);
    data.extend_from_slice(b"4X2");
    let result = AsciiTelemetry::decode_value(&mut data.as_slice());
    assert!(result.is_err());
}
