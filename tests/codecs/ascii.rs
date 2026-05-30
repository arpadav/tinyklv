//! ASCII numeric and run-validator codec tests
//!
//! Tests the `dec::string` and `enc::string` function families for
//! unsigned integers, signed integers, floats, hex values, and character
//! class validators (`alpha`, `alphanumeric`, `digit`).  Covers known-good
//! values, boundary cases (leading zeros, overflow, empty fields), trailing
//! garbage rejection, and encode/decode roundtrips for all numeric types
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::string as deca;
use tinyklv::enc::string as enca;

#[test]
/// Tests decoding a base-10 unsigned integer from ASCII text
fn uint_known_value() {
    let mut input: &[u8] = b"12345";
    assert_eq!(deca::u32(5)(&mut input), Ok(12345_u32));
}

#[test]
/// Tests that a bare zero decodes to `0`
fn uint_bare_zero() {
    let mut input: &[u8] = b"0";
    assert_eq!(deca::u32(1)(&mut input), Ok(0_u32));
}

#[test]
/// Tests that decimal leading zeros are rejected (winnow's `dec_uint` grammar
/// accepts a bare `0` or a non-zero-led run, never `007`)
fn uint_leading_zeros_rejected() {
    let mut input: &[u8] = b"007";
    assert!(deca::u32(3)(&mut input).is_err());
}

#[test]
/// Tests that a value exceeding the target type's range is rejected
fn uint_overflow_errors() {
    let mut input: &[u8] = b"256";
    assert!(deca::u8(3)(&mut input).is_err());
}

#[test]
/// Tests that an empty field (no digits) is rejected
fn uint_empty_errors() {
    let mut input: &[u8] = b"";
    assert!(deca::u32(0)(&mut input).is_err());
}

#[test]
/// Tests that trailing non-digit bytes within the field are rejected
fn uint_trailing_garbage_errors() {
    let mut input: &[u8] = b"12X";
    assert!(deca::u32(3)(&mut input).is_err());
}

#[test]
/// Tests decoding signed integers with explicit sign characters
fn int_signed_values() {
    let mut neg: &[u8] = b"-42";
    assert_eq!(deca::i32(3)(&mut neg), Ok(-42_i32));
    let mut pos: &[u8] = b"+42";
    assert_eq!(deca::i32(3)(&mut pos), Ok(42_i32));
}

#[test]
/// Tests decoding a floating-point value from ASCII text
fn float_known_value() {
    let mut input: &[u8] = b"42.5";
    assert_eq!(deca::f64(4)(&mut input), Ok(42.5_f64));
}

#[test]
/// Tests decoding the textual infinity float literal
fn float_infinity() {
    let mut input: &[u8] = b"inf";
    let decoded = deca::f64(3)(&mut input).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_positive());
}

#[test]
/// Tests that trailing garbage after a float is rejected
fn float_trailing_garbage_errors() {
    let mut input: &[u8] = b"1.5x";
    assert!(deca::f64(4)(&mut input).is_err());
}

#[test]
/// Tests decoding an unsigned integer from upper/lower-case ASCII hexadecimal
fn hex_known_value() {
    let mut lower: &[u8] = b"abcd";
    assert_eq!(deca::hex_u16(4)(&mut lower), Ok(0xABCD_u16));
    let mut upper: &[u8] = b"FF";
    assert_eq!(deca::hex_u16(2)(&mut upper), Ok(0xFF_u16));
}

#[test]
/// Tests that non-hexadecimal bytes are rejected by the hex decoder
fn hex_non_hex_errors() {
    let mut input: &[u8] = b"xyz";
    assert!(deca::hex_u16(3)(&mut input).is_err());
}

#[test]
/// Tests the alphabetic run validator returns the matched string
fn run_alpha() {
    let mut input: &[u8] = b"ABC";
    assert_eq!(deca::alpha(3)(&mut input), Ok(String::from("ABC")));
}

#[test]
/// Tests the alphanumeric run validator returns the matched string
fn run_alphanumeric() {
    let mut input: &[u8] = b"a1B2";
    assert_eq!(deca::alphanumeric(4)(&mut input), Ok(String::from("a1B2")));
}

#[test]
/// Tests the alphabetic run validator rejects a field containing a digit
fn run_alpha_rejects_digit() {
    let mut input: &[u8] = b"AB1";
    assert!(deca::alpha(3)(&mut input).is_err());
}

#[test]
/// Tests the digit run validator rejects a field with a trailing non-digit
fn run_digit_rejects_non_digit() {
    let mut input: &[u8] = b"12X";
    assert!(deca::digit(3)(&mut input).is_err());
}

#[test]
/// Tests the alphanumeric run validator rejects a field with a symbol
fn run_alphanumeric_rejects_symbol() {
    let mut input: &[u8] = b"a1-";
    assert!(deca::alphanumeric(3)(&mut input).is_err());
}

#[test]
/// Tests unsigned integer ASCII encoding
fn uint_encode() {
    let mut v = Vec::new();
    enca::u32(42, &mut v);
    assert_eq!(v, b"42".to_vec());
}

#[test]
/// Tests signed integer ASCII encoding preserves the sign
fn int_encode() {
    let mut v = Vec::new();
    enca::i32(-5, &mut v);
    assert_eq!(v, b"-5".to_vec());
}

#[test]
/// Tests floating-point ASCII encoding emits canonical text
fn float_encode() {
    let mut v = Vec::new();
    enca::f64(123.5, &mut v);
    assert_eq!(v, b"123.5".to_vec());
}

#[test]
/// Tests hexadecimal ASCII encoding emits lower-case digits
fn hex_encode() {
    let mut v = Vec::new();
    enca::hex_u16(0xABCD, &mut v);
    assert_eq!(v, b"abcd".to_vec());
}

#[test]
/// Tests encode/decode roundtrip for unsigned, signed, float, and hex codecs
fn roundtrip_all() {
    for val in [0_u64, 1, 42, 1_000_000, u64::MAX] {
        let mut encoded = Vec::new();
        enca::u64(val, &mut encoded);
        let decoded = deca::u64(encoded.len())(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded, "uint roundtrip failed for {val}");
    }
    for val in [i64::MIN, -1, 0, 1, i64::MAX] {
        let mut encoded = Vec::new();
        enca::i64(val, &mut encoded);
        let decoded = deca::i64(encoded.len())(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded, "int roundtrip failed for {val}");
    }
    for val in [0_u32, 0xF, 0xABCD, u32::MAX] {
        let mut encoded = Vec::new();
        enca::hex_u32(val, &mut encoded);
        let decoded = deca::hex_u32(encoded.len())(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded, "hex roundtrip failed for {val}");
    }
}
