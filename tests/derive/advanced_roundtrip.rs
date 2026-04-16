//! Roundtrip, duplicate-key, and Option-encoding tests for `#[derive(Klv)]`
//!
//! Covers last-wins semantics when the same key appears multiple times,
//! None-field omission from the encoded byte stream, and identity roundtrip
//! across several struct shapes (all-required, all-optional-some, mixed).
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// structs
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct SixField {
    #[klv(key = 0x01, dec = decode_color,     enc = encode_color)]
    color: Color,
    #[klv(key = 0x02, dec = decode_priority,  enc = encode_priority)]
    priority: Priority,
    #[klv(key = 0x03, dec = decode_velocity,  enc = encode_velocity)]
    velocity: Velocity,
    #[klv(key = 0x04, dec = decode_coordinate, enc = encode_coordinate)]
    coordinate: Coordinate,
    #[klv(key = 0x05, dec = decode_attitude,  enc = encode_attitude)]
    attitude: Attitude,
    #[klv(key = 0x06, dec = decode_timestamp, enc = encode_timestamp)]
    timestamp: Timestamp,
}

// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct ThreeOptFields {
    #[klv(key = 0x01, dec = decode_color,    enc = encode_color)]
    color: Option<Color>,
    #[klv(key = 0x02, dec = decode_velocity, enc = encode_velocity)]
    velocity: Option<Velocity>,
    #[klv(key = 0x03, dec = decode_coordinate, enc = encode_coordinate)]
    coordinate: Option<Coordinate>,
}

// --------------------------------------------------
// all-required roundtrip shape
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct AllRequired {
    #[klv(key = 0x01, dec = decode_color,     enc = encode_color)]
    color: Color,
    #[klv(key = 0x02, dec = decode_timestamp, enc = encode_timestamp)]
    timestamp: Timestamp,
}

// --------------------------------------------------
// all-optional roundtrip shape
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct AllOptional {
    #[klv(key = 0x01, dec = decode_color,    enc = encode_color)]
    color: Option<Color>,
    #[klv(key = 0x02, dec = decode_velocity, enc = encode_velocity)]
    velocity: Option<Velocity>,
}

// --------------------------------------------------
// mixed required + optional roundtrip shape
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct MixedShape {
    #[klv(key = 0x01, dec = decode_priority, enc = encode_priority)]
    priority: Priority,
    #[klv(key = 0x02, dec = decode_attitude, enc = encode_attitude)]
    attitude: Option<Attitude>,
}

// --------------------------------------------------
// owned-encoder structs (encoder takes T, not &T)
// --------------------------------------------------

/// Encoder that takes Priority by value — tests autoref-deref dispatch
fn encode_priority_owned(v: Priority) -> Vec<u8> {
    encode_priority(&v)
}

/// Encoder that takes Color by value
fn encode_color_owned(v: Color) -> Vec<u8> {
    encode_color(&v)
}

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct OwnedEncoderStruct {
    #[klv(key = 0x01, dec = decode_priority, enc = tinyklv::enc_owned!(encode_priority_owned))]
    priority: Priority,
    #[klv(key = 0x02, dec = decode_color,    enc = tinyklv::enc_owned!(encode_color_owned))]
    color: Color,
}

// --------------------------------------------------
// helpers
// --------------------------------------------------

fn make_six_field_a() -> SixField {
    SixField {
        color: Color::Red,
        priority: Priority::Low,
        velocity: Velocity {
            dx: 1,
            dy: 2,
            dz: 3,
        },
        coordinate: Coordinate { lat: 0.0, lon: 0.0 },
        attitude: Attitude {
            roll: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        },
        timestamp: Timestamp {
            seconds: 100,
            nanos: 0,
        },
    }
}

fn make_six_field_b() -> SixField {
    SixField {
        color: Color::Alpha,
        priority: Priority::Critical,
        velocity: Velocity {
            dx: -100,
            dy: 200,
            dz: -50,
        },
        coordinate: Coordinate {
            lat: 90.0,
            lon: -180.0,
        },
        attitude: Attitude {
            roll: 1.0,
            pitch: -1.0,
            yaw: 0.5,
        },
        timestamp: Timestamp {
            seconds: 999,
            nanos: 999,
        },
    }
}

// --------------------------------------------------
// tests
// --------------------------------------------------

#[test]
fn duplicate_6field_last_wins() {
    // Encode two different SixField values and concatenate - last-wins per key.
    let a = make_six_field_a();
    let b = make_six_field_b();

    let mut stream = a.encode_value();
    stream.extend(b.encode_value());

    let decoded = SixField::decode(&mut stream.as_slice()).unwrap();

    // b's values must win for every field
    assert_eq!(decoded.color, b.color);
    assert_eq!(decoded.priority, b.priority);
    assert_eq!(decoded.velocity, b.velocity);
    assert_eq!(decoded.coordinate, b.coordinate);
    assert_eq!(decoded.attitude, b.attitude);
    assert_eq!(decoded.timestamp, b.timestamp);
}

#[test]
fn encode_skips_none() {
    let val = ThreeOptFields {
        color: Some(Color::Green),
        velocity: None,
        coordinate: Some(Coordinate { lat: 1.0, lon: 2.0 }),
    };

    let encoded = val.encode_value();

    // Color: key(1) + len(1) + 2 bytes = 4
    // Coordinate: key(1) + len(1) + 16 bytes = 18
    // Velocity (None): 0 bytes
    let expected_len = 1 + 1 + 2 + 1 + 1 + 16;
    assert_eq!(
        encoded.len(),
        expected_len,
        "None field must contribute zero bytes to encoded output"
    );

    // Velocity TLV header (key=0x02, len=0x06) must not appear in the stream.
    // Single-byte scan would false-positive on Color::Green's value (0x0002).
    assert!(
        !encoded.windows(2).any(|w| w == [0x02, 0x06]),
        "velocity key+len pair must not appear when velocity is None"
    );

    // Roundtrip
    let decoded = ThreeOptFields::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, val);
}

#[test]
fn roundtrip_identity_all_required() {
    let original = AllRequired {
        color: Color::Blue,
        timestamp: Timestamp {
            seconds: 42,
            nanos: 7,
        },
    };
    let encoded = original.encode_value();
    let decoded = AllRequired::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn roundtrip_identity_all_optional_some() {
    let original = AllOptional {
        color: Some(Color::Alpha),
        velocity: Some(Velocity {
            dx: 10,
            dy: -10,
            dz: 0,
        }),
    };
    let encoded = original.encode_value();
    let decoded = AllOptional::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn roundtrip_identity_mixed_shape() {
    let original = MixedShape {
        priority: Priority::Medium,
        attitude: Some(Attitude {
            roll: 0.1,
            pitch: 0.2,
            yaw: 0.3,
        }),
    };
    let encoded = original.encode_value();
    let decoded = MixedShape::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn roundtrip_identity_mixed_shape_none() {
    let original = MixedShape {
        priority: Priority::High,
        attitude: None,
    };
    let encoded = original.encode_value();
    let decoded = MixedShape::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn roundtrip_owned_encoder() {
    // Encoder functions take T by value (not &T) — enc_owned! macro
    // clones the field and passes owned value to the encoder.
    let original = OwnedEncoderStruct {
        priority: Priority::Critical,
        color: Color::Blue,
    };
    let encoded = original.encode_value();
    let decoded = OwnedEncoderStruct::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
