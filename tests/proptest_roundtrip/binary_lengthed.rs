// --------------------------------------------------
// external
// --------------------------------------------------
use proptest::prelude::*;

// --------------------------------------------------
// exact-size roundtrips: enc(x) |> dec(sizeof(T)) == x
// --------------------------------------------------

proptest! {
    #[test]
    fn be_u16_lengthed_exact(val: u16) {
        let encoded = tinyklv::enc::binary::be_u16_lengthed(2)(val);
        let decoded = tinyklv::dec::binary::be_u16_lengthed(2)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    fn be_u32_lengthed_exact(val: u32) {
        let encoded = tinyklv::enc::binary::be_u32_lengthed(4)(val);
        let decoded = tinyklv::dec::binary::be_u32_lengthed(4)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    fn be_u64_lengthed_exact(val: u64) {
        let encoded = tinyklv::enc::binary::be_u64_lengthed(8)(val);
        let decoded = tinyklv::dec::binary::be_u64_lengthed(8)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    fn le_u16_lengthed_exact(val: u16) {
        let encoded = tinyklv::enc::binary::le_u16_lengthed(2)(val);
        let decoded = tinyklv::dec::binary::le_u16_lengthed(2)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    fn le_u32_lengthed_exact(val: u32) {
        let encoded = tinyklv::enc::binary::le_u32_lengthed(4)(val);
        let decoded = tinyklv::dec::binary::le_u32_lengthed(4)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    fn le_u64_lengthed_exact(val: u64) {
        let encoded = tinyklv::enc::binary::le_u64_lengthed(8)(val);
        let decoded = tinyklv::dec::binary::le_u64_lengthed(8)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// output length invariant: enc always produces
// exactly `len` bytes
// --------------------------------------------------

proptest! {
    #[test]
    fn be_u16_lengthed_output_len(val: u16, len in 1usize..=4) {
        let encoded = tinyklv::enc::binary::be_u16_lengthed(len)(val);
        prop_assert_eq!(encoded.len(), len);
    }

    #[test]
    fn be_u32_lengthed_output_len(val: u32, len in 1usize..=6) {
        let encoded = tinyklv::enc::binary::be_u32_lengthed(len)(val);
        prop_assert_eq!(encoded.len(), len);
    }

    #[test]
    fn le_u16_lengthed_output_len(val: u16, len in 1usize..=4) {
        let encoded = tinyklv::enc::binary::le_u16_lengthed(len)(val);
        prop_assert_eq!(encoded.len(), len);
    }

    #[test]
    fn le_u32_lengthed_output_len(val: u32, len in 1usize..=6) {
        let encoded = tinyklv::enc::binary::le_u32_lengthed(len)(val);
        prop_assert_eq!(encoded.len(), len);
    }
}

// --------------------------------------------------
// idempotency: enc(dec(enc(x, len), len), len) == enc(x, len)
// verifies that a re-encode after a decode produces
// identical bytes
// --------------------------------------------------

proptest! {
    #[test]
    fn be_u16_lengthed_idempotent(val: u16) {
        let enc1 = tinyklv::enc::binary::be_u16_lengthed(2)(val);
        let decoded = tinyklv::dec::binary::be_u16_lengthed(2)(&mut enc1.as_slice()).unwrap();
        let enc2 = tinyklv::enc::binary::be_u16_lengthed(2)(decoded);
        prop_assert_eq!(enc1, enc2);
    }

    #[test]
    fn le_u32_lengthed_idempotent(val: u32) {
        let enc1 = tinyklv::enc::binary::le_u32_lengthed(4)(val);
        let decoded = tinyklv::dec::binary::le_u32_lengthed(4)(&mut enc1.as_slice()).unwrap();
        let enc2 = tinyklv::enc::binary::le_u32_lengthed(4)(decoded);
        prop_assert_eq!(enc1, enc2);
    }
}

// --------------------------------------------------
// truncation: values that fit in fewer bytes survive
// a shorter-than-sizeof enc/dec cycle
// the low byte(s) must survive a 1-byte lengthed
// encode/decode cycle
// --------------------------------------------------

proptest! {
    #[test]
    fn be_u16_lengthed_1byte_low_byte(val: u8) {
        // encoding a u16 whose value fits in u8 with len=1 must round-trip cleanly
        let full: u16 = val as u16;
        let encoded = tinyklv::enc::binary::be_u16_lengthed(1)(full);
        prop_assert_eq!(encoded.len(), 1);
        let decoded = tinyklv::dec::binary::be_u16_lengthed(1)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(full, decoded);
    }

    #[test]
    fn le_u32_lengthed_1byte_low_byte(val: u8) {
        let full: u32 = val as u32;
        let encoded = tinyklv::enc::binary::le_u32_lengthed(1)(full);
        prop_assert_eq!(encoded.len(), 1);
        let decoded = tinyklv::dec::binary::le_u32_lengthed(1)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(full, decoded);
    }
}
