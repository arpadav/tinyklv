#[test]
/// Tests decoding a known single-byte `u8` value (`0x42`).
fn u8_known_value() {
    let mut input: &[u8] = &[0x42];
    assert_eq!(tinyklv::dec::binary::u8(&mut input), Ok(0x42_u8));
}

#[test]
/// Tests `u8` encode/`u8` decode roundtrip across boundary values.
fn u8_roundtrip() {
    for val in [0_u8, 1, 127, 128, 255] {
        let encoded = tinyklv::enc::binary::u8(val);
        let decoded = tinyklv::dec::binary::u8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `le_u16` decodes bytes in little-endian order (low byte first).
fn le_u16_known_value() {
    // 0x0102 little-endian: low byte first
    let mut input: &[u8] = &[0x02, 0x01];
    assert_eq!(tinyklv::dec::binary::le_u16(&mut input), Ok(0x0102_u16));
}

#[test]
/// Tests `le_u16` encoder emits low byte first for `0x0102`.
fn le_u16_encode() {
    // 0x0102 in LE: low byte 0x02 first
    assert_eq!(tinyklv::enc::binary::le_u16(0x0102_u16), vec![0x02, 0x01]);
}

#[test]
/// Tests `le_u16` encode/decode roundtrip for `0xABCD`.
fn le_u16_roundtrip() {
    let val = 0xABCD_u16;
    let encoded = tinyklv::enc::binary::le_u16(val);
    let decoded = tinyklv::dec::binary::le_u16(&mut encoded.as_slice()).unwrap();
    assert_eq!(val, decoded);
}

#[test]
/// Tests `le_u16` decodes `[0x00, 0x00]` to `0`.
fn le_u16_zero() {
    let mut input: &[u8] = &[0x00, 0x00];
    assert_eq!(tinyklv::dec::binary::le_u16(&mut input), Ok(0_u16));
}

#[test]
/// Tests `le_u16` decodes `[0xFF, 0xFF]` to `u16::MAX`.
fn le_u16_max() {
    let mut input: &[u8] = &[0xFF, 0xFF];
    assert_eq!(tinyklv::dec::binary::le_u16(&mut input), Ok(u16::MAX));
}

#[test]
/// Tests `le_u16` encoder emits little-endian byte order (low byte first).
fn le_u16_byte_order() {
    // 0x0102 LE: low byte [0x02] comes first
    let encoded = tinyklv::enc::binary::le_u16(0x0102_u16);
    assert_eq!(encoded, vec![0x02, 0x01]);
}

#[test]
/// Tests `le_u32` decodes `to_le_bytes` output back to the original value.
fn le_u32_known_value() {
    let bytes = 0x01020304_u32.to_le_bytes();
    let mut input: &[u8] = &bytes;
    assert_eq!(tinyklv::dec::binary::le_u32(&mut input), Ok(0x01020304_u32));
}

#[test]
/// Tests `le_u32` encode/decode roundtrip across boundary values.
fn le_u32_roundtrip() {
    for val in [0_u32, 1, u32::MAX / 2, u32::MAX] {
        let encoded = tinyklv::enc::binary::le_u32(val);
        let decoded = tinyklv::dec::binary::le_u32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `le_u32` emits bytes in little-endian order.
fn le_u32_byte_order() {
    let encoded = tinyklv::enc::binary::le_u32(0x01020304_u32);
    assert_eq!(encoded, vec![0x04, 0x03, 0x02, 0x01]);
}

#[test]
/// Tests `le_u64` encode/decode roundtrip across boundary values.
fn le_u64_roundtrip() {
    for val in [0_u64, 1, u64::MAX / 2, u64::MAX] {
        let encoded = tinyklv::enc::binary::le_u64(val);
        let decoded = tinyklv::dec::binary::le_u64(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `le_u128` encode/decode roundtrip across boundary values.
fn le_u128_roundtrip() {
    for val in [0_u128, 1, u128::MAX / 2, u128::MAX] {
        let encoded = tinyklv::enc::binary::le_u128(val);
        let decoded = tinyklv::dec::binary::le_u128(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `i8` encode/decode roundtrip across signed boundary values.
fn i8_roundtrip() {
    for val in [i8::MIN, -1_i8, 0, 1, i8::MAX] {
        let encoded = tinyklv::enc::binary::i8(val);
        let decoded = tinyklv::dec::binary::i8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `le_i16` decodes `[0xFE, 0xFF]` as `-2` (two's complement little-endian).
fn le_i16_known_value() {
    // -2 as i16 in LE: 0xFFFE -> [0xFE, 0xFF]
    let mut input: &[u8] = &[0xFE, 0xFF];
    assert_eq!(tinyklv::dec::binary::le_i16(&mut input), Ok(-2_i16));
}

#[test]
/// Tests `le_i16` encode/decode roundtrip across signed boundary values.
fn le_i16_roundtrip() {
    for val in [i16::MIN, -1_i16, 0, 1, i16::MAX] {
        let encoded = tinyklv::enc::binary::le_i16(val);
        let decoded = tinyklv::dec::binary::le_i16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `le_i32` encode/decode roundtrip across signed boundary values.
fn le_i32_roundtrip() {
    for val in [i32::MIN, -1_i32, 0, 1, i32::MAX] {
        let encoded = tinyklv::enc::binary::le_i32(val);
        let decoded = tinyklv::dec::binary::le_i32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `le_i64` encode/decode roundtrip across signed boundary values.
fn le_i64_roundtrip() {
    for val in [i64::MIN, -1_i64, 0, 1, i64::MAX] {
        let encoded = tinyklv::enc::binary::le_i64(val);
        let decoded = tinyklv::dec::binary::le_i64(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `le_i128` encode/decode roundtrip across signed boundary values.
fn le_i128_roundtrip() {
    for val in [i128::MIN, -1_i128, 0, 1, i128::MAX] {
        let encoded = tinyklv::enc::binary::le_i128(val);
        let decoded = tinyklv::dec::binary::le_i128(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
/// Tests `le_f32` encode/decode roundtrip preserves `0.0`.
fn le_f32_zero() {
    let encoded = tinyklv::enc::binary::le_f32(0.0_f32);
    let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(0.0_f32, decoded);
}

#[test]
/// Tests `le_f32` encode/decode preserves the exact bit pattern of `E`.
fn le_f32_roundtrip_known() {
    let val = std::f32::consts::E;
    let encoded = tinyklv::enc::binary::le_f32(val);
    let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(val.to_bits(), decoded.to_bits());
}

#[test]
/// Tests `le_f32` encode/decode preserves positive infinity.
fn le_f32_infinity() {
    let encoded = tinyklv::enc::binary::le_f32(f32::INFINITY);
    let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_positive());
}

#[test]
/// Tests `le_f32` encode/decode produces a NaN when fed a NaN.
fn le_f32_nan() {
    let encoded = tinyklv::enc::binary::le_f32(f32::NAN);
    let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_nan());
}

#[test]
/// Tests `le_f64` encode/decode roundtrip preserves `0.0`.
fn le_f64_zero() {
    let encoded = tinyklv::enc::binary::le_f64(0.0_f64);
    let decoded = tinyklv::dec::binary::le_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(0.0_f64, decoded);
}

#[test]
/// Tests `le_f64` encode/decode preserves the exact bit pattern of `E`.
fn le_f64_roundtrip_known() {
    let val = std::f64::consts::E;
    let encoded = tinyklv::enc::binary::le_f64(val);
    let decoded = tinyklv::dec::binary::le_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(val.to_bits(), decoded.to_bits());
}

#[test]
/// Tests `le_f64` encode/decode produces a NaN when fed a NaN.
fn le_f64_nan() {
    let encoded = tinyklv::enc::binary::le_f64(f64::NAN);
    let decoded = tinyklv::dec::binary::le_f64(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_nan());
}

#[test]
/// Tests that BE and LE `u16` encoders produce byte-reversed outputs for the same value.
fn be_vs_le_u16_differ() {
    let val = 0x0102_u16;
    let be = tinyklv::enc::binary::be_u16(val);
    let le = tinyklv::enc::binary::le_u16(val);
    assert_eq!(be, vec![0x01, 0x02]);
    assert_eq!(le, vec![0x02, 0x01]);
}

#[test]
/// Tests that BE and LE `u32` encoders produce byte-reversed outputs for the same value.
fn be_vs_le_u32_differ() {
    let val = 0x01020304_u32;
    let be = tinyklv::enc::binary::be_u32(val);
    let le = tinyklv::enc::binary::le_u32(val);
    assert_eq!(be, vec![0x01, 0x02, 0x03, 0x04]);
    assert_eq!(le, vec![0x04, 0x03, 0x02, 0x01]);
}

#[test]
/// Fixed-width little-endian decoders error (not panic) on short input of `N - 1` bytes.
fn le_short_input_errors() {
    assert!(tinyklv::dec::binary::le_u16(&mut &[0u8; 1][..]).is_err());
    assert!(tinyklv::dec::binary::le_u32(&mut &[0u8; 3][..]).is_err());
    assert!(tinyklv::dec::binary::le_u64(&mut &[0u8; 7][..]).is_err());
    assert!(tinyklv::dec::binary::le_u128(&mut &[0u8; 15][..]).is_err());
    assert!(tinyklv::dec::binary::le_f32(&mut &[0u8; 3][..]).is_err());
    assert!(tinyklv::dec::binary::le_f64(&mut &[0u8; 7][..]).is_err());
}

#[test]
/// A failed little-endian fixed-width decode must NOT advance the cursor.
fn le_short_input_does_not_advance_cursor() {
    let buf = [0x01, 0x02, 0x03];
    let mut input: &[u8] = &buf;
    assert!(tinyklv::dec::binary::le_u32(&mut input).is_err());
    assert_eq!(input.len(), 3, "cursor advanced on a failed le_u32");
}
