use tinyklv::prelude::*;

// --------------------------------------------------
// u8 - endianness is irrelevant for single-byte
// types, but both decoders exist
// --------------------------------------------------

#[test]
fn le_u8_known_value() {
    let mut input: &[u8] = &[0x42];
    assert_eq!(tinyklv::dec::binary::le_u8(&mut input), Ok(0x42_u8));
}

#[test]
fn le_u8_roundtrip() {
    for val in [0_u8, 1, 127, 128, 255] {
        let encoded = tinyklv::enc::binary::u8(val);
        let decoded = tinyklv::dec::binary::le_u8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// u16
// --------------------------------------------------

#[test]
fn le_u16_known_value() {
    // 0x0102 little-endian: low byte first
    let mut input: &[u8] = &[0x02, 0x01];
    assert_eq!(tinyklv::dec::binary::le_u16(&mut input), Ok(0x0102_u16));
}

#[test]
fn le_u16_encode() {
    // 0x0102 in LE: low byte 0x02 first
    assert_eq!(tinyklv::enc::binary::le_u16(0x0102_u16), vec![0x02, 0x01]);
}

#[test]
fn le_u16_roundtrip() {
    let val = 0xABCD_u16;
    let encoded = tinyklv::enc::binary::le_u16(val);
    let decoded = tinyklv::dec::binary::le_u16(&mut encoded.as_slice()).unwrap();
    assert_eq!(val, decoded);
}

#[test]
fn le_u16_zero() {
    let mut input: &[u8] = &[0x00, 0x00];
    assert_eq!(tinyklv::dec::binary::le_u16(&mut input), Ok(0_u16));
}

#[test]
fn le_u16_max() {
    let mut input: &[u8] = &[0xFF, 0xFF];
    assert_eq!(tinyklv::dec::binary::le_u16(&mut input), Ok(u16::MAX));
}

#[test]
fn le_u16_byte_order() {
    // 0x0102 LE: low byte [0x02] comes first
    let encoded = tinyklv::enc::binary::le_u16(0x0102_u16);
    assert_eq!(encoded, vec![0x02, 0x01]);
}

// --------------------------------------------------
// u32
// --------------------------------------------------

#[test]
fn le_u32_known_value() {
    let bytes = 0x01020304_u32.to_le_bytes();
    let mut input: &[u8] = &bytes;
    assert_eq!(tinyklv::dec::binary::le_u32(&mut input), Ok(0x01020304_u32));
}

#[test]
fn le_u32_roundtrip() {
    for val in [0_u32, 1, u32::MAX / 2, u32::MAX] {
        let encoded = tinyklv::enc::binary::le_u32(val);
        let decoded = tinyklv::dec::binary::le_u32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

#[test]
fn le_u32_byte_order() {
    let encoded = tinyklv::enc::binary::le_u32(0x01020304_u32);
    assert_eq!(encoded, vec![0x04, 0x03, 0x02, 0x01]);
}

// --------------------------------------------------
// u64
// --------------------------------------------------

#[test]
fn le_u64_roundtrip() {
    for val in [0_u64, 1, u64::MAX / 2, u64::MAX] {
        let encoded = tinyklv::enc::binary::le_u64(val);
        let decoded = tinyklv::dec::binary::le_u64(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// u128
// --------------------------------------------------

#[test]
fn le_u128_roundtrip() {
    for val in [0_u128, 1, u128::MAX / 2, u128::MAX] {
        let encoded = tinyklv::enc::binary::le_u128(val);
        let decoded = tinyklv::dec::binary::le_u128(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i8
// --------------------------------------------------

#[test]
fn le_i8_roundtrip() {
    for val in [i8::MIN, -1_i8, 0, 1, i8::MAX] {
        let encoded = tinyklv::enc::binary::i8(val);
        let decoded = tinyklv::dec::binary::le_i8(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i16
// --------------------------------------------------

#[test]
fn le_i16_known_value() {
    // -2 as i16 in LE: 0xFFFE -> [0xFE, 0xFF]
    let mut input: &[u8] = &[0xFE, 0xFF];
    assert_eq!(tinyklv::dec::binary::le_i16(&mut input), Ok(-2_i16));
}

#[test]
fn le_i16_roundtrip() {
    for val in [i16::MIN, -1_i16, 0, 1, i16::MAX] {
        let encoded = tinyklv::enc::binary::le_i16(val);
        let decoded = tinyklv::dec::binary::le_i16(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i32
// --------------------------------------------------

#[test]
fn le_i32_roundtrip() {
    for val in [i32::MIN, -1_i32, 0, 1, i32::MAX] {
        let encoded = tinyklv::enc::binary::le_i32(val);
        let decoded = tinyklv::dec::binary::le_i32(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i64
// --------------------------------------------------

#[test]
fn le_i64_roundtrip() {
    for val in [i64::MIN, -1_i64, 0, 1, i64::MAX] {
        let encoded = tinyklv::enc::binary::le_i64(val);
        let decoded = tinyklv::dec::binary::le_i64(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// i128
// --------------------------------------------------

#[test]
fn le_i128_roundtrip() {
    for val in [i128::MIN, -1_i128, 0, 1, i128::MAX] {
        let encoded = tinyklv::enc::binary::le_i128(val);
        let decoded = tinyklv::dec::binary::le_i128(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// f32
// --------------------------------------------------

#[test]
fn le_f32_zero() {
    let encoded = tinyklv::enc::binary::le_f32(0.0_f32);
    let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(0.0_f32, decoded);
}

#[test]
fn le_f32_roundtrip_known() {
    let val = std::f32::consts::E;
    let encoded = tinyklv::enc::binary::le_f32(val);
    let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(val.to_bits(), decoded.to_bits());
}

#[test]
fn le_f32_infinity() {
    let encoded = tinyklv::enc::binary::le_f32(f32::INFINITY);
    let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_infinite() && decoded.is_sign_positive());
}

#[test]
fn le_f32_nan() {
    let encoded = tinyklv::enc::binary::le_f32(f32::NAN);
    let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_nan());
}

// --------------------------------------------------
// f64
// --------------------------------------------------

#[test]
fn le_f64_zero() {
    let encoded = tinyklv::enc::binary::le_f64(0.0_f64);
    let decoded = tinyklv::dec::binary::le_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(0.0_f64, decoded);
}

#[test]
fn le_f64_roundtrip_known() {
    let val = std::f64::consts::E;
    let encoded = tinyklv::enc::binary::le_f64(val);
    let decoded = tinyklv::dec::binary::le_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(val.to_bits(), decoded.to_bits());
}

#[test]
fn le_f64_nan() {
    let encoded = tinyklv::enc::binary::le_f64(f64::NAN);
    let decoded = tinyklv::dec::binary::le_f64(&mut encoded.as_slice()).unwrap();
    assert!(decoded.is_nan());
}

// --------------------------------------------------
// BE vs LE produce different bytes for multi-byte
// values
// --------------------------------------------------

#[test]
fn be_vs_le_u16_differ() {
    let val = 0x0102_u16;
    let be = tinyklv::enc::binary::be_u16(val);
    let le = tinyklv::enc::binary::le_u16(val);
    assert_eq!(be, vec![0x01, 0x02]);
    assert_eq!(le, vec![0x02, 0x01]);
}

#[test]
fn be_vs_le_u32_differ() {
    let val = 0x01020304_u32;
    let be = tinyklv::enc::binary::be_u32(val);
    let le = tinyklv::enc::binary::le_u32(val);
    assert_eq!(be, vec![0x01, 0x02, 0x03, 0x04]);
    assert_eq!(le, vec![0x04, 0x03, 0x02, 0x01]);
}
