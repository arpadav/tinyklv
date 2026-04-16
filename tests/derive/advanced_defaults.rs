//! Tests for `default(typ=...)` container attribute and `init=` field attribute
//!
//! Covers container-level type defaults (eliminating per-field dec/enc),
//! field-level `init` values used when a key is absent, init override when
//! the key is present, and non-KLV fields falling back to `Default::default()`.
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// DefaultTyped - container-level default dec/enc for Color and Priority
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    default(typ = Color,    dec = decode_color,    enc = encode_color),
    default(typ = Priority, dec = decode_priority, enc = encode_priority),
)]
struct DefaultTyped {
    // No per-field dec/enc - resolved from container defaults
    #[klv(key = 0x01)]
    color: Color,
    #[klv(key = 0x02)]
    priority: Priority,
    // Explicit dec/enc still works alongside container defaults
    #[klv(key = 0x03, dec = decode_velocity, enc = encode_velocity)]
    velocity: Velocity,
}

// --------------------------------------------------
// InitColor - Color field with `init = Color::Red`
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct InitColor {
    #[klv(key = 0x01, dec = decode_color, enc = encode_color, init = Color::Red)]
    color: Color,
}

// --------------------------------------------------
// InitTimestamp - Timestamp field with `init = Timestamp { seconds: 0, nanos: 0 }`
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct InitTimestamp {
    #[klv(
        key = 0x01,
        dec = decode_timestamp,
        enc = encode_timestamp,
        init = Timestamp { seconds: 0, nanos: 0 }
    )]
    timestamp: Timestamp,
}

// --------------------------------------------------
// WithNonKlvField - klv Color + non-klv counter
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    allow_unimplemented_encode,
)]
struct WithNonKlvField {
    #[klv(key = 0x01, dec = decode_color, enc = encode_color)]
    color: Color,
    // No #[klv] - should resolve to Default::default() = 0
    counter: u32,
}

// --------------------------------------------------
// helpers - hand-craft TLV bytes for assertions
// --------------------------------------------------

fn tlv(key: u8, value: Vec<u8>) -> Vec<u8> {
    let mut out = vec![key, value.len() as u8];
    out.extend(value);
    out
}

// --------------------------------------------------
// tests: default(typ = ...)
// --------------------------------------------------

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
    stream.extend(tlv(0x01, encode_color(&original.color)));
    stream.extend(tlv(0x02, encode_priority(&original.priority)));
    stream.extend(tlv(0x03, encode_velocity(&original.velocity)));

    let decoded = DefaultTyped::decode(&mut stream.as_slice()).unwrap();
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
    let decoded = DefaultTyped::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original, "roundtrip via container-level defaults");
}

// --------------------------------------------------
// tests: init= on enum field
// --------------------------------------------------

#[test]
fn init_color_enum_absent() {
    // Stream has no key 0x01 - init value Color::Red should be used
    let result = InitColor::decode(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.color,
        Color::Red,
        "absent key should yield init value"
    );
}

#[test]
fn init_color_enum_present() {
    // Stream has key 0x01 = Color::Blue - decoded value overrides init
    let stream = tlv(0x01, encode_color(&Color::Blue));
    let result = InitColor::decode(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.color,
        Color::Blue,
        "present key should yield decoded value"
    );
}

// --------------------------------------------------
// tests: init= on struct field
// --------------------------------------------------

#[test]
fn init_timestamp_struct_absent() {
    let zero = Timestamp {
        seconds: 0,
        nanos: 0,
    };
    let result = InitTimestamp::decode(&mut [].as_slice()).unwrap();
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
    let stream = tlv(0x01, encode_timestamp(&ts));
    let result = InitTimestamp::decode(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.timestamp, ts,
        "present key should yield decoded Timestamp"
    );
}

// --------------------------------------------------
// tests: init overridden when present
// --------------------------------------------------

#[test]
fn init_overridden_when_present() {
    // init = Color::Red, but stream carries Color::Blue
    let stream = tlv(0x01, encode_color(&Color::Blue));
    let result = InitColor::decode(&mut stream.as_slice()).unwrap();
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

// --------------------------------------------------
// tests: non-KLV field uses Default
// --------------------------------------------------

#[test]
fn non_klv_field_default() {
    let stream = tlv(0x01, encode_color(&Color::Green));
    let result = WithNonKlvField::decode(&mut stream.as_slice()).unwrap();
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
    let mut stream: Vec<u8> = tlv(0x01, encode_color(&Color::Alpha));
    stream.extend_from_slice(&[0xFF, 0x02, 0xAB, 0xCD]);

    let result = WithNonKlvField::decode(&mut stream.as_slice()).unwrap();
    assert_eq!(result.color, Color::Alpha);
    assert_eq!(
        result.counter, 0,
        "non-KLV counter must not be affected by unknown keys"
    );
}
