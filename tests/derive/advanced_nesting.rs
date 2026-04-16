//! Tests for nested `#[derive(Klv)]` structs - inner types that are themselves
//! Klv-derived, exercising full encode/decode symmetry across composition levels.
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// test 1: single level of Klv-derived nesting
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct SensorModule {
    #[klv(key = 0x01, dec = decode_sensor_reading, enc = encode_sensor_reading)]
    reading: SensorReading,
    #[klv(key = 0x02, dec = decode_color, enc = encode_color)]
    indicator: Color,
}

fn encode_sensor_module(v: &SensorModule) -> Vec<u8> {
    v.encode_value()
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Platform {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    id: u16,
    #[klv(key = 0x02, dec = SensorModule::decode, enc = encode_sensor_module)]
    sensor: SensorModule,
    #[klv(key = 0x03, dec = decode_coordinate, enc = encode_coordinate)]
    position: Coordinate,
}

#[test]
fn nested_klv_derived_roundtrip() {
    let original = Platform {
        id: 42,
        sensor: SensorModule {
            reading: SensorReading {
                kind: SensorKind::Temperature,
                value: 23.5,
            },
            indicator: Color::Green,
        },
        position: Coordinate {
            lat: 48.8566,
            lon: 2.3522,
        },
    };
    let encoded = original.encode_value();
    let decoded = Platform::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn nested_klv_derived_roundtrip_extreme_values() {
    let original = Platform {
        id: u16::MAX,
        sensor: SensorModule {
            reading: SensorReading {
                kind: SensorKind::Vibration,
                value: f32::MAX,
            },
            indicator: Color::Unknown(0xDEAD),
        },
        position: Coordinate {
            lat: -90.0,
            lon: -180.0,
        },
    };
    let encoded = original.encode_value();
    let decoded = Platform::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn nested_klv_derived_roundtrip_zero_values() {
    let original = Platform {
        id: 0,
        sensor: SensorModule {
            reading: SensorReading {
                kind: SensorKind::Pressure,
                value: 0.0,
            },
            indicator: Color::Red,
        },
        position: Coordinate { lat: 0.0, lon: 0.0 },
    };
    let encoded = original.encode_value();
    let decoded = Platform::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// test 2: three levels of Klv-derived nesting
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Core {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    value: u32,
}

fn encode_core(v: &Core) -> Vec<u8> {
    v.encode_value()
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Module {
    #[klv(key = 0x01, dec = Core::decode, enc = encode_core)]
    core: Core,
    #[klv(key = 0x02, dec = decode_color, enc = encode_color)]
    color: Color,
}

fn encode_module(v: &Module) -> Vec<u8> {
    v.encode_value()
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct System {
    #[klv(key = 0x01, dec = Module::decode, enc = encode_module)]
    module: Module,
    #[klv(key = 0x02, dec = decode_timestamp, enc = encode_timestamp)]
    timestamp: Timestamp,
}

#[test]
fn nested_two_deep() {
    let original = System {
        module: Module {
            core: Core { value: 0xCAFE_BABE },
            color: Color::Blue,
        },
        timestamp: Timestamp {
            seconds: 1_700_000_000,
            nanos: 500,
        },
    };
    let encoded = original.encode_value();
    let decoded = System::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn nested_two_deep_min_values() {
    let original = System {
        module: Module {
            core: Core { value: 0 },
            color: Color::Alpha,
        },
        timestamp: Timestamp {
            seconds: 0,
            nanos: 0,
        },
    };
    let encoded = original.encode_value();
    let decoded = System::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn nested_two_deep_max_values() {
    let original = System {
        module: Module {
            core: Core { value: u32::MAX },
            color: Color::Unknown(u16::MAX),
        },
        timestamp: Timestamp {
            seconds: u32::MAX,
            nanos: u16::MAX,
        },
    };
    let encoded = original.encode_value();
    let decoded = System::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// test 3: optional Klv-derived inner field
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct PlatformOptional {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    id: u16,
    #[klv(key = 0x02, dec = SensorModule::decode, enc = encode_sensor_module)]
    sensor: Option<SensorModule>,
}

#[test]
fn nested_optional_sensor_present() {
    let original = PlatformOptional {
        id: 7,
        sensor: Some(SensorModule {
            reading: SensorReading {
                kind: SensorKind::Humidity,
                value: 55.0,
            },
            indicator: Color::Alpha,
        }),
    };
    let encoded = original.encode_value();
    let decoded = PlatformOptional::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
    assert!(decoded.sensor.is_some());
}

#[test]
fn nested_optional_sensor_absent() {
    let original = PlatformOptional {
        id: 99,
        sensor: None,
    };
    let encoded = original.encode_value();
    let decoded = PlatformOptional::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
    assert!(decoded.sensor.is_none());
}

#[test]
fn nested_optional_roundtrip_toggle() {
    let with_sensor = PlatformOptional {
        id: 1,
        sensor: Some(SensorModule {
            reading: SensorReading {
                kind: SensorKind::Temperature,
                value: -10.0,
            },
            indicator: Color::Green,
        }),
    };
    let without_sensor = PlatformOptional {
        id: 2,
        sensor: None,
    };

    let enc_with = with_sensor.encode_value();
    let enc_without = without_sensor.encode_value();

    assert_ne!(enc_with, enc_without);
    assert_eq!(
        PlatformOptional::decode(&mut enc_with.as_slice()).unwrap(),
        with_sensor
    );
    assert_eq!(
        PlatformOptional::decode(&mut enc_without.as_slice()).unwrap(),
        without_sensor
    );
}

// --------------------------------------------------
// test 4: Klv-derived inner with enum fields
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct StatusInner {
    #[klv(key = 0x01, dec = decode_color, enc = encode_color)]
    color: Color,
    #[klv(key = 0x02, dec = decode_priority, enc = encode_priority)]
    priority: Priority,
}

fn encode_status_inner(v: &StatusInner) -> Vec<u8> {
    v.encode_value()
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct StatusOuter {
    #[klv(key = 0x01, dec = StatusInner::decode, enc = encode_status_inner)]
    status: StatusInner,
    #[klv(key = 0x02, dec = decode_velocity, enc = encode_velocity)]
    velocity: Velocity,
}

#[test]
fn nested_with_enum_field() {
    let original = StatusOuter {
        status: StatusInner {
            color: Color::Red,
            priority: Priority::Critical,
        },
        velocity: Velocity {
            dx: 100,
            dy: -50,
            dz: 0,
        },
    };
    let encoded = original.encode_value();
    let decoded = StatusOuter::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn nested_with_enum_field_all_variants() {
    let cases = [
        (Color::Red, Priority::Low),
        (Color::Green, Priority::Medium),
        (Color::Blue, Priority::High),
        (Color::Alpha, Priority::Critical),
        (Color::Unknown(0x00FF), Priority::Low),
    ];
    for (color, priority) in cases {
        let original = StatusOuter {
            status: StatusInner { color, priority },
            velocity: Velocity {
                dx: 1,
                dy: 2,
                dz: 3,
            },
        };
        let encoded = original.encode_value();
        let decoded = StatusOuter::decode(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded, original);
    }
}

#[test]
fn nested_with_enum_field_extreme_velocity() {
    let original = StatusOuter {
        status: StatusInner {
            color: Color::Blue,
            priority: Priority::High,
        },
        velocity: Velocity {
            dx: i16::MIN,
            dy: i16::MAX,
            dz: 0,
        },
    };
    let encoded = original.encode_value();
    let decoded = StatusOuter::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
