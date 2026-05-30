//! Mixed-type field tests for `#[derive(Klv)]`
//!
//! Tests the `Mixed` struct (a `u8` byte value, a `u32` integer, a `varlen`
//! UTF-8 string, and an optional `u16`). Covers decode with all fields
//! present, absent optional, reversed field order, missing required field,
//! and full encode/decode roundtrip with and without the optional
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::Klv;
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
struct Mixed {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    byte_val: u8,
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    int_val: u32,
    #[klv(
        key = 0x03,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8
    )]
    name: String,
    #[klv(
        key = 0x04,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    optional_short: Option<u16>,
}

#[test]
/// Tests decoding a struct mixing fixed-width integers, a variable-length string, and an optional field with all keys present
fn decode_all_fields_present() {
    let name = b"KLV";
    let mut data = vec![
        0x01_u8,
        0x01,
        0xAB,
        0x02,
        0x04,
        0x00,
        0x01,
        0x02,
        0x03,
        0x03,
        name.len() as u8,
    ];
    data.extend_from_slice(name);
    data.extend_from_slice(&[0x04, 0x02, 0x12, 0x34]);
    let result = Mixed::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.byte_val, 0xAB);
    assert_eq!(result.int_val, 0x00010203);
    assert_eq!(result.name, "KLV");
    assert_eq!(result.optional_short, Some(0x1234));
}

#[test]
/// Verifies that in a mixed-type struct the optional field decodes to `None` when its key is absent
fn decode_optional_absent() {
    let name = b"TEST";
    let mut data = vec![
        0x01_u8,
        0x01,
        0x01,
        0x02,
        0x04,
        0xFF,
        0xFF,
        0xFF,
        0xFF,
        0x03,
        name.len() as u8,
    ];
    data.extend_from_slice(name);
    let result = Mixed::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.byte_val, 0x01);
    assert_eq!(result.int_val, u32::MAX);
    assert_eq!(result.name, "TEST");
    assert_eq!(result.optional_short, None);
}

#[test]
/// Tests encode/decode roundtrip for a struct mixing integers, a `String`, and a `Some` optional
fn roundtrip_mixed_types() {
    let original = Mixed {
        byte_val: 0xAB,
        int_val: 0xDEAD_BEEF,
        name: String::from("MISSION"),
        optional_short: Some(0x1234),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = Mixed::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests roundtrip for a mixed-type struct with zeros, an empty string, and `None` optional
fn roundtrip_mixed_types_no_optional() {
    let original = Mixed {
        byte_val: 0,
        int_val: 0,
        name: String::new(),
        optional_short: None,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = Mixed::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests that a missing required `byte_val` (key `0x01`) causes decode to return `Err` even when other fields are present
fn decode_missing_required_fails() {
    let name = b"X";
    let mut data = vec![0x02_u8, 0x04, 0x00, 0x00, 0x00, 0x01, 0x03, 1];
    data.extend_from_slice(name);
    let result = Mixed::decode_value(&mut data.as_slice());
    assert!(result.is_err());
}

#[test]
/// Verifies mixed-type decoding when the optional, variable, and fixed fields appear in reverse key order
fn decode_reversed_field_order() {
    let name = b"rev";
    let mut data = vec![0x04_u8, 0x02, 0x00, 0x07, 0x03, name.len() as u8];
    data.extend_from_slice(name);
    data.extend_from_slice(&[0x02, 0x04, 0x00, 0x00, 0x00, 0x05, 0x01, 0x01, 0x09]);
    let result = Mixed::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.byte_val, 9);
    assert_eq!(result.int_val, 5);
    assert_eq!(result.name, "rev");
    assert_eq!(result.optional_short, Some(7));
}
