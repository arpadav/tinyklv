//! Property-based roundtrip and invariant tests for lengthed binary codecs
//!
//! Tests `be_u16_lengthed`, `be_u32_lengthed`, `be_u64_lengthed`,
//! `le_u16_lengthed`, `le_u32_lengthed`, and `le_u64_lengthed` via
//! `proptest`. Covers exact-size roundtrips (`len == sizeof(T)`), output-
//! length invariants (`encoded.len() == len`), idempotency (encode-decode-
//! encode produces identical bytes), and single-byte roundtrips for values
//! fitting in one byte
//!
//! Author: aav
// --------------------------------------------------
// external
// --------------------------------------------------
use proptest::prelude::*;

proptest! {
    #[test]
    /// Property: `be_u16_lengthed(2)` encode/decode roundtrip holds for all u16 values
    fn be_u16_lengthed_exact(val: u16) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u16_lengthed(2)(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_u16_lengthed(2)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    /// Property: `be_u32_lengthed(4)` encode/decode roundtrip holds for all u32 values
    fn be_u32_lengthed_exact(val: u32) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u32_lengthed(4)(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_u32_lengthed(4)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    /// Property: `be_u64_lengthed(8)` encode/decode roundtrip holds for all u64 values
    fn be_u64_lengthed_exact(val: u64) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u64_lengthed(8)(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_u64_lengthed(8)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    /// Property: `le_u16_lengthed(2)` encode/decode roundtrip holds for all u16 values
    fn le_u16_lengthed_exact(val: u16) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::le_u16_lengthed(2)(val, &mut encoded);
        let decoded = tinyklv::dec::binary::le_u16_lengthed(2)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    /// Property: `le_u32_lengthed(4)` encode/decode roundtrip holds for all u32 values
    fn le_u32_lengthed_exact(val: u32) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::le_u32_lengthed(4)(val, &mut encoded);
        let decoded = tinyklv::dec::binary::le_u32_lengthed(4)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    #[test]
    /// Property: `le_u64_lengthed(8)` encode/decode roundtrip holds for all u64 values
    fn le_u64_lengthed_exact(val: u64) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::le_u64_lengthed(8)(val, &mut encoded);
        let decoded = tinyklv::dec::binary::le_u64_lengthed(8)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }
}

proptest! {
    #[test]
    /// Property: `be_u16_lengthed(len)` always produces exactly `len` bytes of output
    fn be_u16_lengthed_output_len(val: u16, len in 1usize..=4) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u16_lengthed(len)(val, &mut encoded);
        prop_assert_eq!(encoded.len(), len);
    }

    #[test]
    /// Property: `be_u32_lengthed(len)` always produces exactly `len` bytes of output
    fn be_u32_lengthed_output_len(val: u32, len in 1usize..=6) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u32_lengthed(len)(val, &mut encoded);
        prop_assert_eq!(encoded.len(), len);
    }

    #[test]
    /// Property: `le_u16_lengthed(len)` always produces exactly `len` bytes of output
    fn le_u16_lengthed_output_len(val: u16, len in 1usize..=4) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::le_u16_lengthed(len)(val, &mut encoded);
        prop_assert_eq!(encoded.len(), len);
    }

    #[test]
    /// Property: `le_u32_lengthed(len)` always produces exactly `len` bytes of output
    fn le_u32_lengthed_output_len(val: u32, len in 1usize..=6) {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::le_u32_lengthed(len)(val, &mut encoded);
        prop_assert_eq!(encoded.len(), len);
    }
}

proptest! {
    #[test]
    /// Property: `be_u16_lengthed` encode after decode-encode produces the same bytes (idempotent)
    fn be_u16_lengthed_idempotent(val: u16) {
        let mut enc1 = Vec::new();
        tinyklv::enc::binary::be_u16_lengthed(2)(val, &mut enc1);
        let decoded = tinyklv::dec::binary::be_u16_lengthed(2)(&mut enc1.as_slice()).unwrap();
        let mut enc2 = Vec::new();
        tinyklv::enc::binary::be_u16_lengthed(2)(decoded, &mut enc2);
        prop_assert_eq!(enc1, enc2);
    }

    #[test]
    /// Property: `le_u32_lengthed` encode after decode-encode produces the same bytes (idempotent)
    fn le_u32_lengthed_idempotent(val: u32) {
        let mut enc1 = Vec::new();
        tinyklv::enc::binary::le_u32_lengthed(4)(val, &mut enc1);
        let decoded = tinyklv::dec::binary::le_u32_lengthed(4)(&mut enc1.as_slice()).unwrap();
        let mut enc2 = Vec::new();
        tinyklv::enc::binary::le_u32_lengthed(4)(decoded, &mut enc2);
        prop_assert_eq!(enc1, enc2);
    }
}

proptest! {
    #[test]
    /// Property: encoding a u16 that fits in a byte with `be_u16_lengthed(1)` roundtrips cleanly
    fn be_u16_lengthed_1byte_low_byte(val: u8) {
        // encoding a u16 whose value fits in u8 with len=1 must round-trip cleanly
        let full: u16 = val as u16;
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u16_lengthed(1)(full, &mut encoded);
        prop_assert_eq!(encoded.len(), 1);
        let decoded = tinyklv::dec::binary::be_u16_lengthed(1)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(full, decoded);
    }

    #[test]
    /// Property: encoding a u32 that fits in a byte with `le_u32_lengthed(1)` roundtrips cleanly
    fn le_u32_lengthed_1byte_low_byte(val: u8) {
        let full: u32 = val as u32;
        let mut encoded = Vec::new();
        tinyklv::enc::binary::le_u32_lengthed(1)(full, &mut encoded);
        prop_assert_eq!(encoded.len(), 1);
        let decoded = tinyklv::dec::binary::le_u32_lengthed(1)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(full, decoded);
    }
}
