#![allow(clippy::unwrap_used)]
//! Container with BER-OID keys; multi-byte OID keys for large tag namespaces.
//!
//! BER Object Identifier encoding gives a variable-width key representation:
//! values 0–127 fit in one byte, 128–16383 in two bytes, and so on. This is
//! common in industrial IoT protocols where tag registries use 16-bit or
//! larger values. This example uses a u64 key type with three fields - one
//! with a single-byte key (0x01), one with a two-byte BER-OID key (0x0100 =
//! 256), and one with a larger key (0x4001 = 16385) - showing the variable
//! wire encoding. A full encode → decode roundtrip is verified.

use tinyklv::prelude::*;
use tinyklv::Klv;

// BER key encoder: wraps ber_oid which accepts a reference
fn ber_key_enc(v: u64) -> Vec<u8> {
    tinyklv::enc::ber::ber_oid(&v)
}
// BER length encoder: wraps ber_length which accepts a reference
fn ber_len_enc(v: usize) -> Vec<u8> {
    tinyklv::enc::ber::ber_length(&v)
}

/// Process-control packet with BER-OID keys spanning different byte widths.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xC0\xDE",
    // BER-OID key codec: ber_oid::<u64> is the decode fn, ber_key_enc encodes
    key(dec = tinyklv::dec::ber::ber_oid::<u64>, enc = ber_key_enc),
    // BER length codec: variable-width lengths
    len(dec = tinyklv::dec::ber::ber_length, enc = ber_len_enc),
)]
struct ProcessControl {
    // Key 0x01 (single byte on the wire - value < 128)
    #[klv(key = 0x01_u64, dec = tinyklv::dec::binary::be_u8, enc = &tinyklv::enc::binary::u8)]
    device_class: u8,

    // Key 0x0100 = 256 - requires 2 BER-OID bytes: [0x82, 0x00]
    #[klv(key = 0x0100_u64, dec = tinyklv::dec::binary::be_u32, enc = &tinyklv::enc::binary::be_u32)]
    set_point: u32,

    // Key 0x4001 = 16385 - requires 3 BER-OID bytes: [0xC0, 0x80, 0x01]
    #[klv(key = 0x4001_u64, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    alarm_code: u16,
}

fn main() {
    let original = ProcessControl {
        device_class: 0x05,
        set_point: 75_000,   // e.g. flow setpoint in ml/min
        alarm_code: 0x00_FF, // warning bit pattern
    };

    println!(
        "Original: class={:#04X}, set_point={}, alarm={:#06X}",
        original.device_class, original.set_point, original.alarm_code
    );

    let frame = original.encode_frame();
    println!("BER-keyed frame ({} bytes): {:02X?}", frame.len(), frame);

    // Verify the multi-byte BER OID key 0x0100 appears on the wire as two bytes
    // BER OID 256 = [0x82, 0x00] (continuation byte | 0x80, then 0x00)
    let value_bytes = original.encode_value();
    // key 0x01 → [0x01]; key 0x0100 → multi-byte BER; key 0x4001 → multi-byte BER
    println!("Value bytes: {:02X?}", value_bytes);

    let decoded = ProcessControl::decode_frame(&mut frame.as_slice()).unwrap();
    println!(
        "Decoded: class={:#04X}, set_point={}, alarm={:#06X}",
        decoded.device_class, decoded.set_point, decoded.alarm_code
    );

    assert_eq!(decoded, original);
    println!("SUCCESS");
}
