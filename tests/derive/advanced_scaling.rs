//! Large struct and exhaustive optionality tests for `#[derive(Klv)]`
//!
//! Tests three struct shapes across a broad set of domain types: a 12-field
//! `TelemetryPacket` (primitives, enums, coordinates, status flags), an
//! 8-field `OptionalSuite` (all fields `Option<T>`), and an 8-field
//! `RequiredSuite` (all fields required). Covers full decode, encode/decode
//! roundtrip, all-present and all-absent optional scenarios, each optional
//! field present individually, and each required field missing individually
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::Klv;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct TelemetryPacket {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    id: u16,
    #[klv(
        key = 0x02,
        dec = Timestamp::decode_value,
        enc = Timestamp::encode_value,
    )]
    timestamp: Timestamp,
    #[klv(
        key = 0x03,
        dec = Coordinate::decode_value,
        enc = Coordinate::encode_value,
    )]
    position: Coordinate,
    #[klv(
        key = 0x04,
        dec = decb::be_f32,
        enc = *encb::be_f32,
    )]
    altitude: f32,
    #[klv(
        key = 0x05,
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    velocity: Velocity,
    #[klv(
        key = 0x06,
        dec = Attitude::decode_value,
        enc = Attitude::encode_value,
    )]
    attitude: Attitude,
    #[klv(
        key = 0x07,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
    #[klv(
        key = 0x08,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    priority: Priority,
    #[klv(
        key = 0x09,
        dec = Material::decode_value,
        enc = Material::encode_value,
    )]
    material: Material,
    #[klv(
        key = 0x0A,
        dec = StatusFlags::decode_value,
        enc = StatusFlags::encode_value,
    )]
    status: StatusFlags,
    #[klv(
        key = 0x0B,
        dec = OpMode::decode_value,
        enc = OpMode::encode_value,
    )]
    mode: OpMode,

    #[klv(
        key = 0x0C,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    battery: u8,
}

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct OptionalSuite {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Option<Color>,
    #[klv(
        key = 0x02,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    priority: Option<Priority>,
    #[klv(
        key = 0x03,
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    velocity: Option<Velocity>,
    #[klv(
        key = 0x04,
        dec = Attitude::decode_value,
        enc = Attitude::encode_value,
    )]
    attitude: Option<Attitude>,
    #[klv(
        key = 0x05,
        dec = Timestamp::decode_value,
        enc = Timestamp::encode_value,
    )]
    timestamp: Option<Timestamp>,
    #[klv(
        key = 0x06,
        dec = Coordinate::decode_value,
        enc = Coordinate::encode_value,
    )]
    coordinate: Option<Coordinate>,
    #[klv(
        key = 0x07,
        dec = StatusFlags::decode_value,
        enc = StatusFlags::encode_value,
    )]
    status: Option<StatusFlags>,
    #[klv(
        key = 0x08,
        dec = Material::decode_value,
        enc = Material::encode_value,
    )]
    material: Option<Material>,
}

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct RequiredSuite {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
    #[klv(
        key = 0x02,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    priority: Priority,
    #[klv(
        key = 0x03,
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    velocity: Velocity,
    #[klv(
        key = 0x04,
        dec = Attitude::decode_value,
        enc = Attitude::encode_value,
    )]
    attitude: Attitude,
    #[klv(
        key = 0x05,
        dec = Timestamp::decode_value,
        enc = Timestamp::encode_value,
    )]
    timestamp: Timestamp,
    #[klv(
        key = 0x06,
        dec = Coordinate::decode_value,
        enc = Coordinate::encode_value,
    )]
    coordinate: Coordinate,
    #[klv(
        key = 0x07,
        dec = StatusFlags::decode_value,
        enc = StatusFlags::encode_value,
    )]
    status: StatusFlags,
    #[klv(
        key = 0x08,
        dec = Material::decode_value,
        enc = Material::encode_value,
    )]
    material: Material,
}

/// Append a single TLV triple `[key:1][len:1][value:N]` to `data`
///
/// Calls `value` into a scratch buffer, then pushes the key byte, the
/// length byte, and all value bytes into `data`
fn push_tlv(data: &mut Vec<u8>, key: u8, value: impl FnOnce(&mut Vec<u8>)) {
    let mut buf = Vec::new();
    value(&mut buf);
    data.push(key);
    data.push(buf.len() as u8);
    data.extend(buf);
}

/// Construct a realistic [`TelemetryPacket`] with non-trivial values in every field
fn telemetry_fixture() -> TelemetryPacket {
    TelemetryPacket {
        id: 0x1234,
        timestamp: Timestamp {
            seconds: 1_700_000_000,
            nanos: 500,
        },
        position: Coordinate {
            lat: 48.8566,
            lon: 2.3522,
        },
        altitude: 1234.5_f32,
        velocity: Velocity {
            dx: 10,
            dy: -5,
            dz: 3,
        },
        attitude: Attitude {
            roll: 0.1_f32,
            pitch: -0.2_f32,
            yaw: 1.57_f32,
        },
        color: Color::Green,
        priority: Priority::High,
        material: Material::Composite,
        status: StatusFlags {
            active: true,
            armed: false,
            locked: true,
            mode: 7,
        },
        mode: OpMode::Active,
        battery: 85,
    }
}

/// Construct a [`RequiredSuite`] fixture with varied non-trivial values in every field
fn required_suite_fixture() -> RequiredSuite {
    RequiredSuite {
        color: Color::Blue,
        priority: Priority::Critical,
        velocity: Velocity {
            dx: 0,
            dy: 0,
            dz: -1,
        },
        attitude: Attitude {
            roll: 0.0_f32,
            pitch: 0.0_f32,
            yaw: 0.0_f32,
        },
        timestamp: Timestamp {
            seconds: 1_000_000,
            nanos: 0,
        },
        coordinate: Coordinate {
            lat: -33.8688,
            lon: 151.2093,
        },
        status: StatusFlags {
            active: false,
            armed: true,
            locked: false,
            mode: 2,
        },
        material: Material::Aluminum,
    }
}

/// Hand-build the raw byte stream for a [`TelemetryPacket`] using `push_tlv`
///
/// Encodes every field in declaration-key order so the bytes match what
/// `decode_value` expects; used to produce a known-good stream for the
/// `large_struct_decode` test
fn build_telemetry_bytes(p: &TelemetryPacket) -> Vec<u8> {
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| encb::be_u16(p.id, __b));
    push_tlv(&mut data, 0x02, |__b| p.timestamp.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| p.position.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| encb::be_f32(p.altitude, __b));
    push_tlv(&mut data, 0x05, |__b| p.velocity.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| p.attitude.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| p.color.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| p.priority.encode_value(__b));
    push_tlv(&mut data, 0x09, |__b| p.material.encode_value(__b));
    push_tlv(&mut data, 0x0A, |__b| p.status.encode_value(__b));
    push_tlv(&mut data, 0x0B, |__b| p.mode.encode_value(__b));
    push_tlv(&mut data, 0x0C, |__b| encb::u8(p.battery, __b));
    data
}

/// Hand-build the raw byte stream for a [`RequiredSuite`] using `push_tlv`
///
/// Encodes every field in declaration-key order; used by tests that verify
/// all-present decoding and per-field missing-required-key failures
fn build_required_bytes(s: &RequiredSuite) -> Vec<u8> {
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| s.color.encode_value(__b));
    push_tlv(&mut data, 0x02, |__b| s.priority.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| s.velocity.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| s.attitude.encode_value(__b));
    push_tlv(&mut data, 0x05, |__b| s.timestamp.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| s.coordinate.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| s.status.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| s.material.encode_value(__b));
    data
}

#[test]
/// Tests decoding a 12-field telemetry packet combining primitives, enums, coordinates, and status flags
fn large_struct_decode() {
    let fixture = telemetry_fixture();
    let data = build_telemetry_bytes(&fixture);
    let result = TelemetryPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.id, fixture.id);
    assert_eq!(result.timestamp, fixture.timestamp);
    assert_eq!(result.position, fixture.position);
    assert_eq!(result.altitude, fixture.altitude);
    assert_eq!(result.velocity, fixture.velocity);
    assert_eq!(result.attitude, fixture.attitude);
    assert_eq!(result.color, fixture.color);
    assert_eq!(result.priority, fixture.priority);
    assert_eq!(result.material, fixture.material);
    assert_eq!(result.status, fixture.status);
    assert_eq!(result.mode, fixture.mode);
    assert_eq!(result.battery, fixture.battery);
}

#[test]
/// Tests full encode/decode roundtrip for the 12-field telemetry packet
fn large_struct_roundtrip() {
    let original = telemetry_fixture();
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = TelemetryPacket::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that when all eight optional fields have keys present, they all decode to `Some(_)`
fn all_optional_8_all_present() {
    let color = Color::Red;
    let priority = Priority::Medium;
    let velocity = Velocity {
        dx: 1,
        dy: 2,
        dz: 3,
    };
    let attitude = Attitude {
        roll: 0.5_f32,
        pitch: 0.5_f32,
        yaw: 0.5_f32,
    };
    let timestamp = Timestamp {
        seconds: 42,
        nanos: 0,
    };
    let coordinate = Coordinate { lat: 0.0, lon: 0.0 };
    let status = StatusFlags {
        active: true,
        armed: true,
        locked: true,
        mode: 0,
    };
    let material = Material::Steel;

    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| color.encode_value(__b));
    push_tlv(&mut data, 0x02, |__b| priority.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| velocity.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| attitude.encode_value(__b));
    push_tlv(&mut data, 0x05, |__b| timestamp.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| coordinate.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| status.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| material.encode_value(__b));

    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, Some(color));
    assert_eq!(result.priority, Some(priority));
    assert_eq!(result.velocity, Some(velocity));
    assert_eq!(result.attitude, Some(attitude));
    assert_eq!(result.timestamp, Some(timestamp));
    assert_eq!(result.coordinate, Some(coordinate));
    assert_eq!(result.status, Some(status));
    assert_eq!(result.material, Some(material));
}

#[test]
/// Tests that decoding an empty input into an all-optional struct yields `None` for every field without error
fn all_optional_8_all_absent() {
    let result = OptionalSuite::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, None);
    assert_eq!(result.material, None);
}

#[test]
/// Tests that only the `color` optional decodes to `Some` when its key is the sole key present; others stay `None`
fn all_optional_each_alone_color() {
    let val = Color::Alpha;
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| val.encode_value(__b));
    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, Some(val));
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, None);
    assert_eq!(result.material, None);
}

#[test]
/// Tests that only the `priority` optional decodes to `Some` when it is the sole key present
fn all_optional_each_alone_priority() {
    let val = Priority::Low;
    let mut data = vec![];
    push_tlv(&mut data, 0x02, |__b| val.encode_value(__b));
    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, Some(val));
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, None);
    assert_eq!(result.material, None);
}

#[test]
/// Tests that only the `velocity` optional decodes to `Some` when it is the sole key present
fn all_optional_each_alone_velocity() {
    let val = Velocity {
        dx: -100,
        dy: 200,
        dz: 0,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x03, |__b| val.encode_value(__b));
    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, Some(val));
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, None);
    assert_eq!(result.material, None);
}

#[test]
/// Tests that only the `attitude` optional decodes to `Some` when it is the sole key present
fn all_optional_each_alone_attitude() {
    let val = Attitude {
        roll: 1.0_f32,
        pitch: 2.0_f32,
        yaw: 3.0_f32,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x04, |__b| val.encode_value(__b));
    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, Some(val));
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, None);
    assert_eq!(result.material, None);
}

#[test]
/// Tests that only the `timestamp` optional decodes to `Some` when it is the sole key present
fn all_optional_each_alone_timestamp() {
    let val = Timestamp {
        seconds: 999,
        nanos: 1,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x05, |__b| val.encode_value(__b));
    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, Some(val));
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, None);
    assert_eq!(result.material, None);
}

#[test]
/// Tests that only the `coordinate` optional decodes to `Some` when it is the sole key present
fn all_optional_each_alone_coordinate() {
    let val = Coordinate {
        lat: 51.5074,
        lon: -0.1278,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x06, |__b| val.encode_value(__b));
    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, Some(val));
    assert_eq!(result.status, None);
    assert_eq!(result.material, None);
}

#[test]
/// Tests that only the `status` optional decodes to `Some` when it is the sole key present
fn all_optional_each_alone_status() {
    let val = StatusFlags {
        active: false,
        armed: false,
        locked: false,
        mode: 31,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x07, |__b| val.encode_value(__b));
    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, Some(val));
    assert_eq!(result.material, None);
}

#[test]
/// Tests that only the `material` optional decodes to `Some` when it is the sole key present
fn all_optional_each_alone_material() {
    let val = Material::Ceramic;
    let mut data = vec![];
    push_tlv(&mut data, 0x08, |__b| val.encode_value(__b));
    let result = OptionalSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, None);
    assert_eq!(result.material, Some(val));
}

// --------------------------------------------------
// test 6: all_required_8_present
// --------------------------------------------------

#[test]
/// Tests that all eight required fields decode correctly when every key is present
fn all_required_8_present() {
    let fixture = required_suite_fixture();
    let data = build_required_bytes(&fixture);
    let result = RequiredSuite::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result, fixture);
}

// --------------------------------------------------
// test 7: all_required_each_missing
// --------------------------------------------------

#[test]
/// Tests that decoding fails when the required `color` key is absent from the stream
fn all_required_each_missing_color() {
    let s = required_suite_fixture();
    let mut data = vec![];
    // omit key 0x01 (color)
    push_tlv(&mut data, 0x02, |__b| s.priority.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| s.velocity.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| s.attitude.encode_value(__b));
    push_tlv(&mut data, 0x05, |__b| s.timestamp.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| s.coordinate.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| s.status.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| s.material.encode_value(__b));
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `priority` key is absent from the stream
fn all_required_each_missing_priority() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| s.color.encode_value(__b));
    // omit key 0x02 (priority)
    push_tlv(&mut data, 0x03, |__b| s.velocity.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| s.attitude.encode_value(__b));
    push_tlv(&mut data, 0x05, |__b| s.timestamp.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| s.coordinate.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| s.status.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| s.material.encode_value(__b));
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `velocity` key is absent from the stream
fn all_required_each_missing_velocity() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| s.color.encode_value(__b));
    push_tlv(&mut data, 0x02, |__b| s.priority.encode_value(__b));
    // omit key 0x03 (velocity)
    push_tlv(&mut data, 0x04, |__b| s.attitude.encode_value(__b));
    push_tlv(&mut data, 0x05, |__b| s.timestamp.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| s.coordinate.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| s.status.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| s.material.encode_value(__b));
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `attitude` key is absent from the stream
fn all_required_each_missing_attitude() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| s.color.encode_value(__b));
    push_tlv(&mut data, 0x02, |__b| s.priority.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| s.velocity.encode_value(__b));
    // omit key 0x04 (attitude)
    push_tlv(&mut data, 0x05, |__b| s.timestamp.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| s.coordinate.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| s.status.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| s.material.encode_value(__b));
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `timestamp` key is absent from the stream
fn all_required_each_missing_timestamp() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| s.color.encode_value(__b));
    push_tlv(&mut data, 0x02, |__b| s.priority.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| s.velocity.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| s.attitude.encode_value(__b));
    // omit key 0x05 (timestamp)
    push_tlv(&mut data, 0x06, |__b| s.coordinate.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| s.status.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| s.material.encode_value(__b));
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `coordinate` key is absent from the stream
fn all_required_each_missing_coordinate() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| s.color.encode_value(__b));
    push_tlv(&mut data, 0x02, |__b| s.priority.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| s.velocity.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| s.attitude.encode_value(__b));
    push_tlv(&mut data, 0x05, |__b| s.timestamp.encode_value(__b));
    // omit key 0x06 (coordinate)
    push_tlv(&mut data, 0x07, |__b| s.status.encode_value(__b));
    push_tlv(&mut data, 0x08, |__b| s.material.encode_value(__b));
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `status` key is absent from the stream
fn all_required_each_missing_status() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| s.color.encode_value(__b));
    push_tlv(&mut data, 0x02, |__b| s.priority.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| s.velocity.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| s.attitude.encode_value(__b));
    push_tlv(&mut data, 0x05, |__b| s.timestamp.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| s.coordinate.encode_value(__b));
    // omit key 0x07 (status)
    push_tlv(&mut data, 0x08, |__b| s.material.encode_value(__b));
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `material` key is absent from the stream
fn all_required_each_missing_material() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, |__b| s.color.encode_value(__b));
    push_tlv(&mut data, 0x02, |__b| s.priority.encode_value(__b));
    push_tlv(&mut data, 0x03, |__b| s.velocity.encode_value(__b));
    push_tlv(&mut data, 0x04, |__b| s.attitude.encode_value(__b));
    push_tlv(&mut data, 0x05, |__b| s.timestamp.encode_value(__b));
    push_tlv(&mut data, 0x06, |__b| s.coordinate.encode_value(__b));
    push_tlv(&mut data, 0x07, |__b| s.status.encode_value(__b));
    // omit key 0x08 (material)
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}
