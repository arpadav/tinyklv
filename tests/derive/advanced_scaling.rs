//! Large struct and exhaustive optionality tests for `#[derive(Klv)]`
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct TelemetryPacket {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    id: u16,
    #[klv(key = 0x02, dec = Timestamp::decode_value, enc = Timestamp::encode_value)]
    timestamp: Timestamp,
    #[klv(key = 0x03, dec = Coordinate::decode_value, enc = Coordinate::encode_value)]
    position: Coordinate,
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_f32, enc = &tinyklv::enc::binary::be_f32)]
    altitude: f32,
    #[klv(key = 0x05, dec = Velocity::decode_value, enc = Velocity::encode_value)]
    velocity: Velocity,
    #[klv(key = 0x06, dec = Attitude::decode_value, enc = Attitude::encode_value)]
    attitude: Attitude,
    #[klv(key = 0x07, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
    #[klv(key = 0x08, dec = Priority::decode_value, enc = Priority::encode_value)]
    priority: Priority,
    #[klv(key = 0x09, dec = Material::decode_value, enc = Material::encode_value)]
    material: Material,
    #[klv(key = 0x0A, dec = StatusFlags::decode_value, enc = StatusFlags::encode_value)]
    status: StatusFlags,
    #[klv(key = 0x0B, dec = OpMode::decode_value, enc = OpMode::encode_value)]
    mode: OpMode,

    #[klv(
        key = 0x0C,
        dec = tinyklv::dec::binary::be_u8,
        enc = &tinyklv::enc::binary::u8,
    )]
    battery: u8,
}

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct OptionalSuite {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Option<Color>,
    #[klv(key = 0x02, dec = Priority::decode_value, enc = Priority::encode_value)]
    priority: Option<Priority>,
    #[klv(key = 0x03, dec = Velocity::decode_value, enc = Velocity::encode_value)]
    velocity: Option<Velocity>,
    #[klv(key = 0x04, dec = Attitude::decode_value, enc = Attitude::encode_value)]
    attitude: Option<Attitude>,
    #[klv(key = 0x05, dec = Timestamp::decode_value, enc = Timestamp::encode_value)]
    timestamp: Option<Timestamp>,
    #[klv(key = 0x06, dec = Coordinate::decode_value, enc = Coordinate::encode_value)]
    coordinate: Option<Coordinate>,
    #[klv(key = 0x07, dec = StatusFlags::decode_value, enc = StatusFlags::encode_value)]
    status: Option<StatusFlags>,
    #[klv(key = 0x08, dec = Material::decode_value, enc = Material::encode_value)]
    material: Option<Material>,
}

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct RequiredSuite {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
    #[klv(key = 0x02, dec = Priority::decode_value, enc = Priority::encode_value)]
    priority: Priority,
    #[klv(key = 0x03, dec = Velocity::decode_value, enc = Velocity::encode_value)]
    velocity: Velocity,
    #[klv(key = 0x04, dec = Attitude::decode_value, enc = Attitude::encode_value)]
    attitude: Attitude,
    #[klv(key = 0x05, dec = Timestamp::decode_value, enc = Timestamp::encode_value)]
    timestamp: Timestamp,
    #[klv(key = 0x06, dec = Coordinate::decode_value, enc = Coordinate::encode_value)]
    coordinate: Coordinate,
    #[klv(key = 0x07, dec = StatusFlags::decode_value, enc = StatusFlags::encode_value)]
    status: StatusFlags,
    #[klv(key = 0x08, dec = Material::decode_value, enc = Material::encode_value)]
    material: Material,
}

fn push_tlv(data: &mut Vec<u8>, key: u8, value: Vec<u8>) {
    data.push(key);
    data.push(value.len() as u8);
    data.extend(value);
}

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

fn build_telemetry_bytes(p: &TelemetryPacket) -> Vec<u8> {
    let mut data = vec![];
    push_tlv(&mut data, 0x01, tinyklv::enc::binary::be_u16(p.id));
    push_tlv(&mut data, 0x02, p.timestamp.encode_value());
    push_tlv(&mut data, 0x03, p.position.encode_value());
    push_tlv(&mut data, 0x04, tinyklv::enc::binary::be_f32(p.altitude));
    push_tlv(&mut data, 0x05, p.velocity.encode_value());
    push_tlv(&mut data, 0x06, p.attitude.encode_value());
    push_tlv(&mut data, 0x07, p.color.encode_value());
    push_tlv(&mut data, 0x08, p.priority.encode_value());
    push_tlv(&mut data, 0x09, p.material.encode_value());
    push_tlv(&mut data, 0x0A, p.status.encode_value());
    push_tlv(&mut data, 0x0B, p.mode.encode_value());
    push_tlv(&mut data, 0x0C, tinyklv::enc::binary::u8(p.battery));
    data
}

fn build_required_bytes(s: &RequiredSuite) -> Vec<u8> {
    let mut data = vec![];
    push_tlv(&mut data, 0x01, s.color.encode_value());
    push_tlv(&mut data, 0x02, s.priority.encode_value());
    push_tlv(&mut data, 0x03, s.velocity.encode_value());
    push_tlv(&mut data, 0x04, s.attitude.encode_value());
    push_tlv(&mut data, 0x05, s.timestamp.encode_value());
    push_tlv(&mut data, 0x06, s.coordinate.encode_value());
    push_tlv(&mut data, 0x07, s.status.encode_value());
    push_tlv(&mut data, 0x08, s.material.encode_value());
    data
}

#[test]
/// Tests decoding a 12-field telemetry packet combining primitives, enums, coordinates, and status flags.
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
/// Tests full encode/decode roundtrip for the 12-field telemetry packet.
fn large_struct_roundtrip() {
    let original = telemetry_fixture();
    let encoded = original.encode_value();
    let decoded = TelemetryPacket::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that when all eight optional fields have keys present on the wire, they all decode to `Some(_)`.
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
    push_tlv(&mut data, 0x01, color.encode_value());
    push_tlv(&mut data, 0x02, priority.encode_value());
    push_tlv(&mut data, 0x03, velocity.encode_value());
    push_tlv(&mut data, 0x04, attitude.encode_value());
    push_tlv(&mut data, 0x05, timestamp.encode_value());
    push_tlv(&mut data, 0x06, coordinate.encode_value());
    push_tlv(&mut data, 0x07, status.encode_value());
    push_tlv(&mut data, 0x08, material.encode_value());

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
/// Tests that decoding an empty input into an all-optional struct yields `None` for every field without error.
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
/// Tests that only the `color` optional decodes to `Some` when its key is the sole key present; others stay `None`.
fn all_optional_each_alone_color() {
    let val = Color::Alpha;
    let mut data = vec![];
    push_tlv(&mut data, 0x01, val.encode_value());
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
/// Tests that only the `priority` optional decodes to `Some` when it is the sole key present.
fn all_optional_each_alone_priority() {
    let val = Priority::Low;
    let mut data = vec![];
    push_tlv(&mut data, 0x02, val.encode_value());
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
/// Tests that only the `velocity` optional decodes to `Some` when it is the sole key present.
fn all_optional_each_alone_velocity() {
    let val = Velocity {
        dx: -100,
        dy: 200,
        dz: 0,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x03, val.encode_value());
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
/// Tests that only the `attitude` optional decodes to `Some` when it is the sole key present.
fn all_optional_each_alone_attitude() {
    let val = Attitude {
        roll: 1.0_f32,
        pitch: 2.0_f32,
        yaw: 3.0_f32,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x04, val.encode_value());
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
/// Tests that only the `timestamp` optional decodes to `Some` when it is the sole key present.
fn all_optional_each_alone_timestamp() {
    let val = Timestamp {
        seconds: 999,
        nanos: 1,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x05, val.encode_value());
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
/// Tests that only the `coordinate` optional decodes to `Some` when it is the sole key present.
fn all_optional_each_alone_coordinate() {
    let val = Coordinate {
        lat: 51.5074,
        lon: -0.1278,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x06, val.encode_value());
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
/// Tests that only the `status` optional decodes to `Some` when it is the sole key present.
fn all_optional_each_alone_status() {
    let val = StatusFlags {
        active: false,
        armed: false,
        locked: false,
        mode: 31,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x07, val.encode_value());
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
/// Tests that only the `material` optional decodes to `Some` when it is the sole key present.
fn all_optional_each_alone_material() {
    let val = Material::Ceramic;
    let mut data = vec![];
    push_tlv(&mut data, 0x08, val.encode_value());
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
/// Tests that all eight required fields decode correctly when every key is present on the wire.
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
/// Tests that decoding fails when the required `color` key is absent from the stream.
fn all_required_each_missing_color() {
    let s = required_suite_fixture();
    let mut data = vec![];
    // omit key 0x01 (color)
    push_tlv(&mut data, 0x02, s.priority.encode_value());
    push_tlv(&mut data, 0x03, s.velocity.encode_value());
    push_tlv(&mut data, 0x04, s.attitude.encode_value());
    push_tlv(&mut data, 0x05, s.timestamp.encode_value());
    push_tlv(&mut data, 0x06, s.coordinate.encode_value());
    push_tlv(&mut data, 0x07, s.status.encode_value());
    push_tlv(&mut data, 0x08, s.material.encode_value());
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `priority` key is absent from the stream.
fn all_required_each_missing_priority() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, s.color.encode_value());
    // omit key 0x02 (priority)
    push_tlv(&mut data, 0x03, s.velocity.encode_value());
    push_tlv(&mut data, 0x04, s.attitude.encode_value());
    push_tlv(&mut data, 0x05, s.timestamp.encode_value());
    push_tlv(&mut data, 0x06, s.coordinate.encode_value());
    push_tlv(&mut data, 0x07, s.status.encode_value());
    push_tlv(&mut data, 0x08, s.material.encode_value());
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `velocity` key is absent from the stream.
fn all_required_each_missing_velocity() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, s.color.encode_value());
    push_tlv(&mut data, 0x02, s.priority.encode_value());
    // omit key 0x03 (velocity)
    push_tlv(&mut data, 0x04, s.attitude.encode_value());
    push_tlv(&mut data, 0x05, s.timestamp.encode_value());
    push_tlv(&mut data, 0x06, s.coordinate.encode_value());
    push_tlv(&mut data, 0x07, s.status.encode_value());
    push_tlv(&mut data, 0x08, s.material.encode_value());
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `attitude` key is absent from the stream.
fn all_required_each_missing_attitude() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, s.color.encode_value());
    push_tlv(&mut data, 0x02, s.priority.encode_value());
    push_tlv(&mut data, 0x03, s.velocity.encode_value());
    // omit key 0x04 (attitude)
    push_tlv(&mut data, 0x05, s.timestamp.encode_value());
    push_tlv(&mut data, 0x06, s.coordinate.encode_value());
    push_tlv(&mut data, 0x07, s.status.encode_value());
    push_tlv(&mut data, 0x08, s.material.encode_value());
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `timestamp` key is absent from the stream.
fn all_required_each_missing_timestamp() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, s.color.encode_value());
    push_tlv(&mut data, 0x02, s.priority.encode_value());
    push_tlv(&mut data, 0x03, s.velocity.encode_value());
    push_tlv(&mut data, 0x04, s.attitude.encode_value());
    // omit key 0x05 (timestamp)
    push_tlv(&mut data, 0x06, s.coordinate.encode_value());
    push_tlv(&mut data, 0x07, s.status.encode_value());
    push_tlv(&mut data, 0x08, s.material.encode_value());
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `coordinate` key is absent from the stream.
fn all_required_each_missing_coordinate() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, s.color.encode_value());
    push_tlv(&mut data, 0x02, s.priority.encode_value());
    push_tlv(&mut data, 0x03, s.velocity.encode_value());
    push_tlv(&mut data, 0x04, s.attitude.encode_value());
    push_tlv(&mut data, 0x05, s.timestamp.encode_value());
    // omit key 0x06 (coordinate)
    push_tlv(&mut data, 0x07, s.status.encode_value());
    push_tlv(&mut data, 0x08, s.material.encode_value());
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `status` key is absent from the stream.
fn all_required_each_missing_status() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, s.color.encode_value());
    push_tlv(&mut data, 0x02, s.priority.encode_value());
    push_tlv(&mut data, 0x03, s.velocity.encode_value());
    push_tlv(&mut data, 0x04, s.attitude.encode_value());
    push_tlv(&mut data, 0x05, s.timestamp.encode_value());
    push_tlv(&mut data, 0x06, s.coordinate.encode_value());
    // omit key 0x07 (status)
    push_tlv(&mut data, 0x08, s.material.encode_value());
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}

#[test]
/// Tests that decoding fails when the required `material` key is absent from the stream.
fn all_required_each_missing_material() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, s.color.encode_value());
    push_tlv(&mut data, 0x02, s.priority.encode_value());
    push_tlv(&mut data, 0x03, s.velocity.encode_value());
    push_tlv(&mut data, 0x04, s.attitude.encode_value());
    push_tlv(&mut data, 0x05, s.timestamp.encode_value());
    push_tlv(&mut data, 0x06, s.coordinate.encode_value());
    push_tlv(&mut data, 0x07, s.status.encode_value());
    // omit key 0x08 (material)
    assert!(RequiredSuite::decode_value(&mut data.as_slice()).is_err());
}
