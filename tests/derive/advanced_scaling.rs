//! Large struct and exhaustive optionality tests for `#[derive(Klv)]`
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// TelemetryPacket - 12-field required struct
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct TelemetryPacket {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    id: u16,
    #[klv(key = 0x02, dec = decode_timestamp, enc = encode_timestamp)]
    timestamp: Timestamp,
    #[klv(key = 0x03, dec = decode_coordinate, enc = encode_coordinate)]
    position: Coordinate,
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_f32, enc = enc_f32)]
    altitude: f32,
    #[klv(key = 0x05, dec = decode_velocity, enc = encode_velocity)]
    velocity: Velocity,
    #[klv(key = 0x06, dec = decode_attitude, enc = encode_attitude)]
    attitude: Attitude,
    #[klv(key = 0x07, dec = decode_color, enc = encode_color)]
    color: Color,
    #[klv(key = 0x08, dec = decode_priority, enc = encode_priority)]
    priority: Priority,
    #[klv(key = 0x09, dec = decode_material, enc = encode_material)]
    material: Material,
    #[klv(key = 0x0A, dec = decode_status_flags, enc = encode_status_flags)]
    status: StatusFlags,
    #[klv(key = 0x0B, dec = decode_opmode, enc = encode_opmode)]
    mode: OpMode,
    #[klv(key = 0x0C, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    battery: u8,
}

// --------------------------------------------------
// OptionalSuite - 8 optional fields, all types
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct OptionalSuite {
    #[klv(key = 0x01, dec = decode_color, enc = encode_color)]
    color: Option<Color>,
    #[klv(key = 0x02, dec = decode_priority, enc = encode_priority)]
    priority: Option<Priority>,
    #[klv(key = 0x03, dec = decode_velocity, enc = encode_velocity)]
    velocity: Option<Velocity>,
    #[klv(key = 0x04, dec = decode_attitude, enc = encode_attitude)]
    attitude: Option<Attitude>,
    #[klv(key = 0x05, dec = decode_timestamp, enc = encode_timestamp)]
    timestamp: Option<Timestamp>,
    #[klv(key = 0x06, dec = decode_coordinate, enc = encode_coordinate)]
    coordinate: Option<Coordinate>,
    #[klv(key = 0x07, dec = decode_status_flags, enc = encode_status_flags)]
    status: Option<StatusFlags>,
    #[klv(key = 0x08, dec = decode_material, enc = encode_material)]
    material: Option<Material>,
}

// --------------------------------------------------
// RequiredSuite - same 8 types, all required
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct RequiredSuite {
    #[klv(key = 0x01, dec = decode_color, enc = encode_color)]
    color: Color,
    #[klv(key = 0x02, dec = decode_priority, enc = encode_priority)]
    priority: Priority,
    #[klv(key = 0x03, dec = decode_velocity, enc = encode_velocity)]
    velocity: Velocity,
    #[klv(key = 0x04, dec = decode_attitude, enc = encode_attitude)]
    attitude: Attitude,
    #[klv(key = 0x05, dec = decode_timestamp, enc = encode_timestamp)]
    timestamp: Timestamp,
    #[klv(key = 0x06, dec = decode_coordinate, enc = encode_coordinate)]
    coordinate: Coordinate,
    #[klv(key = 0x07, dec = decode_status_flags, enc = encode_status_flags)]
    status: StatusFlags,
    #[klv(key = 0x08, dec = decode_material, enc = encode_material)]
    material: Material,
}

// --------------------------------------------------
// helpers
// --------------------------------------------------

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
    push_tlv(&mut data, 0x01, enc_u16(&p.id));
    push_tlv(&mut data, 0x02, encode_timestamp(&p.timestamp));
    push_tlv(&mut data, 0x03, encode_coordinate(&p.position));
    push_tlv(&mut data, 0x04, enc_f32(&p.altitude));
    push_tlv(&mut data, 0x05, encode_velocity(&p.velocity));
    push_tlv(&mut data, 0x06, encode_attitude(&p.attitude));
    push_tlv(&mut data, 0x07, encode_color(&p.color));
    push_tlv(&mut data, 0x08, encode_priority(&p.priority));
    push_tlv(&mut data, 0x09, encode_material(&p.material));
    push_tlv(&mut data, 0x0A, encode_status_flags(&p.status));
    push_tlv(&mut data, 0x0B, encode_opmode(&p.mode));
    push_tlv(&mut data, 0x0C, enc_u8(&p.battery));
    data
}

fn build_required_bytes(s: &RequiredSuite) -> Vec<u8> {
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&s.color));
    push_tlv(&mut data, 0x02, encode_priority(&s.priority));
    push_tlv(&mut data, 0x03, encode_velocity(&s.velocity));
    push_tlv(&mut data, 0x04, encode_attitude(&s.attitude));
    push_tlv(&mut data, 0x05, encode_timestamp(&s.timestamp));
    push_tlv(&mut data, 0x06, encode_coordinate(&s.coordinate));
    push_tlv(&mut data, 0x07, encode_status_flags(&s.status));
    push_tlv(&mut data, 0x08, encode_material(&s.material));
    data
}

// --------------------------------------------------
// test 1: large_struct_decode
// --------------------------------------------------

#[test]
fn large_struct_decode() {
    let fixture = telemetry_fixture();
    let data = build_telemetry_bytes(&fixture);
    let result = TelemetryPacket::decode(&mut data.as_slice()).unwrap();
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

// --------------------------------------------------
// test 2: large_struct_roundtrip
// --------------------------------------------------

#[test]
fn large_struct_roundtrip() {
    let original = telemetry_fixture();
    let encoded = original.encode_value();
    let decoded = TelemetryPacket::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// test 3: all_optional_8_all_present
// --------------------------------------------------

#[test]
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
    push_tlv(&mut data, 0x01, encode_color(&color));
    push_tlv(&mut data, 0x02, encode_priority(&priority));
    push_tlv(&mut data, 0x03, encode_velocity(&velocity));
    push_tlv(&mut data, 0x04, encode_attitude(&attitude));
    push_tlv(&mut data, 0x05, encode_timestamp(&timestamp));
    push_tlv(&mut data, 0x06, encode_coordinate(&coordinate));
    push_tlv(&mut data, 0x07, encode_status_flags(&status));
    push_tlv(&mut data, 0x08, encode_material(&material));

    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
    assert_eq!(result.color, Some(color));
    assert_eq!(result.priority, Some(priority));
    assert_eq!(result.velocity, Some(velocity));
    assert_eq!(result.attitude, Some(attitude));
    assert_eq!(result.timestamp, Some(timestamp));
    assert_eq!(result.coordinate, Some(coordinate));
    assert_eq!(result.status, Some(status));
    assert_eq!(result.material, Some(material));
}

// --------------------------------------------------
// test 4: all_optional_8_all_absent
// --------------------------------------------------

#[test]
fn all_optional_8_all_absent() {
    let result = OptionalSuite::decode(&mut [].as_slice()).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
    assert_eq!(result.velocity, None);
    assert_eq!(result.attitude, None);
    assert_eq!(result.timestamp, None);
    assert_eq!(result.coordinate, None);
    assert_eq!(result.status, None);
    assert_eq!(result.material, None);
}

// --------------------------------------------------
// test 5: all_optional_each_alone
// --------------------------------------------------

#[test]
fn all_optional_each_alone_color() {
    let val = Color::Alpha;
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&val));
    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
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
fn all_optional_each_alone_priority() {
    let val = Priority::Low;
    let mut data = vec![];
    push_tlv(&mut data, 0x02, encode_priority(&val));
    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
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
fn all_optional_each_alone_velocity() {
    let val = Velocity {
        dx: -100,
        dy: 200,
        dz: 0,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x03, encode_velocity(&val));
    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
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
fn all_optional_each_alone_attitude() {
    let val = Attitude {
        roll: 1.0_f32,
        pitch: 2.0_f32,
        yaw: 3.0_f32,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x04, encode_attitude(&val));
    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
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
fn all_optional_each_alone_timestamp() {
    let val = Timestamp {
        seconds: 999,
        nanos: 1,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x05, encode_timestamp(&val));
    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
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
fn all_optional_each_alone_coordinate() {
    let val = Coordinate {
        lat: 51.5074,
        lon: -0.1278,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x06, encode_coordinate(&val));
    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
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
fn all_optional_each_alone_status() {
    let val = StatusFlags {
        active: false,
        armed: false,
        locked: false,
        mode: 31,
    };
    let mut data = vec![];
    push_tlv(&mut data, 0x07, encode_status_flags(&val));
    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
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
fn all_optional_each_alone_material() {
    let val = Material::Ceramic;
    let mut data = vec![];
    push_tlv(&mut data, 0x08, encode_material(&val));
    let result = OptionalSuite::decode(&mut data.as_slice()).unwrap();
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
fn all_required_8_present() {
    let fixture = required_suite_fixture();
    let data = build_required_bytes(&fixture);
    let result = RequiredSuite::decode(&mut data.as_slice()).unwrap();
    assert_eq!(result, fixture);
}

// --------------------------------------------------
// test 7: all_required_each_missing
// --------------------------------------------------

#[test]
fn all_required_each_missing_color() {
    let s = required_suite_fixture();
    let mut data = vec![];
    // omit key 0x01 (color)
    push_tlv(&mut data, 0x02, encode_priority(&s.priority));
    push_tlv(&mut data, 0x03, encode_velocity(&s.velocity));
    push_tlv(&mut data, 0x04, encode_attitude(&s.attitude));
    push_tlv(&mut data, 0x05, encode_timestamp(&s.timestamp));
    push_tlv(&mut data, 0x06, encode_coordinate(&s.coordinate));
    push_tlv(&mut data, 0x07, encode_status_flags(&s.status));
    push_tlv(&mut data, 0x08, encode_material(&s.material));
    assert!(RequiredSuite::decode(&mut data.as_slice()).is_err());
}

#[test]
fn all_required_each_missing_priority() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&s.color));
    // omit key 0x02 (priority)
    push_tlv(&mut data, 0x03, encode_velocity(&s.velocity));
    push_tlv(&mut data, 0x04, encode_attitude(&s.attitude));
    push_tlv(&mut data, 0x05, encode_timestamp(&s.timestamp));
    push_tlv(&mut data, 0x06, encode_coordinate(&s.coordinate));
    push_tlv(&mut data, 0x07, encode_status_flags(&s.status));
    push_tlv(&mut data, 0x08, encode_material(&s.material));
    assert!(RequiredSuite::decode(&mut data.as_slice()).is_err());
}

#[test]
fn all_required_each_missing_velocity() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&s.color));
    push_tlv(&mut data, 0x02, encode_priority(&s.priority));
    // omit key 0x03 (velocity)
    push_tlv(&mut data, 0x04, encode_attitude(&s.attitude));
    push_tlv(&mut data, 0x05, encode_timestamp(&s.timestamp));
    push_tlv(&mut data, 0x06, encode_coordinate(&s.coordinate));
    push_tlv(&mut data, 0x07, encode_status_flags(&s.status));
    push_tlv(&mut data, 0x08, encode_material(&s.material));
    assert!(RequiredSuite::decode(&mut data.as_slice()).is_err());
}

#[test]
fn all_required_each_missing_attitude() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&s.color));
    push_tlv(&mut data, 0x02, encode_priority(&s.priority));
    push_tlv(&mut data, 0x03, encode_velocity(&s.velocity));
    // omit key 0x04 (attitude)
    push_tlv(&mut data, 0x05, encode_timestamp(&s.timestamp));
    push_tlv(&mut data, 0x06, encode_coordinate(&s.coordinate));
    push_tlv(&mut data, 0x07, encode_status_flags(&s.status));
    push_tlv(&mut data, 0x08, encode_material(&s.material));
    assert!(RequiredSuite::decode(&mut data.as_slice()).is_err());
}

#[test]
fn all_required_each_missing_timestamp() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&s.color));
    push_tlv(&mut data, 0x02, encode_priority(&s.priority));
    push_tlv(&mut data, 0x03, encode_velocity(&s.velocity));
    push_tlv(&mut data, 0x04, encode_attitude(&s.attitude));
    // omit key 0x05 (timestamp)
    push_tlv(&mut data, 0x06, encode_coordinate(&s.coordinate));
    push_tlv(&mut data, 0x07, encode_status_flags(&s.status));
    push_tlv(&mut data, 0x08, encode_material(&s.material));
    assert!(RequiredSuite::decode(&mut data.as_slice()).is_err());
}

#[test]
fn all_required_each_missing_coordinate() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&s.color));
    push_tlv(&mut data, 0x02, encode_priority(&s.priority));
    push_tlv(&mut data, 0x03, encode_velocity(&s.velocity));
    push_tlv(&mut data, 0x04, encode_attitude(&s.attitude));
    push_tlv(&mut data, 0x05, encode_timestamp(&s.timestamp));
    // omit key 0x06 (coordinate)
    push_tlv(&mut data, 0x07, encode_status_flags(&s.status));
    push_tlv(&mut data, 0x08, encode_material(&s.material));
    assert!(RequiredSuite::decode(&mut data.as_slice()).is_err());
}

#[test]
fn all_required_each_missing_status() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&s.color));
    push_tlv(&mut data, 0x02, encode_priority(&s.priority));
    push_tlv(&mut data, 0x03, encode_velocity(&s.velocity));
    push_tlv(&mut data, 0x04, encode_attitude(&s.attitude));
    push_tlv(&mut data, 0x05, encode_timestamp(&s.timestamp));
    push_tlv(&mut data, 0x06, encode_coordinate(&s.coordinate));
    // omit key 0x07 (status)
    push_tlv(&mut data, 0x08, encode_material(&s.material));
    assert!(RequiredSuite::decode(&mut data.as_slice()).is_err());
}

#[test]
fn all_required_each_missing_material() {
    let s = required_suite_fixture();
    let mut data = vec![];
    push_tlv(&mut data, 0x01, encode_color(&s.color));
    push_tlv(&mut data, 0x02, encode_priority(&s.priority));
    push_tlv(&mut data, 0x03, encode_velocity(&s.velocity));
    push_tlv(&mut data, 0x04, encode_attitude(&s.attitude));
    push_tlv(&mut data, 0x05, encode_timestamp(&s.timestamp));
    push_tlv(&mut data, 0x06, encode_coordinate(&s.coordinate));
    push_tlv(&mut data, 0x07, encode_status_flags(&s.status));
    // omit key 0x08 (material)
    assert!(RequiredSuite::decode(&mut data.as_slice()).is_err());
}
