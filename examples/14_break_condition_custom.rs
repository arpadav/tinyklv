#![allow(clippy::unwrap_used)]
//! Custom break-condition skip logic embedded in a manual DecodeValue impl.
//!
//! The `BreakConditionType::Skip` pattern is demonstrated by writing a manual
//! `DecodeValue` implementation for a sensor-reading struct. Key 0xFE is a
//! reserved/deprecated tag whose value bytes must be consumed and discarded
//! without error, letting the decode loop continue to find the valid fields.
//! This is the canonical approach because the blanket `BreakCondition` impl
//! covers all `DecodeValue` types, so you embed the skip logic directly in
//! the manual decode loop where you have the concrete key type in hand.

use tinyklv::prelude::*;
use tinyklv::Klv;

// --- A struct we will encode WITH the derive macro -------------------------

/// Simple sensor frame used on the encode side.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xF0\xF1",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct SensorFrame {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    /// Key 0x01: temperature in 0.01 °C units
    temperature_centideg: u16,

    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = &tinyklv::enc::binary::be_u32)]
    /// Key 0x02: CO₂ concentration in ppm
    co2_ppm: u32,
}

// --- A manual decoder that skips key 0xFE --------------------------------

/// Same logical payload but decoded with a hand-written loop that silently
/// skips reserved key 0xFE, demonstrating the Skip break-condition pattern.
#[derive(Debug, PartialEq)]
struct SensorReading {
    temperature_centideg: u16,
    co2_ppm: u32,
}

impl tinyklv::DecodeValue<&[u8]> for SensorReading {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut temperature: Option<u16> = None;
        let mut co2: Option<u32> = None;

        loop {
            // Try to read the next key; clean EOF → stop looping
            let key = match tinyklv::dec::binary::be_u8(input) {
                Ok(k) => k,
                Err(_) => break,
            };
            let len = match tinyklv::dec::binary::be_u8_as_usize(input) {
                Ok(l) => l,
                Err(_) => break,
            };

            // Key 0xFE is reserved - consume its value bytes and continue
            if key == 0xFE {
                // Skip: consume exactly `len` bytes without decoding them
                let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                continue;
            }

            match key {
                0x01 => {
                    temperature = tinyklv::dec::binary::be_u16(input).ok().or(temperature);
                }
                0x02 => {
                    co2 = tinyklv::dec::binary::be_u32(input).ok().or(co2);
                }
                _ => {
                    // Unknown key: consume and skip
                    let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                }
            }
        }

        // Both fields are required - return Err if either is missing
        match (temperature, co2) {
            (Some(t), Some(c)) => Ok(SensorReading {
                temperature_centideg: t,
                co2_ppm: c,
            }),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}

fn main() {
    // Build a byte stream that interleaves the reserved key 0xFE with
    // two valid fields in arbitrary order
    let mut stream: Vec<u8> = Vec::new();

    // Valid field: key=0x01, len=2, temperature=2350 (23.50 °C)
    stream.extend_from_slice(&[0x01, 0x02]);
    stream.extend_from_slice(&2_350_u16.to_be_bytes());

    // Reserved field: key=0xFE, len=4, garbage - must be skipped silently
    stream.extend_from_slice(&[0xFE, 0x04, 0xDE, 0xAD, 0xBE, 0xEF]);

    // Valid field: key=0x02, len=4, CO2=412 ppm
    stream.extend_from_slice(&[0x02, 0x04]);
    stream.extend_from_slice(&412_u32.to_be_bytes());

    println!("Stream with reserved key 0xFE: {:02X?}", stream);

    let decoded = SensorReading::decode_value(&mut stream.as_slice()).unwrap();
    println!(
        "Decoded: temperature={}(x0.01°C), co2={}ppm",
        decoded.temperature_centideg, decoded.co2_ppm
    );

    assert_eq!(
        decoded.temperature_centideg, 2_350,
        "reserved key 0xFE must not corrupt temperature"
    );
    assert_eq!(
        decoded.co2_ppm, 412,
        "CO2 field must decode correctly after skip"
    );

    // Verify the derive-encoded form also decodes correctly via the manual decoder
    let original = SensorFrame {
        temperature_centideg: 9_999,
        co2_ppm: 800,
    };
    let enc = original.encode_value();
    let dec = SensorReading::decode_value(&mut enc.as_slice()).unwrap();
    assert_eq!(dec.temperature_centideg, original.temperature_centideg);
    assert_eq!(dec.co2_ppm, original.co2_ppm);

    println!("SUCCESS");
}
