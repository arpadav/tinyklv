//! Basic variable-length field decode and roundtrip tests for `#[derive(Klv)]`
//!
//! Tests the `WithString` struct (a fixed `u16` id plus a `size(var)`
//! UTF-8 string). Covers decoding known payloads, zero-length strings,
//! long strings, reversed field order, full encode/decode roundtrip, and
//! missing-required-field errors
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
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
struct WithString {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    id: u16,

    #[klv(
        key = 0x02,
        size(var),
        dec = decs::to_string_utf8,
        enc = encs::from_string_utf8
    )]
    name: String,
}

#[test]
/// Tests decoding a struct containing a fixed `u16` id plus a variable-length UTF-8 string field
fn decode_with_string_klv() {
    let data: &[u8] = &[0x01, 0x02, 0x01, 0x02, 0x02, 0x03, 0x4B, 0x4C, 0x56];
    let result = WithString::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.id, 258);
    assert_eq!(result.name, "KLV");
}

#[test]
/// Verifies decoding a `"Hello World!"` UTF-8 string payload of non-trivial length
fn decode_hello_world() {
    let name = b"Hello World!";
    let mut data = vec![0x01_u8, 0x02, 0x00, 42, 0x02, name.len() as u8];
    data.extend_from_slice(name);
    let result = WithString::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.id, 42);
    assert_eq!(result.name, "Hello World!");
}

#[test]
/// Verifies encode/decode roundtrip for a struct with a variable-length UTF-8 string field
fn encode_with_string_roundtrip() {
    let original = WithString {
        id: 1234,
        name: String::from("MISSION01"),
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithString::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests decoding a zero-length string field (length prefix = 0) as an empty `String`
fn decode_empty_string_field() {
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x00, 0x02, 0x00];
    let result = WithString::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.id, 0);
    assert_eq!(result.name, "");
}

#[test]
/// Tests decoding a 26-byte UTF-8 alphabet string to exercise a longer variable-length payload
fn decode_long_string() {
    let name = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut data = vec![0x01_u8, 0x02, 0x00, 0x01, 0x02, name.len() as u8];
    data.extend_from_slice(name);
    let result = WithString::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.id, 1);
    assert_eq!(result.name, "ABCDEFGHIJKLMNOPQRSTUVWXYZ");
}

#[test]
/// Verifies that a variable-length string field and a fixed integer field decode correctly when presented in reverse order
fn decode_string_reversed_field_order() {
    let name = b"rev";
    let mut data = vec![0x02_u8, name.len() as u8];
    data.extend_from_slice(name);
    data.extend_from_slice(&[0x01, 0x02, 0x00, 0x07]);
    let result = WithString::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.id, 7);
    assert_eq!(result.name, "rev");
}

#[test]
/// Tests that decoding errors when the required string field (key `0x02`) is missing from the input
fn decode_missing_required_string_fails() {
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x01];
    let result = WithString::decode_value(&mut &data[..]);
    assert!(result.is_err());
}
