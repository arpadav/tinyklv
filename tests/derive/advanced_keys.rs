//! Derive macro tests - non-sequential and boundary key values with complex domain types
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;
use tinyklv::Klv;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct WideKeySpacing {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
    #[klv(
        key = 0x40,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    priority: Priority,
    #[klv(
        key = 0x80,
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    velocity: Velocity,
    #[klv(
        key = 0xFE,
        dec = StatusFlags::decode_value,
        enc = StatusFlags::encode_value,
    )]
    flags: StatusFlags,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct BoundaryKeys {
    #[klv(
        key = 0x00,
        dec = Timestamp::decode_value,
        enc = Timestamp::encode_value,
    )]
    timestamp: Timestamp,
    #[klv(
        key = 0x7F,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
    #[klv(
        key = 0x80,
        dec = Attitude::decode_value,
        enc = Attitude::encode_value,
    )]
    attitude: Attitude,
    #[klv(
        key = 0xFF,
        dec = Coordinate::decode_value,
        enc = Coordinate::encode_value,
    )]
    coord: Coordinate,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct WideKeySpacingOptional {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Option<Color>,
    #[klv(
        key = 0x40,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    priority: Option<Priority>,
    #[klv(
        key = 0x80,
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    velocity: Option<Velocity>,
    #[klv(
        key = 0xFE,
        dec = StatusFlags::decode_value,
        enc = StatusFlags::encode_value,
    )]
    flags: Option<StatusFlags>,
}

/// Build a single KLV triple: [key:1][len:1][value:N]
fn klv_triple(key: u8, value: &[u8]) -> Vec<u8> {
    let mut out = vec![key, value.len() as u8];
    out.extend_from_slice(value);
    out
}

#[test]
/// Tests decode-and-roundtrip for a struct using widely-spaced keys (`0x01`, `0x40`, `0x80`, `0xFE`) over complex domain types.
fn wide_key_spacing_roundtrip() {
    let color_bytes = Color::Green.encode_value();
    let priority_bytes = Priority::High.encode_value();
    let velocity_bytes = Velocity {
        dx: 100,
        dy: -50,
        dz: 25,
    }
    .encode_value();
    let flags_bytes = StatusFlags {
        active: true,
        armed: false,
        locked: true,
        mode: 3,
    }
    .encode_value();
    let mut data: Vec<u8> = Vec::new();
    data.extend(klv_triple(0x01, &color_bytes));
    data.extend(klv_triple(0x40, &priority_bytes));
    data.extend(klv_triple(0x80, &velocity_bytes));
    data.extend(klv_triple(0xFE, &flags_bytes));
    let result = WideKeySpacing::decode_value(&mut &data[..]).unwrap();
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
    let decoded = WideKeySpacing::decode_value(&mut &encoded[..]).unwrap();
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
/// Tests encode/decode roundtrip for a struct whose keys sit at `u8` boundaries (`0x00`, `0x7F`, `0x80`, `0xFF`).
fn boundary_keys_roundtrip() {
    let ts = Timestamp {
        seconds: 0xDEAD_BEEF,
        nanos: 0x1234,
    };
    let attitude = Attitude {
        roll: 1.5,
        pitch: -0.5,
        yaw: 3.13,
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
    let decoded = BoundaryKeys::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded.timestamp, ts);
    assert_eq!(decoded.color, Color::Blue);
    assert_eq!(decoded.attitude, attitude);
    assert_eq!(decoded.coord, coord);
}

#[test]
/// Verifies that widely-spaced keys decode correctly when the triples arrive in reverse key order (`0xFE` first, `0x01` last).
fn wide_keys_reversed_order() {
    let color_bytes = Color::Red.encode_value();
    let priority_bytes = Priority::Critical.encode_value();
    let velocity_bytes = Velocity {
        dx: 0,
        dy: 0,
        dz: -1,
    }
    .encode_value();
    let flags_bytes = StatusFlags {
        active: false,
        armed: true,
        locked: false,
        mode: 7,
    }
    .encode_value();
    let mut data: Vec<u8> = Vec::new();
    // reversed: 0xFE, 0x80, 0x40, 0x01
    data.extend(klv_triple(0xFE, &flags_bytes));
    data.extend(klv_triple(0x80, &velocity_bytes));
    data.extend(klv_triple(0x40, &priority_bytes));
    data.extend(klv_triple(0x01, &color_bytes));
    let result = WideKeySpacing::decode_value(&mut &data[..]).unwrap();
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
/// Tests that only the present optional keys (`0x01` and `0x80`) decode to `Some`, while absent keys (`0x40`, `0xFE`) decode to `None`.
fn wide_keys_partial_optional() {
    let color_bytes = Color::Alpha.encode_value();
    let velocity_bytes = Velocity {
        dx: 10,
        dy: 20,
        dz: 30,
    }
    .encode_value();
    let mut data: Vec<u8> = Vec::new();
    data.extend(klv_triple(0x01, &color_bytes));
    data.extend(klv_triple(0x80, &velocity_bytes));
    let result = WideKeySpacingOptional::decode_value(&mut &data[..]).unwrap();
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
