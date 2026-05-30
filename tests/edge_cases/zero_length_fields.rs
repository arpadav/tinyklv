//! Zero-length field edge-case tests
//!
//! Verifies that every lengthed decoder and encoder handles `len = 0`
//! without panicking or consuming bytes.  Covers UTF-8 and UTF-16 LE string
//! decoders, fixed-width binary lengthed decoders (`be_u16`, `be_u32`,
//! `be_u64`), and their encoder counterparts.  Also tests the derive-macro
//! path: a `ZeroLenOptional` struct where a zero-length numeric key maps to
//! `None` and a zero-length string key maps to `Some("")`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::codecs::string::dec::{to_string_utf8, to_string_utf16_le};
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

#[test]
/// Tests that `to_string_utf8(0)` returns an empty string without consuming any input bytes
fn to_string_utf8_zero_len_returns_empty() {
    let mut input: &[u8] = &[0x41, 0x42, 0x43]; // "ABC" - should not be consumed
    let result = to_string_utf8(0)(&mut input).unwrap();
    assert_eq!(result, "", "len=0 should return empty string");
    // Input must be unconsumed
    assert_eq!(input, &[0x41, 0x42, 0x43]);
}

#[test]
/// Tests that `to_string_utf8(0)` succeeds and returns an empty string on empty input
fn to_string_utf8_zero_len_on_empty_returns_empty() {
    let mut input: &[u8] = &[];
    let result = to_string_utf8(0)(&mut input).unwrap();
    assert_eq!(result, "");
}

#[test]
/// Tests that `to_string_utf16_le(0)` returns an empty string without consuming any input bytes
fn to_string_utf16_le_zero_len_returns_empty() {
    let mut input: &[u8] = &[0x41, 0x00];
    let result = to_string_utf16_le(0)(&mut input).unwrap();
    assert_eq!(result, "");
    // Input unconsumed
    assert_eq!(input, &[0x41, 0x00]);
}

#[test]
/// Tests that `be_u16_lengthed(0)` returns `0u16` without consuming input
fn be_u16_lengthed_zero_returns_zero() {
    let mut input: &[u8] = &[0x01, 0x02]; // should not be consumed
    let result = decb::be_u16_lengthed(0)(&mut input).unwrap();
    assert_eq!(result, 0_u16);
    assert_eq!(input, &[0x01, 0x02]);
}

#[test]
/// Tests that `be_u32_lengthed(0)` returns `0u32` without consuming input
fn be_u32_lengthed_zero_returns_zero() {
    let mut input: &[u8] = &[0xAB, 0xCD, 0xEF, 0x01];
    let result = decb::be_u32_lengthed(0)(&mut input).unwrap();
    assert_eq!(result, 0_u32);
}

#[test]
/// Tests that `be_u64_lengthed(0)` returns `0u64` without consuming input
fn be_u64_lengthed_zero_returns_zero() {
    let mut input: &[u8] = &[0x01; 8];
    let result = decb::be_u64_lengthed(0)(&mut input).unwrap();
    assert_eq!(result, 0_u64);
}

#[test]
/// Tests that `enc::binary::be_u16_lengthed(0)` produces an empty byte vector regardless of value
fn enc_be_u16_lengthed_zero_produces_empty() {
    let mut result = Vec::new();
    encb::be_u16_lengthed(0)(0x1234_u16, &mut result);
    assert!(result.is_empty());
}

#[test]
/// Tests that `enc::binary::le_u32_lengthed(0)` produces an empty byte vector regardless of value
fn enc_le_u32_lengthed_zero_produces_empty() {
    let mut result = Vec::new();
    encb::le_u32_lengthed(0)(0xDEADBEEF_u32, &mut result);
    assert!(result.is_empty());
}

#[test]
/// Tests that `enc::binary::be_u64_lengthed(0)` produces an empty byte vector regardless of value
fn enc_be_u64_lengthed_zero_produces_empty() {
    let mut result = Vec::new();
    encb::be_u64_lengthed(0)(u64::MAX, &mut result);
    assert!(result.is_empty());
}

/// KLV struct used to verify zero-length field semantics via the derive macro
///
/// A len=0 numeric key causes `be_u16` to fail on the empty sub-slice,
/// so `numeric` stays `None`.  A len=0 string key calls `to_string_utf8(0)`
/// which succeeds and returns `""`, so `label` becomes `Some("")`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct ZeroLenOptional {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    numeric: Option<u16>,
    #[klv(
        key = 0x02,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8
    )]
    label: Option<String>,
}

#[test]
/// Tests that a zero-length numeric optional becomes `None` (decoder needs bytes) while a zero-length string optional becomes `Some("")`
fn derive_optional_numeric_zero_len_in_stream_gives_none() {
    // key=0x01 with len=0: be_u16 gets an empty sub-slice -> fails -> Option stays None
    // key=0x02 with len=0: to_string_utf8(0) returns "" -> Some("")
    let data: &[u8] = &[
        0x01, 0x00, // key=1, len=0
        0x02, 0x00, // key=2, len=0
    ];
    let result = ZeroLenOptional::decode_value(&mut &data[..]).unwrap();
    assert_eq!(
        result.numeric, None,
        "be_u16 on 0-byte slice should fail -> None"
    );
    assert_eq!(
        result.label,
        Some(String::new()),
        "string len=0 -> Some(\"\")"
    );
}

#[test]
/// Tests that a valid `u16` decodes followed by a zero-length string yielding `Some("")`
fn derive_optional_numeric_normal_then_zero_len() {
    // key=0x01 with len=2: valid u16
    // key=0x02 with len=0: empty string
    let data: &[u8] = &[0x01, 0x02, 0xFF, 0x00, 0x02, 0x00];
    let result = ZeroLenOptional::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.numeric, Some(0xFF00_u16));
    assert_eq!(result.label, Some(String::new()));
}
