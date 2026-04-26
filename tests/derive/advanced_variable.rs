//! Variable-length field (`varlen = true`) tests for `#[derive(Klv)]`
//!
//! Tests variable-length decoders - those with signature
//! `fn(len: usize) -> impl Fn(&mut &[u8]) -> Result<T>` - across fixed/var
//! mixing, Option wrapping, zero-length edge cases, and Vec<SensorReading>.
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    trait_fallback,
)]
struct MixedVarFixed {
    #[klv(key = 0x01)]
    coord: Coordinate,

    #[klv(key = 0x02)]
    color: Color,

    #[klv(key = 0x03)]
    timestamp: Timestamp,

    #[klv(
        key = 0x04,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = encs::from_string_utf8
    )]
    label: String,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct OptVarString {
    #[klv(
        key = 0x01,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    priority: Priority,
    #[klv(
        key = 0x02,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = encs::from_string_utf8
    )]
    label: Option<String>,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct VarSensorArray {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
    #[klv(
        key = 0x02,
        varlen = true,
        dec = decode_sensor_readings,
        enc = encode_sensor_readings
    )]
    readings: Vec<SensorReading>,
}

#[test]
/// Tests encode/decode roundtrip of a struct mixing fixed-length fields with a `varlen = true` UTF-8 label.
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
    let decoded = MixedVarFixed::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that an `Option<String>` with `varlen = true` decodes to `Some` when its key and non-zero payload are present.
fn option_var_present() {
    // priority=Low(0), label="hello" (5 bytes)
    let data: &[u8] = &[
        0x01, 0x01, 0x00, // priority Low
        0x02, 0x05, b'h', b'e', b'l', b'l', b'o', // label "hello"
    ];
    let result = OptVarString::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.priority, Priority::Low);
    assert_eq!(result.label, Some(String::from("hello")));
}

#[test]
/// Tests that an `Option<String>` with `varlen = true` decodes to `None` when its key is absent from the stream.
fn option_var_absent() {
    // only priority present, label key absent
    let data: &[u8] = &[0x01, 0x01, 0x02]; // priority=High
    let result = OptVarString::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.priority, Priority::High);
    assert_eq!(result.label, None);
}

#[test]
/// Tests that a `varlen` optional with length zero decodes to `Some("")` rather than `None`.
fn option_var_zero_len() {
    // key present but len=0 -> Some("")
    let data: &[u8] = &[
        0x01, 0x01, 0x01, // priority=Medium
        0x02, 0x00, // label key, zero length
    ];
    let result = OptVarString::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.priority, Priority::Medium);
    assert_eq!(result.label, Some(String::from("")));
}

#[test]
/// Tests roundtrip of a variable-length `Vec<SensorReading>` packed into a single TLV payload.
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
    let decoded = VarSensorArray::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.color, Color::Blue);
    assert_eq!(decoded.readings.len(), 3);
    // compare kind exactly; f32 value comparison with tolerance
    for (got, want) in decoded.readings.iter().zip(readings.iter()) {
        assert_eq!(got.kind, want.kind);
        assert!((got.value - want.value).abs() < 1e-3);
    }
}
