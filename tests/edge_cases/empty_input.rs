//! Empty-input error tests for all codecs
//!
//! Verifies that every decoder returns `Err` when presented with a zero-byte
//! input slice. Covers all binary types (`u8`/`i8`, `be_u16` through `be_u128`,
//! `le_u16` through `le_u64`, `be_f32`/`be_f64`, `be_u16_lengthed`,
//! `be_u32_lengthed`, `u8_as_usize`, `be_u16_as_usize`, `le_u16_as_usize`),
//! BER (`ber_length`, `ber_oid`), and string (`to_string_utf8` with non-zero
//! len, `to_string_utf16_le` with non-zero len). Also tests derive-macro
//! behavior: all-required struct errors, all-optional struct succeeds with
//! every field `None`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[test]
/// Tests that `u8` errors on empty input
fn u8_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::u8(&mut input).is_err());
}

#[test]
/// Tests that `be_u16` errors on empty input
fn be_u16_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_u16(&mut input).is_err());
}

#[test]
/// Tests that `be_u32` errors on empty input
fn be_u32_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_u32(&mut input).is_err());
}

#[test]
/// Tests that `be_u64` errors on empty input
fn be_u64_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_u64(&mut input).is_err());
}

#[test]
/// Tests that `be_u128` errors on empty input
fn be_u128_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_u128(&mut input).is_err());
}

#[test]
/// Tests that `i8` errors on empty input
fn i8_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::i8(&mut input).is_err());
}

#[test]
/// Tests that `be_i16` errors on empty input
fn be_i16_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_i16(&mut input).is_err());
}

#[test]
/// Tests that `be_i32` errors on empty input
fn be_i32_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_i32(&mut input).is_err());
}

#[test]
/// Tests that `be_i64` errors on empty input
fn be_i64_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_i64(&mut input).is_err());
}

#[test]
/// Tests that `be_f32` errors on empty input
fn be_f32_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_f32(&mut input).is_err());
}

#[test]
/// Tests that `be_f64` errors on empty input
fn be_f64_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_f64(&mut input).is_err());
}

#[test]
/// Tests that `le_u16` errors on empty input
fn le_u16_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::le_u16(&mut input).is_err());
}

#[test]
/// Tests that `le_u32` errors on empty input
fn le_u32_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::le_u32(&mut input).is_err());
}

#[test]
/// Tests that `le_u64` errors on empty input
fn le_u64_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::le_u64(&mut input).is_err());
}

#[test]
/// Tests that `be_u16_lengthed(2)` errors when asked for 2 bytes from empty input
fn be_u16_lengthed_empty_fails_on_positive_len() {
    let mut input: &[u8] = &[];
    assert!(decb::be_u16_lengthed(2)(&mut input).is_err());
}

#[test]
/// Tests that `be_u32_lengthed(4)` errors when asked for 4 bytes from empty input
fn be_u32_lengthed_empty_fails() {
    let mut input: &[u8] = &[];
    assert!(decb::be_u32_lengthed(4)(&mut input).is_err());
}

#[test]
/// Tests that `ber_length` errors on empty input (no first byte to interpret)
fn ber_length_empty() {
    let mut input: &[u8] = &[];
    assert!(decber::ber_length(&mut input).is_err());
}

#[test]
/// Tests that `ber_oid::<u64>` errors on empty input
fn ber_oid_empty() {
    let mut input: &[u8] = &[];
    assert!(decber::ber_oid::<u64>(&mut input).is_err());
}

#[test]
/// Tests that `to_string_utf8(0)` on empty input succeeds and returns an empty string
fn to_string_utf8_empty_zero_len_ok() {
    // len=0 on empty slice is fine
    let mut input: &[u8] = &[];
    let result = decs::to_string_utf8(0)(&mut input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
/// Tests that `to_string_utf8(1)` errors on empty input because one byte is required
fn to_string_utf8_nonzero_len_on_empty_fails() {
    let mut input: &[u8] = &[];
    let result = decs::to_string_utf8(1)(&mut input);
    assert!(result.is_err());
}

#[test]
/// Tests that `to_string_utf16_le(0)` on empty input succeeds and returns an empty string
fn to_string_utf16_le_zero_len_on_empty_ok() {
    // len=0 on empty input is valid - zero bytes consumed, empty string returned
    let mut input: &[u8] = &[];
    let result = decs::to_string_utf16_le(0)(&mut input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
/// Tests that `to_string_utf16_le(2)` errors on empty input because two bytes are required
fn to_string_utf16_le_nonzero_len_on_empty_fails() {
    let mut input: &[u8] = &[];
    let result = decs::to_string_utf16_le(2)(&mut input);
    assert!(result.is_err());
}

#[test]
/// Tests that `u8_as_usize` errors on empty input
fn u8_as_usize_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::u8_as_usize(&mut input).is_err());
}

#[test]
/// Tests that `be_u16_as_usize` errors on empty input
fn be_u16_as_usize_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::be_u16_as_usize(&mut input).is_err());
}

#[test]
/// Tests that `le_u16_as_usize` errors on empty input
fn le_u16_as_usize_empty() {
    let mut input: &[u8] = &[];
    assert!(decb::le_u16_as_usize(&mut input).is_err());
}

#[derive(Klv, Debug, PartialEq)]
/// A struct with all required fields - must fail on empty input
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct AllRequired {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    val: u16,
}

#[test]
/// Tests that deriving `decode_value` on a struct with a required field errors when the input stream is empty
fn derive_all_required_empty_input_fails() {
    let result = AllRequired::decode_value(&mut &[][..]);
    assert!(
        result.is_err(),
        "struct with required field must fail on empty input"
    );
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// A struct with only optional fields - must succeed (all None) on empty input
struct AllOptional {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    val: Option<u16>,
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    other: Option<u32>,
}

#[test]
/// Tests that an all-optional struct decodes an empty stream successfully with every field set to `None`
fn derive_all_optional_empty_input_ok_all_none() {
    let result = AllOptional::decode_value(&mut &[][..]);
    assert!(
        result.is_ok(),
        "struct with only optional fields must succeed on empty input"
    );
    let s = result.unwrap();
    assert_eq!(s.val, None);
    assert_eq!(s.other, None);
}
