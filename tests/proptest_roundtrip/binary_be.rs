use proptest::prelude::*;

macro_rules! roundtrip_test {
    ($(#[doc = $doc:literal])* $name:ident, $ty:ty, $enc:path, $dec:path) => {
        proptest! {
            $(#[doc = $doc])*
            #[test]
            fn $name(val: $ty) {
                let encoded = $enc(val);
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
                let encoded = $enc(val);
                let decoded = $dec(&mut encoded.as_slice()).unwrap();
                prop_assert_eq!(val.to_bits(), decoded.to_bits());
            }
        }
    };
}

// u8 has no be_/le_ encoder variant - single byte, endianness irrelevant.
// The native-endian encoder is tinyklv::enc::binary::u8; the decoder is u8
// (which is just winnow::binary::u8 with implied generics).
roundtrip_test!(
    /// Property: `u8` encode/`u8` decode roundtrip holds for all u8 values.
    u8_roundtrip,
    u8,
    tinyklv::enc::binary::u8,
    tinyklv::dec::binary::u8
);
roundtrip_test!(
    /// Property: `be_u16` encode/decode roundtrip holds for all u16 values.
    be_u16_roundtrip,
    u16,
    tinyklv::enc::binary::be_u16,
    tinyklv::dec::binary::be_u16
);
roundtrip_test!(
    /// Property: `be_u32` encode/decode roundtrip holds for all u32 values.
    be_u32_roundtrip,
    u32,
    tinyklv::enc::binary::be_u32,
    tinyklv::dec::binary::be_u32
);
roundtrip_test!(
    /// Property: `be_u64` encode/decode roundtrip holds for all u64 values.
    be_u64_roundtrip,
    u64,
    tinyklv::enc::binary::be_u64,
    tinyklv::dec::binary::be_u64
);
roundtrip_test!(
    /// Property: `be_u128` encode/decode roundtrip holds for all u128 values.
    be_u128_roundtrip,
    u128,
    tinyklv::enc::binary::be_u128,
    tinyklv::dec::binary::be_u128
);
// i8 has no be_/le_ encoder variant - single byte, endianness irrelevant.
roundtrip_test!(
    /// Property: `i8` encode/`i8` decode roundtrip holds for all i8 values.
    i8_roundtrip,
    i8,
    tinyklv::enc::binary::i8,
    tinyklv::dec::binary::i8
);
roundtrip_test!(
    /// Property: `be_i16` encode/decode roundtrip holds for all i16 values.
    be_i16_roundtrip,
    i16,
    tinyklv::enc::binary::be_i16,
    tinyklv::dec::binary::be_i16
);
roundtrip_test!(
    /// Property: `be_i32` encode/decode roundtrip holds for all i32 values.
    be_i32_roundtrip,
    i32,
    tinyklv::enc::binary::be_i32,
    tinyklv::dec::binary::be_i32
);
roundtrip_test!(
    /// Property: `be_i64` encode/decode roundtrip holds for all i64 values.
    be_i64_roundtrip,
    i64,
    tinyklv::enc::binary::be_i64,
    tinyklv::dec::binary::be_i64
);
roundtrip_test!(
    /// Property: `be_i128` encode/decode roundtrip holds for all i128 values.
    be_i128_roundtrip,
    i128,
    tinyklv::enc::binary::be_i128,
    tinyklv::dec::binary::be_i128
);
roundtrip_float!(
    /// Property: `be_f32` encode/decode preserves exact bit patterns across all f32 values (NaN bits included).
    be_f32_roundtrip,
    f32,
    tinyklv::enc::binary::be_f32,
    tinyklv::dec::binary::be_f32
);
roundtrip_float!(
    /// Property: `be_f64` encode/decode preserves exact bit patterns across all f64 values (NaN bits included).
    be_f64_roundtrip,
    f64,
    tinyklv::enc::binary::be_f64,
    tinyklv::dec::binary::be_f64
);

proptest! {
        #[test]
/// Property: `be_u16` always encodes to exactly `size_of::<u16>()` bytes.
    fn be_u16_encoding_length(val: u16) {
        let encoded = tinyklv::enc::binary::be_u16(val);
        prop_assert_eq!(encoded.len(), std::mem::size_of::<u16>());
    }

        #[test]
/// Property: `be_u32` always encodes to exactly `size_of::<u32>()` bytes.
    fn be_u32_encoding_length(val: u32) {
        let encoded = tinyklv::enc::binary::be_u32(val);
        prop_assert_eq!(encoded.len(), std::mem::size_of::<u32>());
    }

        #[test]
/// Property: `be_u64` always encodes to exactly `size_of::<u64>()` bytes.
    fn be_u64_encoding_length(val: u64) {
        let encoded = tinyklv::enc::binary::be_u64(val);
        prop_assert_eq!(encoded.len(), std::mem::size_of::<u64>());
    }

        #[test]
/// Property: `be_u128` always encodes to exactly `size_of::<u128>()` bytes.
    fn be_u128_encoding_length(val: u128) {
        let encoded = tinyklv::enc::binary::be_u128(val);
        prop_assert_eq!(encoded.len(), std::mem::size_of::<u128>());
    }

        #[test]
/// Property: `be_u16` output matches `u16::to_be_bytes`.
    fn be_u16_matches_to_be_bytes(val: u16) {
        let encoded = tinyklv::enc::binary::be_u16(val);
        prop_assert_eq!(encoded, val.to_be_bytes().to_vec());
    }

        #[test]
/// Property: `be_u32` output matches `u32::to_be_bytes`.
    fn be_u32_matches_to_be_bytes(val: u32) {
        let encoded = tinyklv::enc::binary::be_u32(val);
        prop_assert_eq!(encoded, val.to_be_bytes().to_vec());
    }

        #[test]
/// Property: `be_i32` output matches `i32::to_be_bytes`.
    fn be_i32_matches_to_be_bytes(val: i32) {
        let encoded = tinyklv::enc::binary::be_i32(val);
        prop_assert_eq!(encoded, val.to_be_bytes().to_vec());
    }

        #[test]
/// Property: `u8` encoder emits a single byte equal to the input value.
    fn u8_enc_single_byte(val: u8) {
        let encoded = tinyklv::enc::binary::u8(val);
        prop_assert_eq!(encoded.len(), 1);
        prop_assert_eq!(encoded[0], val);
    }

        #[test]
/// Property: `i8` encoder emits a single byte whose two's complement equals the input value.
    fn i8_enc_single_byte(val: i8) {
        let encoded = tinyklv::enc::binary::i8(val);
        prop_assert_eq!(encoded.len(), 1);
        prop_assert_eq!(encoded[0] as i8, val);
    }
}
