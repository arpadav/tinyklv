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

roundtrip_test!(
    le_u16_roundtrip,
    u16,
    tinyklv::enc::binary::le_u16,
    tinyklv::dec::binary::le_u16
);
roundtrip_test!(
    le_u32_roundtrip,
    u32,
    tinyklv::enc::binary::le_u32,
    tinyklv::dec::binary::le_u32
);
roundtrip_test!(
    le_u64_roundtrip,
    u64,
    tinyklv::enc::binary::le_u64,
    tinyklv::dec::binary::le_u64
);
roundtrip_test!(
    le_u128_roundtrip,
    u128,
    tinyklv::enc::binary::le_u128,
    tinyklv::dec::binary::le_u128
);
roundtrip_test!(
    le_i16_roundtrip,
    i16,
    tinyklv::enc::binary::le_i16,
    tinyklv::dec::binary::le_i16
);
roundtrip_test!(
    le_i32_roundtrip,
    i32,
    tinyklv::enc::binary::le_i32,
    tinyklv::dec::binary::le_i32
);
roundtrip_test!(
    le_i64_roundtrip,
    i64,
    tinyklv::enc::binary::le_i64,
    tinyklv::dec::binary::le_i64
);
roundtrip_test!(
    le_i128_roundtrip,
    i128,
    tinyklv::enc::binary::le_i128,
    tinyklv::dec::binary::le_i128
);
roundtrip_float!(
    le_f32_roundtrip,
    f32,
    tinyklv::enc::binary::le_f32,
    tinyklv::dec::binary::le_f32
);
roundtrip_float!(
    le_f64_roundtrip,
    f64,
    tinyklv::enc::binary::le_f64,
    tinyklv::dec::binary::le_f64
);

proptest! {
    /// LE and BE encodings of the same value must have equal length
    #[test]
    fn le_be_same_length_u16(val: u16) {
        let be = tinyklv::enc::binary::be_u16(val);
        let le = tinyklv::enc::binary::le_u16(val);
        prop_assert_eq!(be.len(), le.len());
    }

    #[test]
    fn le_be_same_length_u32(val: u32) {
        let be = tinyklv::enc::binary::be_u32(val);
        let le = tinyklv::enc::binary::le_u32(val);
        prop_assert_eq!(be.len(), le.len());
    }

    #[test]
    fn le_be_same_length_u64(val: u64) {
        let be = tinyklv::enc::binary::be_u64(val);
        let le = tinyklv::enc::binary::le_u64(val);
        prop_assert_eq!(be.len(), le.len());
    }

    /// LE bytes are the reverse of BE bytes for multi-byte primitives
    #[test]
    fn le_is_reverse_of_be_u16(val: u16) {
        let be = tinyklv::enc::binary::be_u16(val);
        let mut le = tinyklv::enc::binary::le_u16(val);
        le.reverse();
        prop_assert_eq!(be, le);
    }

    #[test]
    fn le_is_reverse_of_be_u32(val: u32) {
        let be = tinyklv::enc::binary::be_u32(val);
        let mut le = tinyklv::enc::binary::le_u32(val);
        le.reverse();
        prop_assert_eq!(be, le);
    }

    #[test]
    fn le_u16_matches_to_le_bytes(val: u16) {
        let encoded = tinyklv::enc::binary::le_u16(val);
        prop_assert_eq!(encoded, val.to_le_bytes().to_vec());
    }

    #[test]
    fn le_u32_matches_to_le_bytes(val: u32) {
        let encoded = tinyklv::enc::binary::le_u32(val);
        prop_assert_eq!(encoded, val.to_le_bytes().to_vec());
    }
}
