// --------------------------------------------------
// u8
// --------------------------------------------------

#[test]
fn be_u8_known_value() {
    let mut input: &[u8] = &[0x42];
    assert_eq!(tinyklv::dec::binary::be_u8(&mut input), Ok(0x42_u8));
}

#[test]
fn be_u8_zero() {
    let mut input: &[u8] = &[0x00];
    assert_eq!(tinyklv::dec::binary::be_u8(&mut input), Ok(0_u8));
}

#[test]
fn be_u8_max() {
    let mut input: &[u8] = &[0xFF];
    assert_eq!(tinyklv::dec::binary::be_u8(&mut input), Ok(u8::MAX));
}

#[test]
fn be_u8_encode() {
    assert_eq!(tinyklv::enc::binary::u8(0x42_u8), vec![0x42]);
}

#[test]
fn be_u8_roundtrip() {
    for val in [0_u8, 1, 127, 128, 255] {
        let encoded = tinyklv::enc::binary::u8(val);
        let decoded = tinyklv::dec::binary::be_u8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded, "roundtrip failed for u8 value {val}");
    }
}

// --------------------------------------------------
// u16
// --------------------------------------------------

#[test]
fn be_u16_known_value() {
    let mut input: &[u8] = &[0x01, 0x02];
    assert_eq!(tinyklv::dec::binary::be_u16(&mut input), Ok(0x0102_u16));
}

#[test]
fn be_u16_encode() {
    assert_eq!(tinyklv::enc::binary::be_u16(0x0102_u16), vec![0x01, 0x02]);
}

#[test]
fn be_u16_roundtrip() {
    let val = 0xABCD_u16;
    let encoded = tinyklv::enc::binary::be_u16(val);
    let decoded = tinyklv::dec::binary::be_u16(&mut encoded.as_slice()).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn be_u16_zero() {
    let mut input: &[u8] = &[0x00, 0x00];
    assert_eq!(tinyklv::dec::binary::be_u16(&mut input), Ok(0_u16));
}

#[test]
fn be_u16_max() {
    let mut input: &[u8] = &[0xFF, 0xFF];
    assert_eq!(tinyklv::dec::binary::be_u16(&mut input), Ok(u16::MAX));
}

// --------------------------------------------------
// u32
// --------------------------------------------------

#[test]
fn be_u32_known_value() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03, 0x04];
    assert_eq!(tinyklv::dec::binary::be_u32(&mut input), Ok(0x01020304_u32));
}

#[test]
fn be_u32_encode() {
    assert_eq!(
        tinyklv::enc::binary::be_u32(0x01020304_u32),
        vec![0x01, 0x02, 0x03, 0x04]
    );
}

#[test]
fn be_u32_roundtrip() {
    for val in [0_u32, 1, u32::MAX / 2, u32::MAX] {
        let encoded = tinyklv::enc::binary::be_u32(val);
        let decoded = tinyklv::dec::binary::be_u32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
fn be_u32_zero() {
    let mut input: &[u8] = &[0x00, 0x00, 0x00, 0x00];
    assert_eq!(tinyklv::dec::binary::be_u32(&mut input), Ok(0_u32));
}

#[test]
fn be_u32_max() {
    let mut input: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF];
    assert_eq!(tinyklv::dec::binary::be_u32(&mut input), Ok(u32::MAX));
}

// --------------------------------------------------
// u64
// --------------------------------------------------

#[test]
fn be_u64_known_value() {
    let mut input: &[u8] = &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    assert_eq!(
        tinyklv::dec::binary::be_u64(&mut input),
        Ok(0x0001020304050607_u64)
    );
}

#[test]
fn be_u64_roundtrip() {
    for val in [0_u64, 1, u64::MAX / 2, u64::MAX] {
        let encoded = tinyklv::enc::binary::be_u64(val);
        let decoded = tinyklv::dec::binary::be_u64(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
fn be_u64_max() {
    let encoded = tinyklv::enc::binary::be_u64(u64::MAX);
    let decoded = tinyklv::dec::binary::be_u64(&mut encoded.as_slice()).unwrap();
    assert_eq!(u64::MAX, decoded);
}

// --------------------------------------------------
// u128
// --------------------------------------------------

#[test]
fn be_u128_roundtrip() {
    for val in [0_u128, 1, u128::MAX / 2, u128::MAX] {
        let encoded = tinyklv::enc::binary::be_u128(val);
        let decoded = tinyklv::dec::binary::be_u128(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i8
// --------------------------------------------------

#[test]
fn be_i8_known_value() {
    let mut input: &[u8] = &[0xFF];
    assert_eq!(tinyklv::dec::binary::be_i8(&mut input), Ok(-1_i8));
}

#[test]
fn be_i8_roundtrip() {
    for val in [i8::MIN, -1_i8, 0, 1, i8::MAX] {
        let encoded = tinyklv::enc::binary::i8(val);
        let decoded = tinyklv::dec::binary::be_i8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i16
// --------------------------------------------------

#[test]
fn be_i16_known_value() {
    let mut input: &[u8] = &[0xFF, 0xFE];
    assert_eq!(tinyklv::dec::binary::be_i16(&mut input), Ok(-2_i16));
}

#[test]
fn be_i16_roundtrip() {
    for val in [i16::MIN, -1_i16, 0, 1, i16::MAX] {
        let encoded = tinyklv::enc::binary::be_i16(val);
        let decoded = tinyklv::dec::binary::be_i16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i32
// --------------------------------------------------

#[test]
fn be_i32_roundtrip() {
    for val in [i32::MIN, -1_i32, 0, 1, i32::MAX] {
        let encoded = tinyklv::enc::binary::be_i32(val);
        let decoded = tinyklv::dec::binary::be_i32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i64
// --------------------------------------------------

#[test]
fn be_i64_roundtrip() {
    for val in [i64::MIN, -1_i64, 0, 1, i64::MAX] {
        let encoded = tinyklv::enc::binary::be_i64(val);
        let decoded = tinyklv::dec::binary::be_i64(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i128
// --------------------------------------------------

#[test]
fn be_i128_roundtrip() {
    for val in [i128::MIN, -1_i128, 0, 1, i128::MAX] {
        let encoded = tinyklv::enc::binary::be_i128(val);
        let decoded = tinyklv::dec::binary::be_i128(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// f32
// --------------------------------------------------

#[test]
fn be_f32_zero() {
    let encoded = tinyklv::enc::binary::be_f32(0.0_f32);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(0.0_f32, decoded);
}

#[test]
fn be_f32_one() {
    let encoded = tinyklv::enc::binary::be_f32(1.0_f32);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(1.0_f32, decoded);
}

#[test]
fn be_f32_neg_one() {
    let encoded = tinyklv::enc::binary::be_f32(-1.0_f32);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(-1.0_f32, decoded);
}

#[test]
fn be_f32_infinity() {
    let encoded = tinyklv::enc::binary::be_f32(f32::INFINITY);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_positive());
}

#[test]
fn be_f32_neg_infinity() {
    let encoded = tinyklv::enc::binary::be_f32(f32::NEG_INFINITY);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_negative());
}

#[test]
fn be_f32_nan() {
    let encoded = tinyklv::enc::binary::be_f32(f32::NAN);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_nan());
}

#[test]
fn be_f32_roundtrip_known() {
    let val = std::f32::consts::PI;
    let encoded = tinyklv::enc::binary::be_f32(val);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(val.to_bits(), decoded.to_bits());
}

// --------------------------------------------------
// f64
// --------------------------------------------------

#[test]
fn be_f64_zero() {
    let encoded = tinyklv::enc::binary::be_f64(0.0_f64);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(0.0_f64, decoded);
}

#[test]
fn be_f64_one() {
    let encoded = tinyklv::enc::binary::be_f64(1.0_f64);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(1.0_f64, decoded);
}

#[test]
fn be_f64_neg_one() {
    let encoded = tinyklv::enc::binary::be_f64(-1.0_f64);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(-1.0_f64, decoded);
}

#[test]
fn be_f64_infinity() {
    let encoded = tinyklv::enc::binary::be_f64(f64::INFINITY);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_positive());
}

#[test]
fn be_f64_neg_infinity() {
    let encoded = tinyklv::enc::binary::be_f64(f64::NEG_INFINITY);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_negative());
}

#[test]
fn be_f64_nan() {
    let encoded = tinyklv::enc::binary::be_f64(f64::NAN);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_nan());
}

#[test]
fn be_f64_roundtrip_known() {
    let val = std::f64::consts::PI;
    let encoded = tinyklv::enc::binary::be_f64(val);
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(val.to_bits(), decoded.to_bits());
}

// --------------------------------------------------
// encode byte layout verification
// --------------------------------------------------

#[test]
fn be_u16_byte_order() {
    // 0x0102 big-endian: high byte first
    let encoded = tinyklv::enc::binary::be_u16(0x0102_u16);
    assert_eq!(encoded, vec![0x01, 0x02]);
}

#[test]
fn be_u32_byte_order() {
    let encoded = tinyklv::enc::binary::be_u32(0x01020304_u32);
    assert_eq!(encoded, vec![0x01, 0x02, 0x03, 0x04]);
}

#[test]
fn be_u64_byte_order() {
    let encoded = tinyklv::enc::binary::be_u64(0x0102030405060708_u64);
    assert_eq!(
        encoded,
        vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    );
}
