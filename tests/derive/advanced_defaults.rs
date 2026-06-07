//! Tests for `default(typ=...)` container attribute and `default` / `default = <expr>` field attributes
//!
//! Covers container-level type defaults (eliminating per-field dec/enc),
//! field-level `default = <expr>` values used when a key is absent,
//! field-level bare `default` which calls `Default::default()` on the field
//! type, override when the key is present, and non-KLV fields falling back
//! to `Default::default()`
// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = Color,    dec = Color::decode_value,    enc = Color::encode_value),
    default(typ = Priority, dec = Priority::decode_value, enc = Priority::encode_value),
)]
struct DefaultTyped {
    #[klv(key = 0x01)]
    /// No per-field dec/enc - resolved from container defaults
    color: Color,

    #[klv(key = 0x02)]
    /// No per-field dec/enc - resolved from container defaults
    priority: Priority,

    #[klv(
        key = 0x03,
        dec = Velocity::decode_value,
        enc = Velocity::encode_value,
    )]
    /// Explicit dec/enc still works alongside container defaults
    velocity: Velocity,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct DefaultExprColor {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
        default = Color::Red,
    )]
    color: Color,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct DefaultExprTimestamp {
    #[klv(
        key = 0x01,
        dec = Timestamp::decode_value,
        enc = Timestamp::encode_value,
        default = Timestamp { seconds: 0, nanos: 0 }
    )]
    timestamp: Timestamp,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    allow_unimplemented_encode,
)]
struct WithNonKlvField {
    #[klv(
        key = 0x01,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Color,
    // No #[klv] - should resolve to Default::default() = 0
    counter: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
enum Flavor {
    #[default]
    Vanilla,
    Chocolate,
    Strawberry,
    Mint,
}
impl tinyklv::DecodeValue<&[u8]> for Flavor {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let v = decb::u8(input)?;
        match v {
            0 => Ok(Flavor::Vanilla),
            1 => Ok(Flavor::Chocolate),
            2 => Ok(Flavor::Strawberry),
            3 => Ok(Flavor::Mint),
            _ => Err(winnow::error::ParserError::from_input(input)),
        }
    }
}
impl tinyklv::EncodeValue for Flavor {
    fn encode_value(&self, out: &mut Vec<u8>) {
        let v = match self {
            Flavor::Vanilla => 0_u8,
            Flavor::Chocolate => 1,
            Flavor::Strawberry => 2,
            Flavor::Mint => 3,
        };
        encb::u8(v, out);
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Custom struct with a non-trivial `Default` impl - distinguishes bare
/// `default` from an inline `default = <expr>` with the same literal values
pub struct Calibration {
    pub gain: f32,
    pub offset: i16,
}
impl Default for Calibration {
    fn default() -> Self {
        Calibration {
            gain: 1.0,
            offset: 0,
        }
    }
}
impl tinyklv::DecodeValue<&[u8]> for Calibration {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let gain = decb::be_f32(input)?;
        let offset = decb::be_i16(input)?;
        Ok(Calibration { gain, offset })
    }
}
impl tinyklv::EncodeValue for Calibration {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_f32(self.gain, out);
        encb::be_i16(self.offset, out);
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct DefaultBareU32 {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
        default,
    )]
    counter: u32,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct DefaultBareStruct {
    #[klv(
        key = 0x01,
        dec = Calibration::decode_value,
        enc = Calibration::encode_value,
        default
    )]
    cal: Calibration,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct DefaultBareEnum {
    #[klv(
        key = 0x01,
        dec = Flavor::decode_value,
        enc = Flavor::encode_value,
        default
    )]
    flavor: Flavor,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct DefaultBareAndExpr {
    #[klv(
        key = 0x01,
        dec = Flavor::decode_value,
        enc = Flavor::encode_value,
        default
    )]
    flavor: Flavor,
    #[klv(
        key = 0x02,
        dec = Calibration::decode_value,
        enc = Calibration::encode_value,
        default = Calibration { gain: 2.5, offset: -7 }
    )]
    cal: Calibration,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct DefaultBareOptionWrapped {
    #[klv(
        key = 0x01,
        dec = Flavor::decode_value,
        enc = Flavor::encode_value,
        default
    )]
    flavor: Option<Flavor>,
}

/// Build a single KLV triple `[key:1][len:1][value:N]` from a pre-encoded value
///
/// Prepends the 1-byte key and 1-byte length header before `value`, producing
/// a complete TLV triple suitable for hand-building test streams
fn tlv(key: u8, value: Vec<u8>) -> Vec<u8> {
    let mut out = vec![key, value.len() as u8];
    out.extend(value);
    out
}

#[test]
/// Tests that `default(typ = ...)` container attributes resolve `dec`/`enc` for fields that omit them
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
    let mut color_v = Vec::new();
    original.color.encode_value(&mut color_v);
    stream.extend(tlv(0x01, color_v));
    let mut priority_v = Vec::new();
    original.priority.encode_value(&mut priority_v);
    stream.extend(tlv(0x02, priority_v));
    let mut velocity_v = Vec::new();
    original.velocity.encode_value(&mut velocity_v);
    stream.extend(tlv(0x03, velocity_v));
    let decoded = DefaultTyped::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(decoded, original, "container-default decode should match");
}

#[test]
/// Verifies encode/decode roundtrip when most fields rely on container-level `default(typ = ...)` codecs
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
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = DefaultTyped::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original, "roundtrip via container-level defaults");
}

#[test]
/// Tests that `default = Color::Red` supplies the fallback when the field's key is absent from the stream
fn default_expr_color_enum_absent() {
    // Stream has no key 0x01 - `default = Color::Red` should be used
    let result = DefaultExprColor::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.color,
        Color::Red,
        "absent key should yield `default = <expr>` value"
    );
}

#[test]
/// Tests that a present key overrides the `default = Color::Red` fallback at decode time
fn default_expr_color_enum_present() {
    // Stream has key 0x01 = Color::Blue - decoded value overrides default expression
    let mut blue_v = Vec::new();
    Color::Blue.encode_value(&mut blue_v);
    let stream = tlv(0x01, blue_v);
    let result = DefaultExprColor::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.color,
        Color::Blue,
        "present key should yield decoded value"
    );
}

#[test]
/// Tests that a struct-valued `default = Timestamp { ... }` fallback is applied when the key is absent
fn default_expr_timestamp_struct_absent() {
    let zero = Timestamp {
        seconds: 0,
        nanos: 0,
    };
    let result = DefaultExprTimestamp::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.timestamp, zero,
        "absent key should yield `default = <expr>` Timestamp"
    );
}

#[test]
/// Tests that decoding a present `Timestamp` key overrides the struct-valued `default = <expr>` fallback
fn default_expr_timestamp_struct_present() {
    let ts = Timestamp {
        seconds: 1_700_000_000,
        nanos: 12_345,
    };
    let mut ts_v = Vec::new();
    ts.encode_value(&mut ts_v);
    let stream = tlv(0x01, ts_v);
    let result = DefaultExprTimestamp::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.timestamp, ts,
        "present key should yield decoded Timestamp"
    );
}

#[test]
/// Verifies explicitly that the decoded value (`Color::Blue`) wins over the `default = Color::Red` fallback when both are available
fn default_expr_overridden_when_present() {
    let mut blue_v = Vec::new();
    Color::Blue.encode_value(&mut blue_v);
    let stream = tlv(0x01, blue_v);
    let result = DefaultExprColor::decode_value(&mut stream.as_slice()).unwrap();
    assert_ne!(
        result.color,
        Color::Red,
        "`default = <expr>` should be overridden by decoded value"
    );
    assert_eq!(
        result.color,
        Color::Blue,
        "decoded Color::Blue must win over Color::Red default"
    );
}

#[test]
/// Tests bare `default` on a primitive field - absent key yields `u32::default() == 0`
fn default_bare_primitive_u32_absent() {
    let result = DefaultBareU32::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.counter, 0,
        "bare `default` on u32 must yield u32::default() == 0"
    );
}

#[test]
/// Tests bare `default` on a primitive field - present key overrides the `Default::default()` fallback
fn default_bare_primitive_u32_present() {
    let mut counter_v = Vec::new();
    encb::be_u32(42, &mut counter_v);
    let stream = tlv(0x01, counter_v);
    let result = DefaultBareU32::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(
        result.counter, 42,
        "present key should override bare `default`"
    );
}

#[test]
/// Tests bare `default` on a custom struct - confirms it actually calls the type's `Default` impl
///
/// `Calibration::default()` is `{ gain: 1.0, offset: 0 }`, distinct from the
/// struct's zero-initialized form, proving the codegen dispatches through the
/// `Default` trait rather than bit-zero memory
fn default_bare_custom_struct_absent() {
    let result = DefaultBareStruct::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.cal,
        Calibration::default(),
        "bare `default` must call `<Calibration as Default>::default()`"
    );
    assert_eq!(result.cal.gain, 1.0);
    assert_eq!(result.cal.offset, 0);
}

#[test]
/// Tests bare `default` on a custom struct when the key is present - decoded value wins
fn default_bare_custom_struct_present() {
    let cal = Calibration {
        gain: 3.15,
        offset: -42,
    };
    let mut cal_v = Vec::new();
    cal.encode_value(&mut cal_v);
    let stream = tlv(0x01, cal_v);
    let result = DefaultBareStruct::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(result.cal, cal, "present key overrides bare `default`");
}

#[test]
/// Tests bare `default` on an enum with `#[derive(Default)]` and `#[default]` variant
fn default_bare_enum_with_derive_default_absent() {
    let result = DefaultBareEnum::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.flavor,
        Flavor::Vanilla,
        "bare `default` must yield the `#[default]` variant"
    );
}

#[test]
/// Tests a single struct that mixes bare `default` and `default = <expr>` on different fields
fn default_bare_and_expr_mixed_absent() {
    let result = DefaultBareAndExpr::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.flavor,
        Flavor::Vanilla,
        "bare `default` field -> Default::default()"
    );
    assert_eq!(
        result.cal,
        Calibration {
            gain: 2.5,
            offset: -7
        },
        "`default = <expr>` field -> inlined expression"
    );
}

#[test]
/// Tests that bare `default` on an `Option<T>` field unwraps through
/// `unwrap_option_type` and calls `<T>::default()` (not `<Option<T>>::default()`)
fn default_bare_option_wrapped_absent() {
    let result = DefaultBareOptionWrapped::decode_value(&mut [].as_slice()).unwrap();
    assert_eq!(
        result.flavor,
        Some(Flavor::Vanilla),
        "Option<Flavor> with bare `default` should be `Some(Flavor::default())`"
    );
}

#[test]
/// Tests that a struct field without `#[klv(...)]` resolves to `Default::default()` while other fields decode normally
fn non_klv_field_default() {
    let mut green_v = Vec::new();
    Color::Green.encode_value(&mut green_v);
    let stream = tlv(0x01, green_v);
    let result = WithNonKlvField::decode_value(&mut stream.as_slice()).unwrap();
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
/// Tests that unknown keys in the stream do not perturb the default-valued non-KLV field
fn non_klv_field_unaffected_by_unknown_keys() {
    let mut alpha_v = Vec::new();
    Color::Alpha.encode_value(&mut alpha_v);
    let mut stream: Vec<u8> = tlv(0x01, alpha_v);
    stream.extend_from_slice(&[0xFF, 0x02, 0xAB, 0xCD]);

    let result = WithNonKlvField::decode_value(&mut stream.as_slice()).unwrap();
    assert_eq!(result.color, Color::Alpha);
    assert_eq!(
        result.counter, 0,
        "non-KLV counter must not be affected by unknown keys"
    );
}
