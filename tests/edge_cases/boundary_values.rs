/// numeric boundary values: 0, 1, MIN, MAX for each type
macro_rules! boundary_roundtrip {
    ($name:ident, $ty:ty, $enc:path, $dec:path, [$($val:expr),+]) => {
        #[test]
        fn $name() {
            for val in [$($val as $ty),+] {
                let encoded = $enc(val);
                let decoded = $dec(&mut encoded.as_slice()).unwrap();
                assert_eq!(val, decoded, "boundary roundtrip failed for value {:?}", val);
            }
        }
    };
}

boundary_roundtrip!(
    be_u8_boundaries,
    u8,
    tinyklv::enc::binary::u8,
    tinyklv::dec::binary::be_u8,
    [0, 1, 127, 128, 254, 255]
);

boundary_roundtrip!(
    be_u16_boundaries,
    u16,
    tinyklv::enc::binary::be_u16,
    tinyklv::dec::binary::be_u16,
    [0, 1, 255, 256, 32767, 32768, 65534, 65535]
);

boundary_roundtrip!(
    be_u32_boundaries,
    u32,
    tinyklv::enc::binary::be_u32,
    tinyklv::dec::binary::be_u32,
    [0, 1, 255, 256, 65535, 65536, 2147483647, 2147483648, 4294967295]
);

boundary_roundtrip!(
    be_u64_boundaries,
    u64,
    tinyklv::enc::binary::be_u64,
    tinyklv::dec::binary::be_u64,
    [0, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX]
);

boundary_roundtrip!(
    le_u16_boundaries,
    u16,
    tinyklv::enc::binary::le_u16,
    tinyklv::dec::binary::le_u16,
    [0, 1, 255, 256, u16::MAX / 2, u16::MAX]
);

boundary_roundtrip!(
    le_u32_boundaries,
    u32,
    tinyklv::enc::binary::le_u32,
    tinyklv::dec::binary::le_u32,
    [0, 1, u32::MAX / 2, u32::MAX]
);

boundary_roundtrip!(
    be_i8_boundaries,
    i8,
    tinyklv::enc::binary::i8,
    tinyklv::dec::binary::be_i8,
    [i8::MIN, -1, 0, 1, i8::MAX]
);

boundary_roundtrip!(
    be_i16_boundaries,
    i16,
    tinyklv::enc::binary::be_i16,
    tinyklv::dec::binary::be_i16,
    [i16::MIN, -256, -1, 0, 1, 256, i16::MAX]
);

boundary_roundtrip!(
    be_i32_boundaries,
    i32,
    tinyklv::enc::binary::be_i32,
    tinyklv::dec::binary::be_i32,
    [i32::MIN, -65536, -1, 0, 1, 65536, i32::MAX]
);

boundary_roundtrip!(
    be_i64_boundaries,
    i64,
    tinyklv::enc::binary::be_i64,
    tinyklv::dec::binary::be_i64,
    [i64::MIN, -1, 0, 1, i64::MAX]
);

#[test]
fn ber_length_boundary_0() {
    let encoded = tinyklv::enc::ber::ber_length(&0_u64);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 0_usize);
}

#[test]
fn ber_length_boundary_127() {
    let encoded = tinyklv::enc::ber::ber_length(&127_u64);
    assert_eq!(encoded.len(), 1, "127 must encode as short form (1 byte)");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 127_usize);
}

#[test]
fn ber_length_boundary_128() {
    let encoded = tinyklv::enc::ber::ber_length(&128_u64);
    assert!(encoded.len() > 1, "128 must encode as long form (> 1 byte)");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 128_usize);
}

#[test]
fn ber_length_boundary_255() {
    let encoded = tinyklv::enc::ber::ber_length(&255_u64);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 255_usize);
}

#[test]
fn ber_length_boundary_256() {
    let encoded = tinyklv::enc::ber::ber_length(&256_u64);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 256_usize);
}

#[test]
fn ber_length_boundary_u32_max() {
    let encoded = tinyklv::enc::ber::ber_length(&u32::MAX);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, u32::MAX as usize);
}

boundary_roundtrip!(
    be_u128_boundaries,
    u128,
    tinyklv::enc::binary::be_u128,
    tinyklv::dec::binary::be_u128,
    [0, 1, u128::MAX / 2, u128::MAX - 1, u128::MAX]
);

boundary_roundtrip!(
    le_u64_boundaries,
    u64,
    tinyklv::enc::binary::le_u64,
    tinyklv::dec::binary::le_u64,
    [0, 1, u64::MAX / 2, u64::MAX]
);

boundary_roundtrip!(
    be_i128_boundaries,
    i128,
    tinyklv::enc::binary::be_i128,
    tinyklv::dec::binary::be_i128,
    [i128::MIN, -1, 0, 1, i128::MAX]
);

#[test]
fn be_f32_boundaries() {
    for bits in [
        0.0_f32.to_bits(),
        1.0_f32.to_bits(),
        (-1.0_f32).to_bits(),
        f32::MIN.to_bits(),
        f32::MAX.to_bits(),
        f32::MIN_POSITIVE.to_bits(),
        f32::EPSILON.to_bits(),
    ] {
        let val = f32::from_bits(bits);
        let encoded = tinyklv::enc::binary::be_f32(val);
        let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), bits);
    }
}

#[test]
fn be_f32_infinity_roundtrip() {
    for val in [f32::INFINITY, f32::NEG_INFINITY] {
        let encoded = tinyklv::enc::binary::be_f32(val);
        let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), val.to_bits());
    }
}

#[test]
fn be_f32_nan_roundtrip_via_bits() {
    // NaN != NaN, so compare bit patterns directly
    let bits = f32::NAN.to_bits();
    let val = f32::from_bits(bits);
    let encoded = tinyklv::enc::binary::be_f32(val);
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.to_bits(), bits);
}

#[test]
fn be_f32_neg_zero_roundtrip() {
    let bits = (-0.0_f32).to_bits();
    let encoded = tinyklv::enc::binary::be_f32(f32::from_bits(bits));
    let decoded = tinyklv::dec::binary::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.to_bits(), bits);
}

#[test]
fn be_f64_boundaries() {
    for bits in [
        0.0_f64.to_bits(),
        1.0_f64.to_bits(),
        (-1.0_f64).to_bits(),
        f64::MIN.to_bits(),
        f64::MAX.to_bits(),
        f64::MIN_POSITIVE.to_bits(),
        f64::EPSILON.to_bits(),
    ] {
        let val = f64::from_bits(bits);
        let encoded = tinyklv::enc::binary::be_f64(val);
        let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), bits);
    }
}

#[test]
fn be_f64_infinity_roundtrip() {
    for val in [f64::INFINITY, f64::NEG_INFINITY] {
        let encoded = tinyklv::enc::binary::be_f64(val);
        let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), val.to_bits());
    }
}

#[test]
fn be_f64_nan_roundtrip_via_bits() {
    let bits = f64::NAN.to_bits();
    let encoded = tinyklv::enc::binary::be_f64(f64::from_bits(bits));
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.to_bits(), bits);
}

#[test]
fn be_f64_neg_zero_roundtrip() {
    let bits = (-0.0_f64).to_bits();
    let encoded = tinyklv::enc::binary::be_f64(f64::from_bits(bits));
    let decoded = tinyklv::dec::binary::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.to_bits(), bits);
}

#[test]
fn le_f32_boundaries() {
    for bits in [
        0.0_f32.to_bits(),
        1.0_f32.to_bits(),
        f32::MAX.to_bits(),
        f32::NAN.to_bits(),
        f32::INFINITY.to_bits(),
        f32::NEG_INFINITY.to_bits(),
        (-0.0_f32).to_bits(),
    ] {
        let val = f32::from_bits(bits);
        let encoded = tinyklv::enc::binary::le_f32(val);
        let decoded = tinyklv::dec::binary::le_f32(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), bits);
    }
}

#[test]
fn le_f64_boundaries() {
    for bits in [
        0.0_f64.to_bits(),
        1.0_f64.to_bits(),
        f64::MAX.to_bits(),
        f64::NAN.to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        (-0.0_f64).to_bits(),
    ] {
        let val = f64::from_bits(bits);
        let encoded = tinyklv::enc::binary::le_f64(val);
        let decoded = tinyklv::dec::binary::le_f64(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), bits);
    }
}
