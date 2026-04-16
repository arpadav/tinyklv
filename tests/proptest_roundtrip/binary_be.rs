// --------------------------------------------------
// external
// --------------------------------------------------
use proptest::prelude::*;

macro_rules! roundtrip_test {
    ($name:ident, $ty:ty, $enc:path, $dec:path) => {
        proptest! {
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
    ($name:ident, $ty:ty, $enc:path, $dec:path) => {
        proptest! {
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
// The native-endian encoder is tinyklv::enc::binary::u8; the decoder is be_u8
// (which is just winnow::binary::be_u8 with implied generics).
roundtrip_test!(
    be_u8_roundtrip,
    u8,
    tinyklv::enc::binary::u8,
    tinyklv::dec::binary::be_u8
);
roundtrip_test!(
    be_u16_roundtrip,
    u16,
    tinyklv::enc::binary::be_u16,
    tinyklv::dec::binary::be_u16
);
roundtrip_test!(
    be_u32_roundtrip,
    u32,
    tinyklv::enc::binary::be_u32,
    tinyklv::dec::binary::be_u32
);
roundtrip_test!(
    be_u64_roundtrip,
    u64,
    tinyklv::enc::binary::be_u64,
    tinyklv::dec::binary::be_u64
);
roundtrip_test!(
    be_u128_roundtrip,
    u128,
    tinyklv::enc::binary::be_u128,
    tinyklv::dec::binary::be_u128
);
// i8 has no be_/le_ encoder variant - single byte, endianness irrelevant.
roundtrip_test!(
    be_i8_roundtrip,
    i8,
    tinyklv::enc::binary::i8,
    tinyklv::dec::binary::be_i8
);
roundtrip_test!(
    be_i16_roundtrip,
    i16,
    tinyklv::enc::binary::be_i16,
    tinyklv::dec::binary::be_i16
);
roundtrip_test!(
    be_i32_roundtrip,
    i32,
    tinyklv::enc::binary::be_i32,
    tinyklv::dec::binary::be_i32
);
roundtrip_test!(
    be_i64_roundtrip,
    i64,
    tinyklv::enc::binary::be_i64,
    tinyklv::dec::binary::be_i64
);
roundtrip_test!(
    be_i128_roundtrip,
    i128,
    tinyklv::enc::binary::be_i128,
    tinyklv::dec::binary::be_i128
);
roundtrip_float!(
    be_f32_roundtrip,
    f32,
    tinyklv::enc::binary::be_f32,
    tinyklv::dec::binary::be_f32
);
roundtrip_float!(
    be_f64_roundtrip,
    f64,
    tinyklv::enc::binary::be_f64,
    tinyklv::dec::binary::be_f64
);

proptest! {
    #[test]
    fn be_u16_encoding_length(val: u16) {
        let encoded = tinyklv::enc::binary::be_u16(val);
        prop_assert_eq!(encoded.len(), std::mem::size_of::<u16>());
    }

    #[test]
    fn be_u32_encoding_length(val: u32) {
        let encoded = tinyklv::enc::binary::be_u32(val);
        prop_assert_eq!(encoded.len(), std::mem::size_of::<u32>());
    }

    #[test]
    fn be_u64_encoding_length(val: u64) {
        let encoded = tinyklv::enc::binary::be_u64(val);
        prop_assert_eq!(encoded.len(), std::mem::size_of::<u64>());
    }

    #[test]
    fn be_u128_encoding_length(val: u128) {
        let encoded = tinyklv::enc::binary::be_u128(val);
        prop_assert_eq!(encoded.len(), std::mem::size_of::<u128>());
    }

    #[test]
    fn be_u16_matches_to_be_bytes(val: u16) {
        let encoded = tinyklv::enc::binary::be_u16(val);
        prop_assert_eq!(encoded, val.to_be_bytes().to_vec());
    }

    #[test]
    fn be_u32_matches_to_be_bytes(val: u32) {
        let encoded = tinyklv::enc::binary::be_u32(val);
        prop_assert_eq!(encoded, val.to_be_bytes().to_vec());
    }

    #[test]
    fn be_i32_matches_to_be_bytes(val: i32) {
        let encoded = tinyklv::enc::binary::be_i32(val);
        prop_assert_eq!(encoded, val.to_be_bytes().to_vec());
    }

    #[test]
    fn u8_enc_single_byte(val: u8) {
        let encoded = tinyklv::enc::binary::u8(val);
        prop_assert_eq!(encoded.len(), 1);
        prop_assert_eq!(encoded[0], val);
    }

    #[test]
    fn i8_enc_single_byte(val: i8) {
        let encoded = tinyklv::enc::binary::i8(val);
        prop_assert_eq!(encoded.len(), 1);
        prop_assert_eq!(encoded[0] as i8, val);
    }
}
