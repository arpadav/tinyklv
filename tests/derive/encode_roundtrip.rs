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
)]
struct AllNumerics {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    u8_field: u8,
    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    u16_field: u16,
    #[klv(
        key = 0x03,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    u32_field: u32,
    #[klv(
        key = 0x04,
        dec = decb::be_u64,
        enc = *encb::be_u64,
    )]
    u64_field: u64,
    #[klv(
        key = 0x05,
        dec = decb::be_i16,
        enc = *encb::be_i16,
    )]
    i16_field: i16,
    #[klv(
        key = 0x06,
        dec = decb::be_i32,
        enc = *encb::be_i32,
    )]
    i32_field: i32,
}

#[test]
/// Tests encode/decode roundtrip across `u8/u16/u32/u64/i16/i32` fields with typical non-zero values including negatives.
fn all_numerics_roundtrip_typical() {
    let original = AllNumerics {
        u8_field: 0xAB,
        u16_field: 0x1234,
        u32_field: 0xDEAD_BEEF,
        u64_field: 0x0102_0304_0506_0708,
        i16_field: -1000,
        i32_field: -100_000,
    };
    let encoded = original.encode_value();
    let decoded = AllNumerics::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that all-zero numeric fields roundtrip through encode/decode unchanged.
fn all_numerics_roundtrip_zeros() {
    let original = AllNumerics {
        u8_field: 0,
        u16_field: 0,
        u32_field: 0,
        u64_field: 0,
        i16_field: 0,
        i32_field: 0,
    };
    let encoded = original.encode_value();
    let decoded = AllNumerics::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that type-maximum values for every numeric field roundtrip correctly.
fn all_numerics_roundtrip_max_values() {
    let original = AllNumerics {
        u8_field: u8::MAX,
        u16_field: u16::MAX,
        u32_field: u32::MAX,
        u64_field: u64::MAX,
        i16_field: i16::MAX,
        i32_field: i32::MAX,
    };
    let encoded = original.encode_value();
    let decoded = AllNumerics::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that `i16::MIN` and `i32::MIN` roundtrip correctly (exercises two's-complement sign-bit boundary).
fn all_numerics_roundtrip_min_signed() {
    let original = AllNumerics {
        u8_field: 0,
        u16_field: 0,
        u32_field: 0,
        u64_field: 0,
        i16_field: i16::MIN,
        i32_field: i32::MIN,
    };
    let encoded = original.encode_value();
    let decoded = AllNumerics::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// struct with optional field roundtrip
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct WithOptionalRoundtrip {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    required: u32,
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    optional: Option<u32>,
}

#[test]
/// Verifies roundtrip when the optional field carries `Some(value)`.
fn optional_some_roundtrip() {
    let original = WithOptionalRoundtrip {
        required: 0xABCD,
        optional: Some(0x1234),
    };
    let encoded = original.encode_value();
    let decoded = WithOptionalRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Verifies that a `None` optional is omitted on encode and decodes back to `None`.
fn optional_none_roundtrip() {
    let original = WithOptionalRoundtrip {
        required: 0xABCD,
        optional: None,
    };
    let encoded = original.encode_value();
    let decoded = WithOptionalRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct WithStringRoundtrip {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    id: u16,
    #[klv(
        key = 0x02,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8
    )]
    name: String,
}

#[test]
/// Tests roundtrip of a plain ASCII UTF-8 string field.
fn string_field_roundtrip_ascii() {
    let original = WithStringRoundtrip {
        id: 1,
        name: String::from("MISSION01"),
    };
    let encoded = original.encode_value();
    let decoded = WithStringRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests roundtrip when the UTF-8 string field is empty.
fn string_field_roundtrip_empty() {
    let original = WithStringRoundtrip {
        id: 0,
        name: String::new(),
    };
    let encoded = original.encode_value();
    let decoded = WithStringRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests roundtrip for a UTF-8 string containing multi-byte characters and an emoji code point.
fn string_field_roundtrip_unicode() {
    let original = WithStringRoundtrip {
        id: 42,
        name: String::from("Héllo 🌍"),
    };
    let encoded = original.encode_value();
    let decoded = WithStringRoundtrip::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
