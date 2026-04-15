// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

// Every fixed-width decoder must fail on empty input.

#[test]
fn be_u8_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_u8(&mut input).is_err());
}

#[test]
fn be_u16_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_u16(&mut input).is_err());
}

#[test]
fn be_u32_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_u32(&mut input).is_err());
}

#[test]
fn be_u64_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_u64(&mut input).is_err());
}

#[test]
fn be_u128_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_u128(&mut input).is_err());
}

#[test]
fn be_i8_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_i8(&mut input).is_err());
}

#[test]
fn be_i16_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_i16(&mut input).is_err());
}

#[test]
fn be_i32_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_i32(&mut input).is_err());
}

#[test]
fn be_i64_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_i64(&mut input).is_err());
}

#[test]
fn be_f32_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_f32(&mut input).is_err());
}

#[test]
fn be_f64_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_f64(&mut input).is_err());
}

#[test]
fn le_u16_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::le_u16(&mut input).is_err());
}

#[test]
fn le_u32_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::le_u32(&mut input).is_err());
}

#[test]
fn le_u64_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::le_u64(&mut input).is_err());
}

#[test]
fn be_u16_lengthed_empty_fails_on_positive_len() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_u16_lengthed(2)(&mut input).is_err());
}

#[test]
fn be_u32_lengthed_empty_fails() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_u32_lengthed(4)(&mut input).is_err());
}

#[test]
fn ber_length_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}

#[test]
fn ber_oid_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::ber::ber_oid::<u64>(&mut input).is_err());
}

#[test]
fn to_string_utf8_empty_zero_len_ok() {
    // len=0 on empty slice is fine
    let mut input: &[u8] = &[];
    let result = tinyklv::dec::binary::to_string_utf8(0)(&mut input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn to_string_utf8_nonzero_len_on_empty_fails() {
    let mut input: &[u8] = &[];
    let result = tinyklv::dec::binary::to_string_utf8(1)(&mut input);
    assert!(result.is_err());
}

#[test]
fn to_string_utf16_le_zero_len_on_empty_ok() {
    // len=0 on empty input is valid — zero bytes consumed, empty string returned
    let mut input: &[u8] = &[];
    let result = tinyklv::dec::binary::to_string_utf16_le(0)(&mut input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn to_string_utf16_le_nonzero_len_on_empty_fails() {
    let mut input: &[u8] = &[];
    let result = tinyklv::dec::binary::to_string_utf16_le(2)(&mut input);
    assert!(result.is_err());
}

#[test]
fn u8_as_usize_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::u8_as_usize(&mut input).is_err());
}

#[test]
fn be_u16_as_usize_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::be_u16_as_usize(&mut input).is_err());
}

#[test]
fn le_u16_as_usize_empty() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::binary::le_u16_as_usize(&mut input).is_err());
}
// --------------------------------------------------
// derive struct: empty input behaviour
// --------------------------------------------------
fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

/// A struct with all required fields — must fail on empty input
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct AllRequired {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    val: u16,
}

#[test]
fn derive_all_required_empty_input_fails() {
    let result = AllRequired::decode(&mut &[][..]);
    assert!(
        result.is_err(),
        "struct with required field must fail on empty input"
    );
}

/// A struct with only optional fields — must succeed (all None) on empty input
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct AllOptional {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    val: Option<u16>,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    other: Option<u32>,
}

#[test]
fn derive_all_optional_empty_input_ok_all_none() {
    let result = AllOptional::decode(&mut &[][..]);
    assert!(
        result.is_ok(),
        "struct with only optional fields must succeed on empty input"
    );
    let s = result.unwrap();
    assert_eq!(s.val, None);
    assert_eq!(s.other, None);
}
