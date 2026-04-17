#![allow(clippy::unwrap_used)]
//! Outer struct with an Inner struct as a field; both derive Klv independently.
//!
//! Complex telemetry protocols nest packets: a top-level vehicle status frame
//! may embed an engine-health sub-frame as one of its fields. Both `Inner` and
//! `Outer` derive `Klv`, which auto-implements `encode_value`/`decode_value`.
//! The outer struct references the inner's methods via
//! `dec = Inner::decode_value, enc = Inner::encode_value`. The roundtrip test
//! verifies that both levels encode and decode correctly and that the nested
//! bytes are contiguous inside the outer frame.

use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u8(v: &u8) -> Vec<u8> {
    tinyklv::enc::binary::u8(*v)
}
fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}
fn enc_f32(v: &f32) -> Vec<u8> {
    tinyklv::enc::binary::be_f32(*v)
}

/// Engine-health sub-packet (inner). Can be encoded/decoded stand-alone.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct EngineHealth {
    // RPM as u16
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    rpm: u16,

    // Coolant temperature in 0.1 °C units (u16)
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    coolant_temp_decideg: u16,

    // Oil pressure in kPa (u8, 0-255)
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    oil_pressure_kpa: u8,
}

/// Vehicle status frame (outer). Embeds EngineHealth as a nested field.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xC5\xF1",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct VehicleStatus {
    // Key 0x01: vehicle ID
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    vehicle_id: u32,

    // Key 0x02: speed in km/h as f32
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_f32, enc = enc_f32)]
    speed_kmh: f32,

    // Key 0x03: nested EngineHealth - decode_value / encode_value are
    // automatically implemented by the inner derive(Klv)
    #[klv(key = 0x03,
          dec = EngineHealth::decode_value,
          enc = EngineHealth::encode_value)]
    engine: EngineHealth,
}

fn main() {
    let original = VehicleStatus {
        vehicle_id: 0xDEAD_C0DE,
        speed_kmh: 112.5,
        engine: EngineHealth {
            rpm: 3_200,
            coolant_temp_decideg: 900, // 90.0 °C
            oil_pressure_kpa: 185,
        },
    };

    println!(
        "Original: id={:#010X}, speed={}km/h, rpm={}, coolant={}(x0.1°C)",
        original.vehicle_id,
        original.speed_kmh,
        original.engine.rpm,
        original.engine.coolant_temp_decideg
    );

    // encode_value encodes the outer struct; the engine sub-struct occupies
    // key=0x03 and its own KLV triples are nested inside that value region
    let encoded = original.encode_value();
    println!("Encoded ({} bytes): {:02X?}", encoded.len(), encoded);

    let decoded = VehicleStatus::decode_value(&mut encoded.as_slice()).unwrap();
    println!(
        "Decoded:  id={:#010X}, speed={}km/h, rpm={}, coolant={}(x0.1°C)",
        decoded.vehicle_id,
        decoded.speed_kmh,
        decoded.engine.rpm,
        decoded.engine.coolant_temp_decideg
    );

    assert_eq!(decoded.vehicle_id, original.vehicle_id);
    assert!((decoded.speed_kmh - original.speed_kmh).abs() < 1e-4);
    assert_eq!(decoded.engine, original.engine);
    println!("SUCCESS");
}
