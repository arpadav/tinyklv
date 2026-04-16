//! Variable-length field (`var = true`) tests for `#[derive(Klv)]`
//!
//! Tests variable-length decoders - those with signature
//! `fn(len: usize) -> impl Fn(&mut &[u8]) -> Result<T>` - across fixed/var
//! mixing, Option wrapping, zero-length edge cases, and Vec<SensorReading>.
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
struct MixedVarFixed {
    #[klv(key = 0x01, dec = decode_coordinate, enc = encode_coordinate)]
    coord: Coordinate,
    #[klv(key = 0x02, dec = decode_color, enc = encode_color)]
    color: Color,
    #[klv(key = 0x03, dec = decode_timestamp, enc = encode_timestamp)]
    timestamp: Timestamp,
    #[klv(
        key = 0x04,
        var = true,
        dec = tinyklv::dec::binary::to_string_utf8,
        enc = tinyklv::enc::string::from_string_utf8
    )]
    label: String,
}

// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct OptVarString {
    #[klv(key = 0x01, dec = decode_priority, enc = encode_priority)]
    priority: Priority,
    #[klv(
        key = 0x02,
        var = true,
        dec = tinyklv::dec::binary::to_string_utf8,
        enc = tinyklv::enc::string::from_string_utf8
    )]
    label: Option<String>,
}

// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct VarSensorArray {
    #[klv(key = 0x01, dec = decode_color, enc = encode_color)]
    color: Color,
    #[klv(
        key = 0x02,
        var = true,
        dec = decode_sensor_readings,
        enc = encode_sensor_readings
    )]
    readings: Vec<SensorReading>,
}

// --------------------------------------------------
// tests
// --------------------------------------------------

#[test]
fn mixed_var_fixed_roundtrip() {
    let original = MixedVarFixed {
        coord: Coordinate {
            lat: 48.8566,
            lon: 2.3522,
        },
        color: Color::Green,
        timestamp: Timestamp {
            seconds: 1_000_000,
            nanos: 250,
        },
        label: String::from("Paris"),
    };
    let encoded = original.encode_value();
    let decoded = MixedVarFixed::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn option_var_present() {
    // priority=Low(0), label="hello" (5 bytes)
    let data: &[u8] = &[
        0x01, 0x01, 0x00, // priority Low
        0x02, 0x05, b'h', b'e', b'l', b'l', b'o', // label "hello"
    ];
    let result = OptVarString::decode(&mut &data[..]).unwrap();
    assert_eq!(result.priority, Priority::Low);
    assert_eq!(result.label, Some(String::from("hello")));
}

#[test]
fn option_var_absent() {
    // only priority present, label key absent
    let data: &[u8] = &[0x01, 0x01, 0x02]; // priority=High
    let result = OptVarString::decode(&mut &data[..]).unwrap();
    assert_eq!(result.priority, Priority::High);
    assert_eq!(result.label, None);
}

#[test]
fn option_var_zero_len() {
    // key present but len=0 → Some("")
    let data: &[u8] = &[
        0x01, 0x01, 0x01, // priority=Medium
        0x02, 0x00, // label key, zero length
    ];
    let result = OptVarString::decode(&mut &data[..]).unwrap();
    assert_eq!(result.priority, Priority::Medium);
    assert_eq!(result.label, Some(String::from("")));
}

#[test]
fn var_sensor_array() {
    // 3 sensor readings × 5 bytes = 15 bytes payload
    let readings = vec![
        SensorReading {
            kind: SensorKind::Temperature,
            value: 36.6,
        },
        SensorReading {
            kind: SensorKind::Pressure,
            value: 1013.25,
        },
        SensorReading {
            kind: SensorKind::Humidity,
            value: 55.0,
        },
    ];
    let original = VarSensorArray {
        color: Color::Blue,
        readings: readings.clone(),
    };
    let encoded = original.encode_value();
    let decoded = VarSensorArray::decode(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.color, Color::Blue);
    assert_eq!(decoded.readings.len(), 3);
    // compare kind exactly; f32 value comparison with tolerance
    for (got, want) in decoded.readings.iter().zip(readings.iter()) {
        assert_eq!(got.kind, want.kind);
        assert!((got.value - want.value).abs() < 1e-3);
    }
}
