//! Tests for `default(typ=...)` container attribute and `init=` field attribute
//!
//! Covers container-level type defaults (eliminating per-field dec/enc),
//! field-level `init` values used when a key is absent, init override when
//! the key is present, and non-KLV fields falling back to `Default::default()`.
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    default(typ = Color,    dec = Color::decode_value,    enc = Color::encode_value),
    default(typ = Priority, dec = Priority::decode_value, enc = Priority::encode_value),
)]
struct DefaultTyped {
    // No per-field dec/enc - resolved from container defaults
    #[klv(key = 0x01)]
    color: Color,
    #[klv(key = 0x02)]
    priority: Priority,
    // Explicit dec/enc still works alongside container defaults
    #[klv(key = 0x03, dec = Velocity::decode_value, enc = Velocity::encode_value)]
    velocity: Velocity,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct InitColor {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value, init = Color::Red)]
    color: Color,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct InitTimestamp {
    #[klv(
        key = 0x01,
        dec = Timestamp::decode_value,
        enc = Timestamp::encode_value,
        init = Timestamp { seconds: 0, nanos: 0 }
    )]
    timestamp: Timestamp,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    allow_unimplemented_encode,
)]
struct WithNonKlvField {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
    // No #[klv] - should resolve to Default::default() = 0
    counter: u32,
}

fn tlv(key: u8, value: Vec<u8>) -> Vec<u8> {
    let mut out = vec![key, value.len() as u8];
    out.extend(value);
    out
}

#[test]
fn default_type_color_and_priority() {
    let original = DefaultTyped {
        color: Color::Alpha,
        priority: Priority::Critical,
        velocity: Velocity {
            dx: 10,
            dy: -20,
            dz: 5,
        },
    };
    // Decode from hand-built stream
    let mut stream: Vec<u8> = Vec::new();
    stream.extend(tlv(0x01, original.color.encode_value()));
    stream.extend(tlv(0x02, original.priority.encode_value()));
    stream.extend(tlv(0x03, original.velocity.encode_value()));
    let decoded = DefaultTyped::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(decoded, original, "container-default decode should match");
}

#[test]
fn default_type_roundtrip() {
    let original = DefaultTyped {
        color: Color::Red,
        priority: Priority::Medium,
        velocity: Velocity {
            dx: 0,
            dy: 0,
            dz: -1,
        },
    };
    let encoded = original.encode_value();
    let decoded = DefaultTyped::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original, "roundtrip via container-level defaults");
}

#[test]
fn init_color_enum_absent() {
    // Stream has no key 0x01 - init value Color::Red should be used
    let result = InitColor::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.color,
        Color::Red,
        "absent key should yield init value"
    );
}

#[test]
fn init_color_enum_present() {
    // Stream has key 0x01 = Color::Blue - decoded value overrides init
    let stream = tlv(0x01, Color::Blue.encode_value());
    let result = InitColor::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.color,
        Color::Blue,
        "present key should yield decoded value"
    );
}

#[test]
fn init_timestamp_struct_absent() {
    let zero = Timestamp {
        seconds: 0,
        nanos: 0,
    };
    let result = InitTimestamp::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.timestamp, zero,
        "absent key should yield init Timestamp"
    );
}

#[test]
fn init_timestamp_struct_present() {
    let ts = Timestamp {
        seconds: 1_700_000_000,
        nanos: 12_345,
    };
    let stream = tlv(0x01, ts.encode_value());
    let result = InitTimestamp::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.timestamp, ts,
        "present key should yield decoded Timestamp"
    );
}

#[test]
fn init_overridden_when_present() {
    // init = Color::Red, but stream carries Color::Blue
    let stream = tlv(0x01, Color::Blue.encode_value());
    let result = InitColor::decode_value(&mut stream.as_slice()).unwrap();
    assert_ne!(
        result.color,
        Color::Red,
        "init should be overridden by decoded value"
    );
    assert_eq!(
        result.color,
        Color::Blue,
        "decoded Color::Blue must win over Color::Red init"
    );
}

#[test]
fn non_klv_field_default() {
    let stream = tlv(0x01, Color::Green.encode_value());
    let result = WithNonKlvField::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.color,
        Color::Green,
        "klv field should decode normally"
    );
    assert_eq!(
        result.counter, 0,
        "non-KLV field should be Default::default() = 0"
    );
}

#[test]
fn non_klv_field_unaffected_by_unknown_keys() {
    // Even if stream has unknown keys, the non-KLV field stays at 0
    let mut stream: Vec<u8> = tlv(0x01, Color::Alpha.encode_value());
    stream.extend_from_slice(&[0xFF, 0x02, 0xAB, 0xCD]);

    let result = WithNonKlvField::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(result.color, Color::Alpha);
    assert_eq!(
        result.counter, 0,
        "non-KLV counter must not be affected by unknown keys"
    );
}
