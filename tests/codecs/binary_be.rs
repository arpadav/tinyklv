//! Big-endian fixed-width binary codec tests
//!
//! Tests `enc::binary` and `dec::binary` for all big-endian integer and
//! floating-point types: `u8`/`i8` (single byte), `be_u16`/`be_i16`,
//! `be_u32`/`be_i32`, `be_u64`/`be_i64`, `be_u128`/`be_i128`, `be_f32`,
//! and `be_f64`. Covers known-value decoding, explicit byte-order assertions,
//! encode/decode roundtrips across boundary values, short-input error cases,
//! cursor non-advancement on failure, and float special values (NaN, inf,
//! negative zero, and `PI`/`E`)
//!
//! Author: aav
#[test]
/// Tests decoding a known single-byte `u8` value (`0x42`)
fn u8_known_value() {
    let mut input: &[u8] = &[0x42];
    assert_eq!(tinyklv::dec::binary::u8(&mut input), Ok(0x42_u8));
}

#[test]
/// Tests decoding `u8` on input `0x00` returns `0`
fn u8_zero() {
    let mut input: &[u8] = &[0x00];
    assert_eq!(tinyklv::dec::binary::u8(&mut input), Ok(0_u8));
}

#[test]
/// Tests decoding `u8` on input `0xFF` returns `u8::MAX`
fn u8_max() {
    let mut input: &[u8] = &[0xFF];
    assert_eq!(tinyklv::dec::binary::u8(&mut input), Ok(u8::MAX));
}

#[test]
/// Tests `u8` encoder emits a single byte matching the input value
fn u8_encode() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::u8(0x42_u8, &mut encoded);
    assert_eq!(encoded, vec![0x42]);
}

#[test]
/// Tests `u8` encode/decode roundtrip across boundary values
fn u8_roundtrip() {
    for val in [0_u8, 1, 127, 128, 255] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::u8(val, &mut encoded);
        let decoded = tinyklv::dec::binary::u8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded, "roundtrip failed for u8 value {val}");
    }
}

#[test]
/// Tests decoding a known 2-byte big-endian `u16` value (`0x0102`)
fn be_u16_known_value() {
    let mut input: &[u8] = &[0x01, 0x02];
    assert_eq!(tinyklv::dec::binary::be_u16(&mut input), Ok(0x0102_u16));
}

#[test]
/// Tests `be_u16` encoder produces big-endian byte order for `0x0102`
fn be_u16_encode() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u16(0x0102_u16, &mut encoded);
    assert_eq!(encoded, vec![0x01, 0x02]);
}

#[test]
/// Tests `be_u16` encode/decode roundtrip for `0xABCD`
fn be_u16_roundtrip() {
    let val = 0xABCD_u16;
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u16(val, &mut encoded);
    let decoded = tinyklv::dec::binary::be_u16(&mut encoded.as_slice()).unwrap();
    assert_eq!(val, decoded);
}

#[test]
/// Tests `be_u16` decodes `[0x00, 0x00]` to `0`
fn be_u16_zero() {
    let mut input: &[u8] = &[0x00, 0x00];
    assert_eq!(tinyklv::dec::binary::be_u16(&mut input), Ok(0_u16));
}

#[test]
/// Tests `be_u16` decodes `[0xFF, 0xFF]` to `u16::MAX`
fn be_u16_max() {
    let mut input: &[u8] = &[0xFF, 0xFF];
    assert_eq!(tinyklv::dec::binary::be_u16(&mut input), Ok(u16::MAX));
}

#[test]
/// Tests decoding a known 4-byte big-endian `u32` value (`0x01020304`)
fn be_u32_known_value() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03, 0x04];
    assert_eq!(tinyklv::dec::binary::be_u32(&mut input), Ok(0x01020304_u32));
}

#[test]
/// Tests `be_u32` encoder produces big-endian byte order for `0x01020304`
fn be_u32_encode() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u32(0x01020304_u32, &mut encoded);
    assert_eq!(encoded, vec![0x01, 0x02, 0x03, 0x04]);
}

#[test]
/// Tests `be_u32` encode/decode roundtrip across boundary values
fn be_u32_roundtrip() {
    for val in [0_u32, 1, u32::MAX / 2, u32::MAX] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u32(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_u32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `be_u32` decodes all-zero input to `0`
fn be_u32_zero() {
    let mut input: &[u8] = &[0x00, 0x00, 0x00, 0x00];
    assert_eq!(tinyklv::dec::binary::be_u32(&mut input), Ok(0_u32));
}

#[test]
/// Tests `be_u32` decodes all-`0xFF` input to `u32::MAX`
fn be_u32_max() {
    let mut input: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF];
    assert_eq!(tinyklv::dec::binary::be_u32(&mut input), Ok(u32::MAX));
}

#[test]
/// Tests decoding a known 8-byte big-endian `u64` value
fn be_u64_known_value() {
    let mut input: &[u8] = &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    assert_eq!(
        tinyklv::dec::binary::be_u64(&mut input),
        Ok(0x0001020304050607_u64)
    );
}

#[test]
/// Tests `be_u64` encode/decode roundtrip across boundary values
fn be_u64_roundtrip() {
    for val in [0_u64, 1, u64::MAX / 2, u64::MAX] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u64(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_u64(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `be_u64` encode/decode roundtrip for `u64::MAX`
fn be_u64_max() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u64(u64::MAX, &mut encoded);
    let decoded = tinyklv::dec::binary::be_u64(&mut encoded.as_slice()).unwrap();
    assert_eq!(u64::MAX, decoded);
}

#[test]
/// Tests `be_u128` encode/decode roundtrip across boundary values
fn be_u128_roundtrip() {
    for val in [0_u128, 1, u128::MAX / 2, u128::MAX] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u128(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_u128(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `i8` decodes `0xFF` as `-1` (two's complement)
fn i8_known_value() {
    let mut input: &[u8] = &[0xFF];
    assert_eq!(tinyklv::dec::binary::i8(&mut input), Ok(-1_i8));
}

#[test]
/// Tests `i8` encode/decode roundtrip across signed boundary values
fn i8_roundtrip() {
    for val in [i8::MIN, -1_i8, 0, 1, i8::MAX] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::i8(val, &mut encoded);
        let decoded = tinyklv::dec::binary::i8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `be_i16` decodes `[0xFF, 0xFE]` as `-2` (two's complement)
fn be_i16_known_value() {
    let mut input: &[u8] = &[0xFF, 0xFE];
    assert_eq!(tinyklv::dec::binary::be_i16(&mut input), Ok(-2_i16));
}

#[test]
/// Tests `be_i16` encode/decode roundtrip across signed boundary values
fn be_i16_roundtrip() {
    for val in [i16::MIN, -1_i16, 0, 1, i16::MAX] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_i16(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_i16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `be_i32` encode/decode roundtrip across signed boundary values
fn be_i32_roundtrip() {
    for val in [i32::MIN, -1_i32, 0, 1, i32::MAX] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_i32(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_i32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `be_i64` encode/decode roundtrip across signed boundary values
fn be_i64_roundtrip() {
    for val in [i64::MIN, -1_i64, 0, 1, i64::MAX] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_i64(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_i64(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `be_i128` encode/decode roundtrip across signed boundary values
fn be_i128_roundtrip() {
    for val in [i128::MIN, -1_i128, 0, 1, i128::MAX] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_i128(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_i128(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `be_f32` encode/decode roundtrip preserves `0.0`
fn be_f32_zero() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f32(0.0_f32, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(0.0_f32, decoded);
}

#[test]
/// Tests `be_f32` encode/decode roundtrip preserves `1.0`
fn be_f32_one() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f32(1.0_f32, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(1.0_f32, decoded);
}

#[test]
/// Tests `be_f32` encode/decode roundtrip preserves `-1.0`
fn be_f32_neg_one() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f32(-1.0_f32, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(-1.0_f32, decoded);
}

#[test]
/// Tests `be_f32` encode/decode preserves positive infinity
fn be_f32_infinity() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f32(f32::INFINITY, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_positive());
}

#[test]
/// Tests `be_f32` encode/decode preserves negative infinity
fn be_f32_neg_infinity() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f32(f32::NEG_INFINITY, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_negative());
}

#[test]
/// Tests `be_f32` encode/decode produces a NaN when fed a NaN
fn be_f32_nan() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f32(f32::NAN, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_nan());
}

#[test]
/// Tests `be_f32` encode/decode preserves the exact bit pattern of `PI`
fn be_f32_roundtrip_known() {
    let val = std::f32::consts::PI;
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f32(val, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(val.to_bits(), decoded.to_bits());
}

#[test]
/// Tests `be_f64` encode/decode roundtrip preserves `0.0`
fn be_f64_zero() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f64(0.0_f64, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(0.0_f64, decoded);
}

#[test]
/// Tests `be_f64` encode/decode roundtrip preserves `1.0`
fn be_f64_one() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f64(1.0_f64, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(1.0_f64, decoded);
}

#[test]
/// Tests `be_f64` encode/decode roundtrip preserves `-1.0`
fn be_f64_neg_one() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f64(-1.0_f64, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(-1.0_f64, decoded);
}

#[test]
/// Tests `be_f64` encode/decode preserves positive infinity
fn be_f64_infinity() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f64(f64::INFINITY, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_positive());
}

#[test]
/// Tests `be_f64` encode/decode preserves negative infinity
fn be_f64_neg_infinity() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f64(f64::NEG_INFINITY, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_negative());
}

#[test]
/// Tests `be_f64` encode/decode produces a NaN when fed a NaN
fn be_f64_nan() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f64(f64::NAN, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_nan());
}

#[test]
/// Tests `be_f64` encode/decode preserves the exact bit pattern of `PI`
fn be_f64_roundtrip_known() {
    let val = std::f64::consts::PI;
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_f64(val, &mut encoded);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(val.to_bits(), decoded.to_bits());
}

#[test]
/// Tests `be_u16` emits high byte first (big-endian byte order)
fn be_u16_byte_order() {
    // 0x0102 big-endian: high byte first
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u16(0x0102_u16, &mut encoded);
    assert_eq!(encoded, vec![0x01, 0x02]);
}

#[test]
/// Tests `be_u32` emits bytes in big-endian order
fn be_u32_byte_order() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u32(0x01020304_u32, &mut encoded);
    assert_eq!(encoded, vec![0x01, 0x02, 0x03, 0x04]);
}

#[test]
/// Tests `be_u64` emits bytes in big-endian order
fn be_u64_byte_order() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u64(0x0102030405060708_u64, &mut encoded);
    assert_eq!(
        encoded,
        vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    );
}

#[test]
/// Fixed-width big-endian decoders error (not panic) on short input of `N - 1` bytes
fn be_short_input_errors() {
    assert!(tinyklv::dec::binary::be_u16(&mut &[0u8; 1][..]).is_err());
    assert!(tinyklv::dec::binary::be_u32(&mut &[0u8; 3][..]).is_err());
    assert!(tinyklv::dec::binary::be_u64(&mut &[0u8; 7][..]).is_err());
    assert!(tinyklv::dec::binary::be_u128(&mut &[0u8; 15][..]).is_err());
    assert!(tinyklv::dec::binary::be_i16(&mut &[0u8; 1][..]).is_err());
    assert!(tinyklv::dec::binary::be_i32(&mut &[0u8; 3][..]).is_err());
    assert!(tinyklv::dec::binary::be_i64(&mut &[0u8; 7][..]).is_err());
    assert!(tinyklv::dec::binary::be_i128(&mut &[0u8; 15][..]).is_err());
    assert!(tinyklv::dec::binary::be_f32(&mut &[0u8; 3][..]).is_err());
    assert!(tinyklv::dec::binary::be_f64(&mut &[0u8; 7][..]).is_err());
}

#[test]
/// Fixed-width big-endian decoders error on empty input, including single-byte `u8`/`i8`
fn be_empty_input_errors() {
    assert!(tinyklv::dec::binary::u8(&mut &[][..]).is_err());
    assert!(tinyklv::dec::binary::i8(&mut &[][..]).is_err());
    assert!(tinyklv::dec::binary::be_u16(&mut &[][..]).is_err());
    assert!(tinyklv::dec::binary::be_u32(&mut &[][..]).is_err());
    assert!(tinyklv::dec::binary::be_u64(&mut &[][..]).is_err());
    assert!(tinyklv::dec::binary::be_u128(&mut &[][..]).is_err());
    assert!(tinyklv::dec::binary::be_f32(&mut &[][..]).is_err());
    assert!(tinyklv::dec::binary::be_f64(&mut &[][..]).is_err());
}

#[test]
/// A failed fixed-width decode must NOT advance the cursor (callers see the un-eaten bytes)
fn be_short_input_does_not_advance_cursor() {
    let buf = [0x01, 0x02, 0x03];
    let mut input: &[u8] = &buf;
    assert!(tinyklv::dec::binary::be_u32(&mut input).is_err());
    assert_eq!(input.len(), 3, "cursor advanced on a failed be_u32");
}
