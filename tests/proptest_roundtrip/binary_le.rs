//! Property-based roundtrip tests for little-endian binary codecs
//!
//! Uses `proptest` with two macros (`roundtrip_test!` for integers and
//! `roundtrip_float!` for bit-exact float comparison) to verify that every
//! little-endian codec satisfies `decode(encode(v)) == v` for arbitrary inputs
//! Also includes LE-vs-BE equal-length properties, byte-reverse symmetry
//! assertions, and `to_le_bytes` match properties
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use proptest::prelude::*;

macro_rules! roundtrip_test {
    ($(#[doc = $doc:literal])* $name:ident, $ty:ty, $enc:path, $dec:path) => {
        proptest! {
            $(#[doc = $doc])*
            #[test]
            fn $name(val: $ty) {
                let mut encoded = Vec::new();
                $enc(val, &mut encoded);
                let decoded = $dec(&mut encoded.as_slice()).unwrap();
                prop_assert_eq!(val, decoded);
            }
        }
    };
}

macro_rules! roundtrip_float {
    ($(#[doc = $doc:literal])* $name:ident, $ty:ty, $enc:path, $dec:path) => {
        proptest! {
            $(#[doc = $doc])*
            #[test]
            fn $name(val: $ty) {
                let mut encoded = Vec::new();
                $enc(val, &mut encoded);
                let decoded = $dec(&mut encoded.as_slice()).unwrap();
                prop_assert_eq!(val.to_bits(), decoded.to_bits());
            }
        }
    };
}

roundtrip_test!(
    /// Property: `le_u16` encode/decode roundtrip holds for all u16 values
    le_u16_roundtrip,
    u16,
    tinyklv::enc::binary::le_u16,
    tinyklv::dec::binary::le_u16
);
roundtrip_test!(
    /// Property: `le_u32` encode/decode roundtrip holds for all u32 values
    le_u32_roundtrip,
    u32,
    tinyklv::enc::binary::le_u32,
    tinyklv::dec::binary::le_u32
);
roundtrip_test!(
    /// Property: `le_u64` encode/decode roundtrip holds for all u64 values
    le_u64_roundtrip,
    u64,
    tinyklv::enc::binary::le_u64,
    tinyklv::dec::binary::le_u64
);
roundtrip_test!(
    /// Property: `le_u128` encode/decode roundtrip holds for all u128 values
    le_u128_roundtrip,
    u128,
    tinyklv::enc::binary::le_u128,
    tinyklv::dec::binary::le_u128
);
roundtrip_test!(
    /// Property: `le_i16` encode/decode roundtrip holds for all i16 values
    le_i16_roundtrip,
    i16,
    tinyklv::enc::binary::le_i16,
    tinyklv::dec::binary::le_i16
);
roundtrip_test!(
    /// Property: `le_i32` encode/decode roundtrip holds for all i32 values
    le_i32_roundtrip,
    i32,
    tinyklv::enc::binary::le_i32,
    tinyklv::dec::binary::le_i32
);
roundtrip_test!(
    /// Property: `le_i64` encode/decode roundtrip holds for all i64 values
    le_i64_roundtrip,
    i64,
    tinyklv::enc::binary::le_i64,
    tinyklv::dec::binary::le_i64
);
roundtrip_test!(
    /// Property: `le_i128` encode/decode roundtrip holds for all i128 values
    le_i128_roundtrip,
    i128,
    tinyklv::enc::binary::le_i128,
    tinyklv::dec::binary::le_i128
);
roundtrip_float!(
    /// Property: `le_f32` encode/decode preserves exact bit patterns across all f32 values
    le_f32_roundtrip,
    f32,
    tinyklv::enc::binary::le_f32,
    tinyklv::dec::binary::le_f32
);
roundtrip_float!(
    /// Property: `le_f64` encode/decode preserves exact bit patterns across all f64 values
    le_f64_roundtrip,
    f64,
    tinyklv::enc::binary::le_f64,
    tinyklv::dec::binary::le_f64
);

proptest! {
        #[test]
/// Property: LE and BE u16 encodings have equal length for all values
    fn le_be_same_length_u16(val: u16) {
        let mut be = Vec::new();
        tinyklv::enc::binary::be_u16(val, &mut be);
        let mut le = Vec::new();
        tinyklv::enc::binary::le_u16(val, &mut le);
        prop_assert_eq!(be.len(), le.len());
    }

        #[test]
/// Property: LE and BE u32 encodings have equal length for all values
    fn le_be_same_length_u32(val: u32) {
        let mut be = Vec::new();
        tinyklv::enc::binary::be_u32(val, &mut be);
        let mut le = Vec::new();
        tinyklv::enc::binary::le_u32(val, &mut le);
        prop_assert_eq!(be.len(), le.len());
    }

        #[test]
/// Property: LE and BE u64 encodings have equal length for all values
    fn le_be_same_length_u64(val: u64) {
        let mut be = Vec::new();
        tinyklv::enc::binary::be_u64(val, &mut be);
        let mut le = Vec::new();
        tinyklv::enc::binary::le_u64(val, &mut le);
        prop_assert_eq!(be.len(), le.len());
    }

        #[test]
/// Property: for any u16, the LE encoding is the byte-reverse of the BE encoding
    fn le_is_reverse_of_be_u16(val: u16) {
        let mut be = Vec::new();
        tinyklv::enc::binary::be_u16(val, &mut be);
        let mut le = Vec::new();
        tinyklv::enc::binary::le_u16(val, &mut le);
        le.reverse();
        prop_assert_eq!(be, le);
    }

        #[test]
/// Property: for any u32, the LE encoding is the byte-reverse of the BE encoding
    fn le_is_reverse_of_be_u32(val: u32) {
        let mut be = Vec::new();
        tinyklv::enc::binary::be_u32(val, &mut be);
        let mut le = Vec::new();
        tinyklv::enc::binary::le_u32(val, &mut le);
        le.reverse();
        prop_assert_eq!(be, le);
    }

        #[test]
/// Property: `le_u16` output matches `u16::to_le_bytes`
    fn le_u16_matches_to_le_bytes(val: u16) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::le_u16(val, &mut encoded);
        prop_assert_eq!(encoded, val.to_le_bytes().to_vec());
    }

        #[test]
/// Property: `le_u32` output matches `u32::to_le_bytes`
    fn le_u32_matches_to_le_bytes(val: u32) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::le_u32(val, &mut encoded);
        prop_assert_eq!(encoded, val.to_le_bytes().to_vec());
    }
}
