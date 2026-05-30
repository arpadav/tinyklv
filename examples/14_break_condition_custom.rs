#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 14 - custom loop control via a hand-written `DecodeValue`
//!
//! Most decoders that need non-standard loop behaviour - silently skip a
//! reserved/deprecated tag, stop on a terminator key, or abort on tamper
//! detection - should reach for the `#[klv(break_on = ..)]` container
//! attribute (see `book_14_break_condition.rs`). This example shows the
//! lower-level escape hatch for cases the attribute does not cover: drop the
//! derive on the decode side and write a manual `DecodeValue` impl that embeds
//! the loop-control match directly. The encode side can still use
//! `#[derive(Klv)]` on a mirror type
//!
//! This example ships with a `SensorFrame` encoder (derived) and a
//! `SensorReading` decoder (manual) that silently skips the reserved key
//! `0xFE` so the loop can continue and find the real fields behind it
//!
//! Showcases:
//! * Hand-written `DecodeValue` impl living next to a derived encoder
//! * `Skip` semantics: consume `len` bytes and continue the loop
//! * Junk prefix bytes before the fields to stress the skip path
//!
//! See also: book Tutorial 14
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"SENSORFRAME",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Encode-side mirror: used only to produce bytes for round-trip tests
struct SensorFrame {
    /// Temperature in 0.01 C units
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temperature_centideg: u16,

    /// CO2 concentration in ppm
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    co2_ppm: u32,
}

#[derive(Debug, PartialEq)]
/// Decode-side form with a hand-written loop that honours a "Skip" rule on
/// the reserved key 0xFE
struct SensorReading {
    temperature_centideg: u16,
    co2_ppm:              u32,
}

/// Classify a key: `0xFE` is a reserved/deprecated tag whose value bytes
/// must be consumed and silently discarded
fn is_reserved(key: u8) -> bool {
    key == 0xFE
}

impl DecodeValue<&[u8]> for SensorReading {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut temperature: Option<u16> = None;
        let mut co2:         Option<u32> = None;

        loop {
            // clean EOF while reading the next key ends the loop
            let key = match decb::u8(input) {
                Ok(k)  => k,
                Err(_) => break,
            };
            let len = match decb::u8_as_usize(input) {
                Ok(l)  => l,
                Err(_) => break,
            };

            // Skip: consume exactly `len` bytes of value and continue
            if is_reserved(key) {
                let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                continue;
            }

            match key {
                0x01 => temperature = Some(decb::be_u16(input)?),
                0x02 => co2         = Some(decb::be_u32(input)?),
                _ => {
                    // unknown key - consume and drop
                    let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                }
            }
        }

        match (temperature, co2) {
            (Some(t), Some(c)) => Ok(SensorReading {
                temperature_centideg: t,
                co2_ppm:              c,
            }),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}

fn main() {
    // build a stream by hand with the reserved key 0xFE interleaved between
    // the two valid fields
    let mut stream: Vec<u8> = Vec::new();

    // valid temperature field:
    //   key = 0x01, len = 2, value = 2350 (23.50 C, big-endian u16)
    stream.extend_from_slice(&[
        // key + length:
            0x01, 0x02,
        // big-endian u16 payload (2350 -> 0x092E):
            0x09, 0x2E,
    ]);

    // reserved triple that must be silently skipped:
    //   key = 0xFE, len = 4, value = four garbage bytes
    stream.extend_from_slice(&[
        // reserved key + length:
            0xFE, 0x04,
        // junk value:
            0xDE, 0xAD, 0xBE, 0xEF,
    ]);

    // valid CO2 field:
    //   key = 0x02, len = 4, value = 412 ppm (big-endian u32)
    stream.extend_from_slice(&[
        // key + length:
            0x02, 0x04,
        // big-endian u32 payload (412 -> 0x0000019C):
            0x00, 0x00, 0x01, 0x9C,
    ]);

    // decode - the manual loop skips the reserved key and keeps going
    let decoded = SensorReading::decode_value(
        &mut stream.as_slice(),
    ).unwrap();

    // assert - the reserved key did not corrupt the two real fields
    assert_eq!(decoded.temperature_centideg, 2_350);
    assert_eq!(decoded.co2_ppm,                412);

    // bonus - the same hand-written decoder accepts bytes produced by the
    // derive-generated encoder on the mirror type
    let original = SensorFrame {
        temperature_centideg: 9_999,
        co2_ppm:                800,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let via_derive = SensorReading::decode_value(
        &mut encoded.as_slice(),
    ).unwrap();
    assert_eq!(via_derive.temperature_centideg, original.temperature_centideg);
    assert_eq!(via_derive.co2_ppm,              original.co2_ppm);
}
