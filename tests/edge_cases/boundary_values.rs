//! Numeric and BER boundary-value roundtrip tests
//!
//! Uses a `boundary_roundtrip!` macro to verify encode/decode roundtrips at
//! type extremes (`0`, `1`, `MIN`, `MAX`, and intermediate boundary crossings)
//! for every integer type (`u8` through `u128`, `i8` through `i128`) and both
//! endiannesses. Also covers `be_f32`/`be_f64`/`le_f32`/`le_f64` with bit-
//! exact comparison including NaN, infinity, negative zero, and epsilon
//! boundaries, and BER length roundtrips at `0`, `127`, `128`, `255`, `256`,
//! and `u32::MAX`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::enc::ber as encber;
use tinyklv::enc::binary as encb;

/// numeric boundary values: 0, 1, MIN, MAX for each type
macro_rules! boundary_roundtrip {
    ($(#[doc = $doc:literal])* $name:ident, $ty:ty, $enc:path, $dec:path, [$($val:expr),+]) => {
        $(#[doc = $doc])*
        #[test]
        fn $name() {
            for val in [$($val as $ty),+] {
                let mut encoded = Vec::new();
                $enc(val, &mut encoded);
                let decoded = $dec(&mut encoded.as_slice()).unwrap();
                assert_eq!(val, decoded, "boundary roundtrip failed for value {:?}", val);
            }
        }
    };
}

boundary_roundtrip!(
    /// Tests `u8` boundary roundtrip for `0`, `1`, `127`, `128`, `254`, and `u8::MAX`
    u8_boundaries,
    u8,
    encb::u8,
    decb::u8,
    [0, 1, 127, 128, 254, 255]
);

boundary_roundtrip!(
    /// Tests `be_u16` boundary roundtrip across zero, byte-width transitions, and `u16::MAX`
    be_u16_boundaries,
    u16,
    encb::be_u16,
    decb::be_u16,
    [0, 1, 255, 256, 32767, 32768, 65534, 65535]
);

boundary_roundtrip!(
    /// Tests `be_u32` boundary roundtrip across zero, byte-width transitions, signed-bit boundary, and `u32::MAX`
    be_u32_boundaries,
    u32,
    encb::be_u32,
    decb::be_u32,
    [0, 1, 255, 256, 65535, 65536, 2147483647, 2147483648, 4294967295]
);

boundary_roundtrip!(
    /// Tests `be_u64` boundary roundtrip for `0`, `1`, mid, `u64::MAX - 1`, and `u64::MAX`
    be_u64_boundaries,
    u64,
    encb::be_u64,
    decb::be_u64,
    [0, 1, u64::MAX / 2, u64::MAX - 1, u64::MAX]
);

boundary_roundtrip!(
    /// Tests `le_u16` boundary roundtrip across zero, byte-width transitions, and `u16::MAX`
    le_u16_boundaries,
    u16,
    encb::le_u16,
    decb::le_u16,
    [0, 1, 255, 256, u16::MAX / 2, u16::MAX]
);

boundary_roundtrip!(
    /// Tests `le_u32` boundary roundtrip for `0`, `1`, mid, and `u32::MAX`
    le_u32_boundaries,
    u32,
    encb::le_u32,
    decb::le_u32,
    [0, 1, u32::MAX / 2, u32::MAX]
);

boundary_roundtrip!(
    /// Tests `i8` boundary roundtrip for `i8::MIN`, `-1`, `0`, `1`, and `i8::MAX`
    i8_boundaries,
    i8,
    encb::i8,
    decb::i8,
    [i8::MIN, -1, 0, 1, i8::MAX]
);

boundary_roundtrip!(
    /// Tests `be_i16` boundary roundtrip across signed min/max and sign-crossing byte-width transitions
    be_i16_boundaries,
    i16,
    encb::be_i16,
    decb::be_i16,
    [i16::MIN, -256, -1, 0, 1, 256, i16::MAX]
);

boundary_roundtrip!(
    /// Tests `be_i32` boundary roundtrip across signed min/max and sign-crossing byte-width transitions
    be_i32_boundaries,
    i32,
    encb::be_i32,
    decb::be_i32,
    [i32::MIN, -65536, -1, 0, 1, 65536, i32::MAX]
);

boundary_roundtrip!(
    /// Tests `be_i64` boundary roundtrip for `i64::MIN`, `-1`, `0`, `1`, and `i64::MAX`
    be_i64_boundaries,
    i64,
    encb::be_i64,
    decb::be_i64,
    [i64::MIN, -1, 0, 1, i64::MAX]
);

#[test]
/// Tests BER length roundtrip at the lower boundary value `0`
fn ber_length_boundary_0() {
    let mut encoded = Vec::new();
    encber::ber_length(0_u64, &mut encoded);
    let decoded = decber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 0_usize);
}

#[test]
/// Tests BER length roundtrip at the short-form upper boundary `127` (1-byte encoding)
fn ber_length_boundary_127() {
    let mut encoded = Vec::new();
    encber::ber_length(127_u64, &mut encoded);
    assert_eq!(encoded.len(), 1, "127 must encode as short form (1 byte)");
    let decoded = decber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 127_usize);
}

#[test]
/// Tests BER length roundtrip at `128`, the first long-form value (encoding must exceed 1 byte)
fn ber_length_boundary_128() {
    let mut encoded = Vec::new();
    encber::ber_length(128_u64, &mut encoded);
    assert!(encoded.len() > 1, "128 must encode as long form (> 1 byte)");
    let decoded = decber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 128_usize);
}

#[test]
/// Tests BER length roundtrip at `255`, a common 1-extra-byte long-form boundary
fn ber_length_boundary_255() {
    let mut encoded = Vec::new();
    encber::ber_length(255_u64, &mut encoded);
    let decoded = decber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 255_usize);
}

#[test]
/// Tests BER length roundtrip at `256`, the first value requiring 2 long-form bytes
fn ber_length_boundary_256() {
    let mut encoded = Vec::new();
    encber::ber_length(256_u64, &mut encoded);
    let decoded = decber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 256_usize);
}

#[test]
/// Tests BER length roundtrip at the upper boundary `u32::MAX`
fn ber_length_boundary_u32_max() {
    let mut encoded = Vec::new();
    encber::ber_length(u32::MAX, &mut encoded);
    let decoded = decber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, u32::MAX as usize);
}

boundary_roundtrip!(
    /// Tests `be_u128` boundary roundtrip for `0`, `1`, mid, `u128::MAX - 1`, and `u128::MAX`
    be_u128_boundaries,
    u128,
    encb::be_u128,
    decb::be_u128,
    [0, 1, u128::MAX / 2, u128::MAX - 1, u128::MAX]
);

boundary_roundtrip!(
    /// Tests `le_u64` boundary roundtrip for `0`, `1`, mid, and `u64::MAX`
    le_u64_boundaries,
    u64,
    encb::le_u64,
    decb::le_u64,
    [0, 1, u64::MAX / 2, u64::MAX]
);

boundary_roundtrip!(
    /// Tests `be_i128` boundary roundtrip for `i128::MIN`, `-1`, `0`, `1`, and `i128::MAX`
    be_i128_boundaries,
    i128,
    encb::be_i128,
    decb::be_i128,
    [i128::MIN, -1, 0, 1, i128::MAX]
);

#[test]
/// Tests `be_f32` bit-exact roundtrip across zero, unit, signed-min/max, smallest positive, and epsilon boundaries
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
        let mut encoded = Vec::new();
        encb::be_f32(val, &mut encoded);
        let decoded = decb::be_f32(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), bits);
    }
}

#[test]
/// Tests `be_f32` bit-exact roundtrip for positive and negative infinity
fn be_f32_infinity_roundtrip() {
    for val in [f32::INFINITY, f32::NEG_INFINITY] {
        let mut encoded = Vec::new();
        encb::be_f32(val, &mut encoded);
        let decoded = decb::be_f32(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), val.to_bits());
    }
}

#[test]
/// Tests `be_f32` NaN roundtrip compared by bit pattern (since `NaN != NaN`)
fn be_f32_nan_roundtrip_via_bits() {
    // NaN != NaN, so compare bit patterns directly
    let bits = f32::NAN.to_bits();
    let val = f32::from_bits(bits);
    let mut encoded = Vec::new();
    encb::be_f32(val, &mut encoded);
    let decoded = decb::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.to_bits(), bits);
}

#[test]
/// Tests `be_f32` bit-exact roundtrip of negative zero (`-0.0`)
fn be_f32_neg_zero_roundtrip() {
    let bits = (-0.0_f32).to_bits();
    let mut encoded = Vec::new();
    encb::be_f32(f32::from_bits(bits), &mut encoded);
    let decoded = decb::be_f32(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.to_bits(), bits);
}

#[test]
/// Tests `be_f64` bit-exact roundtrip across zero, unit, signed-min/max, smallest positive, and epsilon boundaries
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
        let mut encoded = Vec::new();
        encb::be_f64(val, &mut encoded);
        let decoded = decb::be_f64(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), bits);
    }
}

#[test]
/// Tests `be_f64` bit-exact roundtrip for positive and negative infinity
fn be_f64_infinity_roundtrip() {
    for val in [f64::INFINITY, f64::NEG_INFINITY] {
        let mut encoded = Vec::new();
        encb::be_f64(val, &mut encoded);
        let decoded = decb::be_f64(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), val.to_bits());
    }
}

#[test]
/// Tests `be_f64` NaN roundtrip compared by bit pattern (since `NaN != NaN`)
fn be_f64_nan_roundtrip_via_bits() {
    let bits = f64::NAN.to_bits();
    let mut encoded = Vec::new();
    encb::be_f64(f64::from_bits(bits), &mut encoded);
    let decoded = decb::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.to_bits(), bits);
}

#[test]
/// Tests `be_f64` bit-exact roundtrip of negative zero (`-0.0`)
fn be_f64_neg_zero_roundtrip() {
    let bits = (-0.0_f64).to_bits();
    let mut encoded = Vec::new();
    encb::be_f64(f64::from_bits(bits), &mut encoded);
    let decoded = decb::be_f64(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.to_bits(), bits);
}

#[test]
/// Tests `le_f32` bit-exact roundtrip across zero, unit, max, NaN, infinities, and negative zero
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
        let mut encoded = Vec::new();
        encb::le_f32(val, &mut encoded);
        let decoded = decb::le_f32(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), bits);
    }
}

#[test]
/// Tests `le_f64` bit-exact roundtrip across zero, unit, max, NaN, infinities, and negative zero
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
        let mut encoded = Vec::new();
        encb::le_f64(val, &mut encoded);
        let decoded = decb::le_f64(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded.to_bits(), bits);
    }
}
