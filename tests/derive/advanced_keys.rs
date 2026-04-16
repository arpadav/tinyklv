//! Derive macro tests - non-sequential and boundary key values with complex domain types
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// wide key spacing (0x01, 0x40, 0x80, 0xFE)
// --------------------------------------------------
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WideKeySpacing {
    #[klv(key = 0x01, dec = decode_color, enc = encode_color)]
    color: Color,
    #[klv(key = 0x40, dec = decode_priority, enc = encode_priority)]
    priority: Priority,
    #[klv(key = 0x80, dec = decode_velocity, enc = encode_velocity)]
    velocity: Velocity,
    #[klv(key = 0xFE, dec = decode_status_flags, enc = encode_status_flags)]
    flags: StatusFlags,
}

// --------------------------------------------------
// boundary keys (0x00, 0x7F, 0x80, 0xFF)
// --------------------------------------------------
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct BoundaryKeys {
    #[klv(key = 0x00, dec = decode_timestamp, enc = encode_timestamp)]
    timestamp: Timestamp,
    #[klv(key = 0x7F, dec = decode_color, enc = encode_color)]
    color: Color,
    #[klv(key = 0x80, dec = decode_attitude, enc = encode_attitude)]
    attitude: Attitude,
    #[klv(key = 0xFF, dec = decode_coordinate, enc = encode_coordinate)]
    coord: Coordinate,
}

// --------------------------------------------------
// optional wide key spacing (0x01, 0x40, 0x80, 0xFE)
// --------------------------------------------------
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WideKeySpacingOptional {
    #[klv(key = 0x01, dec = decode_color, enc = encode_color)]
    color: Option<Color>,
    #[klv(key = 0x40, dec = decode_priority, enc = encode_priority)]
    priority: Option<Priority>,
    #[klv(key = 0x80, dec = decode_velocity, enc = encode_velocity)]
    velocity: Option<Velocity>,
    #[klv(key = 0xFE, dec = decode_status_flags, enc = encode_status_flags)]
    flags: Option<StatusFlags>,
}

// --------------------------------------------------
// helpers
// --------------------------------------------------

/// Build a single KLV triple: [key:1][len:1][value:N]
fn klv_triple(key: u8, value: &[u8]) -> Vec<u8> {
    let mut out = vec![key, value.len() as u8];
    out.extend_from_slice(value);
    out
}

// --------------------------------------------------
// tests
// --------------------------------------------------

#[test]
fn wide_key_spacing_roundtrip() {
    let color_bytes = encode_color(&Color::Green);
    let priority_bytes = encode_priority(&Priority::High);
    let velocity_bytes = encode_velocity(&Velocity {
        dx: 100,
        dy: -50,
        dz: 25,
    });
    let flags_bytes = encode_status_flags(&StatusFlags {
        active: true,
        armed: false,
        locked: true,
        mode: 3,
    });
    let mut data: Vec<u8> = Vec::new();
    data.extend(klv_triple(0x01, &color_bytes));
    data.extend(klv_triple(0x40, &priority_bytes));
    data.extend(klv_triple(0x80, &velocity_bytes));
    data.extend(klv_triple(0xFE, &flags_bytes));
    let result = WideKeySpacing::decode(&mut &data[..]).unwrap();
    assert_eq!(result.color, Color::Green);
    assert_eq!(result.priority, Priority::High);
    assert_eq!(
        result.velocity,
        Velocity {
            dx: 100,
            dy: -50,
            dz: 25
        }
    );
    assert_eq!(
        result.flags,
        StatusFlags {
            active: true,
            armed: false,
            locked: true,
            mode: 3
        }
    );
    // encode -> decode roundtrip
    let encoded = result.encode_value();
    let decoded = WideKeySpacing::decode(&mut &encoded[..]).unwrap();
    assert_eq!(decoded.color, Color::Green);
    assert_eq!(decoded.priority, Priority::High);
    assert_eq!(
        decoded.velocity,
        Velocity {
            dx: 100,
            dy: -50,
            dz: 25
        }
    );
    assert_eq!(
        decoded.flags,
        StatusFlags {
            active: true,
            armed: false,
            locked: true,
            mode: 3
        }
    );
}

#[test]
fn boundary_keys_roundtrip() {
    let ts = Timestamp {
        seconds: 0xDEAD_BEEF,
        nanos: 0x1234,
    };
    let attitude = Attitude {
        roll: 1.5,
        pitch: -0.5,
        yaw: 3.14,
    };
    let coord = Coordinate {
        lat: 48.8566,
        lon: 2.3522,
    };
    let original = BoundaryKeys {
        timestamp: ts,
        color: Color::Blue,
        attitude,
        coord: coord.clone(),
    };
    let encoded = original.encode_value();
    let decoded = BoundaryKeys::decode(&mut &encoded[..]).unwrap();
    assert_eq!(decoded.timestamp, ts);
    assert_eq!(decoded.color, Color::Blue);
    assert_eq!(decoded.attitude, attitude);
    assert_eq!(decoded.coord, coord);
}

#[test]
fn wide_keys_reversed_order() {
    // Same struct as WideKeySpacing, but KLV triples arrive in reverse key order
    let color_bytes = encode_color(&Color::Red);
    let priority_bytes = encode_priority(&Priority::Critical);
    let velocity_bytes = encode_velocity(&Velocity {
        dx: 0,
        dy: 0,
        dz: -1,
    });
    let flags_bytes = encode_status_flags(&StatusFlags {
        active: false,
        armed: true,
        locked: false,
        mode: 7,
    });
    let mut data: Vec<u8> = Vec::new();
    // reversed: 0xFE, 0x80, 0x40, 0x01
    data.extend(klv_triple(0xFE, &flags_bytes));
    data.extend(klv_triple(0x80, &velocity_bytes));
    data.extend(klv_triple(0x40, &priority_bytes));
    data.extend(klv_triple(0x01, &color_bytes));
    let result = WideKeySpacing::decode(&mut &data[..]).unwrap();
    assert_eq!(result.color, Color::Red);
    assert_eq!(result.priority, Priority::Critical);
    assert_eq!(
        result.velocity,
        Velocity {
            dx: 0,
            dy: 0,
            dz: -1
        }
    );
    assert_eq!(
        result.flags,
        StatusFlags {
            active: false,
            armed: true,
            locked: false,
            mode: 7
        }
    );
}

#[test]
fn wide_keys_partial_optional() {
    // Only keys 0x01 (Color) and 0x80 (Velocity) present; 0x40 and 0xFE absent
    let color_bytes = encode_color(&Color::Alpha);
    let velocity_bytes = encode_velocity(&Velocity {
        dx: 10,
        dy: 20,
        dz: 30,
    });
    let mut data: Vec<u8> = Vec::new();
    data.extend(klv_triple(0x01, &color_bytes));
    data.extend(klv_triple(0x80, &velocity_bytes));
    let result = WideKeySpacingOptional::decode(&mut &data[..]).unwrap();
    assert_eq!(result.color, Some(Color::Alpha));
    assert_eq!(result.priority, None);
    assert_eq!(
        result.velocity,
        Some(Velocity {
            dx: 10,
            dy: 20,
            dz: 30
        })
    );
    assert_eq!(result.flags, None);
}
