#![allow(clippy::unwrap_used)]
//! Container-level `default(typ=...)` for per-type defaults + field-level
//! `init = expr` to supply a fallback value when a key is absent from the
//! stream. Container defaults eliminate the need to repeat `dec =`/`enc =`
//! on every field of the same type. Field-level `init` acts like a
//! `Default::default()` escape hatch: if the key never appears in the byte
//! stream the field takes the init value instead of causing a parse error.
//! This pattern is essential for forward-compatible protocols where newer
//! fields may be absent in older stream recordings.

use tinyklv::prelude::*;
use tinyklv::Klv;

// --- Domain types -----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum SignalStrength {
    None,
    Weak,
    Good,
    Excellent,
}

impl tinyklv::DecodeValue<&[u8]> for SignalStrength {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        match tinyklv::dec::binary::be_u8(input)? {
            0 => Ok(SignalStrength::None),
            1 => Ok(SignalStrength::Weak),
            2 => Ok(SignalStrength::Good),
            3 => Ok(SignalStrength::Excellent),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for SignalStrength {
    fn encode_value(&self) -> Vec<u8> {
        tinyklv::enc::binary::u8(match self {
            SignalStrength::None => 0,
            SignalStrength::Weak => 1,
            SignalStrength::Good => 2,
            SignalStrength::Excellent => 3,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NetworkMode {
    Offline,
    WiFi,
    Cellular,
    Satellite,
}

impl tinyklv::DecodeValue<&[u8]> for NetworkMode {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        match tinyklv::dec::binary::be_u8(input)? {
            0 => Ok(NetworkMode::Offline),
            1 => Ok(NetworkMode::WiFi),
            2 => Ok(NetworkMode::Cellular),
            3 => Ok(NetworkMode::Satellite),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}
impl tinyklv::EncodeValue<Vec<u8>> for NetworkMode {
    fn encode_value(&self) -> Vec<u8> {
        tinyklv::enc::binary::u8(match self {
            NetworkMode::Offline => 0,
            NetworkMode::WiFi => 1,
            NetworkMode::Cellular => 2,
            NetworkMode::Satellite => 3,
        })
    }
}

fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}

// --- Struct using container-level defaults and field-level init -------------

/// Radio telemetry config. Both enum fields share codec defaults at container
/// level so individual fields only need a `key = ...` attribute.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    // Container-level defaults: any field of type SignalStrength gets these codecs
    default(typ = SignalStrength,
            dec = SignalStrength::decode_value,
            enc = SignalStrength::encode_value),
    // Similarly for NetworkMode
    default(typ = NetworkMode,
            dec = NetworkMode::decode_value,
            enc = NetworkMode::encode_value),
)]
struct RadioConfig {
    // Resolves dec/enc from container default for SignalStrength
    #[klv(key = 0x01)]
    uplink_signal: SignalStrength,

    // Also resolved from container default
    #[klv(key = 0x02)]
    downlink_signal: SignalStrength,

    // Container default for NetworkMode
    #[klv(key = 0x03)]
    mode: NetworkMode,

    // Explicit codec + init fallback: if key 0x04 is absent, channel = 1
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_u16, enc = tinyklv::enc::binary::be_u16, init = 1_u16)]
    channel: u16,
}

fn main() {
    // --- Case 1: full roundtrip via container defaults ----------------------
    let original = RadioConfig {
        uplink_signal: SignalStrength::Good,
        downlink_signal: SignalStrength::Excellent,
        mode: NetworkMode::Satellite,
        channel: 14,
    };

    let enc = original.encode_value();
    println!("Full encoded ({} bytes): {:02X?}", enc.len(), enc);
    let dec = RadioConfig::decode_value(&mut enc.as_slice()).unwrap();
    assert_eq!(dec, original);
    println!("Full roundtrip: OK");

    // --- Case 2: channel key absent → init value (1) is used ---------------
    // Build stream manually with only the three enum fields
    let partial_enc: Vec<u8> = {
        let mut v = Vec::new();
        // key=0x01, len=1, SignalStrength::Weak
        v.extend_from_slice(&[0x01, 0x01, 1]);
        // key=0x02, len=1, SignalStrength::Good
        v.extend_from_slice(&[0x02, 0x01, 2]);
        // key=0x03, len=1, NetworkMode::WiFi
        v.extend_from_slice(&[0x03, 0x01, 1]);
        // key 0x04 deliberately omitted
        v
    };
    let dec_partial = RadioConfig::decode_value(&mut partial_enc.as_slice()).unwrap();
    println!(
        "Partial decoded: uplink={:?}, channel={}",
        dec_partial.uplink_signal, dec_partial.channel
    );

    // channel must be the init value because key 0x04 was absent
    assert_eq!(
        dec_partial.channel, 1,
        "absent key should yield init value 1"
    );
    assert_eq!(dec_partial.uplink_signal, SignalStrength::Weak);
    assert_eq!(dec_partial.mode, NetworkMode::WiFi);

    println!("SUCCESS");
}
