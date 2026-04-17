#![allow(clippy::unwrap_used)]
//! BER-length, BER-key container; encode_value vs encode_frame comparison.
//!
//! Basic Encoding Rules (BER) are used in many real-world protocols because
//! they give a compact, variable-width representation for both keys and
//! lengths. This example builds a two-field struct that uses BER OID keys and
//! BER lengths, then shows the numeric difference between `encode_value`
//! (field triples only) and `encode_frame` (sentinel + outer-length +
//! field triples). The byte-layout annotations in the output make it easy to
//! see exactly where the extra bytes come from.

use tinyklv::prelude::*;
use tinyklv::Klv;

fn ber_key_enc(v: u64) -> Vec<u8> {
    tinyklv::enc::ber::ber_oid(&v)
}
fn ber_len_enc(v: usize) -> Vec<u8> {
    tinyklv::enc::ber::ber_length(&v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}
fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}

/// Atmospheric sensor packet using BER-encoded keys and lengths.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    // sentinel bytes that identify this packet type on the wire
    sentinel = b"\xA5\x5A",
    // BER OID key codec - keys < 128 are single bytes; larger keys use more
    key(dec = tinyklv::dec::ber::ber_oid::<u64>, enc = ber_key_enc),
    // BER length codec - lengths < 128 are single bytes
    len(dec = tinyklv::dec::ber::ber_length,      enc = ber_len_enc),
)]
struct AtmoSensor {
    // Key 0x01: pressure in Pascal (u32, big-endian)
    #[klv(key = 0x01_u64, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    pressure_pa: u32,

    // Key 0x02: temperature in 0.01 °C units (u16, big-endian)
    #[klv(key = 0x02_u64, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    temperature_centideg: u16,
}

fn main() {
    let original = AtmoSensor {
        pressure_pa: 101_325,        // standard atmosphere
        temperature_centideg: 2_050, // 20.50 °C
    };

    // encode_value: only the field KLV triples, no outer wrapper
    let value_bytes = original.encode_value();
    println!(
        "encode_value ({} bytes): {:02X?}",
        value_bytes.len(),
        value_bytes
    );

    // encode_frame: sentinel + BER-length + field triples
    let frame_bytes = original.encode_frame();
    println!(
        "encode_frame ({} bytes): {:02X?}",
        frame_bytes.len(),
        frame_bytes
    );

    // The frame must start with the sentinel and be longer than the value
    assert_eq!(
        &frame_bytes[0..2],
        b"\xA5\x5A",
        "frame must begin with sentinel"
    );
    assert!(
        frame_bytes.len() > value_bytes.len(),
        "frame includes sentinel + length bytes"
    );

    // decode_frame recovers the struct from the framed bytes
    let decoded = AtmoSensor::decode_frame(&mut frame_bytes.as_slice()).unwrap();
    println!(
        "Decoded: pressure={}Pa, temperature={}(x0.01°C)",
        decoded.pressure_pa, decoded.temperature_centideg
    );

    assert_eq!(decoded, original);
    println!("SUCCESS");
}
