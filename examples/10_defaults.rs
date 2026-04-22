#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 10 - container `default(typ=...)` + field-level `default`.
//!
//! Two ergonomics features compose cleanly on one struct:
//!
//! * **Container `default(typ=..., dec=..., enc=...)`** installs codecs for
//!   every field of a given type, so fields of that type only need a
//!   `key = ...` attribute.
//! * **Field-level `default = expr`** supplies a fallback value used when the
//!   decode loop finishes without ever seeing that key - essential for
//!   forward-compatible protocols where newer fields may be absent in older
//!   stream recordings.
//!
//! Showcases:
//! * Hand-written `DecodeValue` / `EncodeValue` impls on two custom enums
//! * `default(typ = ...)` removing boilerplate from field attributes
//! * `default = 1_u16` giving a decode-time fallback for an absent key
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Debug, Clone, Copy, PartialEq)]
/// Three-level signal strength indicator stored as one byte on the wire
enum SignalStrength {
    None,
    Weak,
    Good,
    Excellent,
}

impl DecodeValue<&[u8]> for SignalStrength {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        match decb::u8(input)? {
            0 => Ok(SignalStrength::None),
            1 => Ok(SignalStrength::Weak),
            2 => Ok(SignalStrength::Good),
            3 => Ok(SignalStrength::Excellent),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}

impl EncodeValue<Vec<u8>> for SignalStrength {
    fn encode_value(&self) -> Vec<u8> {
        encb::u8(match self {
            SignalStrength::None      => 0,
            SignalStrength::Weak      => 1,
            SignalStrength::Good      => 2,
            SignalStrength::Excellent => 3,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Network transport mode stored as one byte on the wire
enum NetworkMode {
    Offline,
    WiFi,
    Cellular,
    Satellite,
}

impl DecodeValue<&[u8]> for NetworkMode {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        match decb::u8(input)? {
            0 => Ok(NetworkMode::Offline),
            1 => Ok(NetworkMode::WiFi),
            2 => Ok(NetworkMode::Cellular),
            3 => Ok(NetworkMode::Satellite),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}

impl EncodeValue<Vec<u8>> for NetworkMode {
    fn encode_value(&self) -> Vec<u8> {
        encb::u8(match self {
            NetworkMode::Offline   => 0,
            NetworkMode::WiFi      => 1,
            NetworkMode::Cellular  => 2,
            NetworkMode::Satellite => 3,
        })
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(
        typ = SignalStrength,
        dec = SignalStrength::decode_value,
        enc = SignalStrength::encode_value,
    ),
    default(
        typ = NetworkMode,
        dec = NetworkMode::decode_value,
        enc = NetworkMode::encode_value,
    ),
)]
/// Radio telemetry config. Enum fields use container defaults; `channel`
/// uses an explicit codec plus `default = 1` for when the key is absent
struct RadioConfig {
    #[klv(key = 0x01)]
    /// Uplink signal quality (resolved via container default)
    uplink_signal: SignalStrength,

    #[klv(key = 0x02)]
    /// Downlink signal quality (resolved via container default)
    downlink_signal: SignalStrength,

    #[klv(key = 0x03)]
    /// Active network transport (resolved via container default)
    mode: NetworkMode,

    #[klv(
        key = 0x04,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        default = 1_u16,
    )]
    /// Radio channel number; defaults to 1 when the key is absent
    channel: u16,
}

fn main() {
    // build - a complete config touching every field
    let original = RadioConfig {
        uplink_signal:   SignalStrength::Good,
        downlink_signal: SignalStrength::Excellent,
        mode:            NetworkMode::Satellite,
        channel:         14,
    };

    // encode + decode - full round-trip through the container defaults
    let encoded = original.encode_value();
    let decoded = RadioConfig::decode_value(
        &mut encoded.as_slice(),
    ).unwrap();
    assert_eq!(decoded, original);

    // build a partial stream by hand, deliberately omitting key 0x04 so
    // that the `default = 1_u16` fallback is exercised on decode
    let partial = [
        // key 0x01, len=1, SignalStrength::Weak:
            0x01, 0x01, 0x01,
        // key 0x02, len=1, SignalStrength::Good:
            0x02, 0x01, 0x02,
        // key 0x03, len=1, NetworkMode::WiFi:
            0x03, 0x01, 0x01,
        // key 0x04 deliberately absent - decoder must fall back to default
    ];

    // decode - the decoder applies the default value for the missing channel key
    let dec_partial = RadioConfig::decode_value(
        &mut partial.as_slice(),
    ).unwrap();

    // assert - present keys carry their real values, channel falls back to 1
    assert_eq!(dec_partial.uplink_signal,   SignalStrength::Weak);
    assert_eq!(dec_partial.downlink_signal, SignalStrength::Good);
    assert_eq!(dec_partial.mode,            NetworkMode::WiFi);
    assert_eq!(dec_partial.channel, 1, "absent key yields default value");
}
