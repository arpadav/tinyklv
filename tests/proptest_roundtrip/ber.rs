//! Property-based roundtrip tests for BER length and OID codecs
//!
//! Uses `proptest` to verify that the [`BerLength`] and [`BerOid`] typed
//! struct APIs and the corresponding `enc::ber` / `dec::ber` function APIs
//! satisfy roundtrip, structural (short-form / long-form MSB), non-empty
//! output, and full-consume invariants across the entire `u32` value range
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::codecs::ber::{BerLength, BerOid};
use tinyklv::prelude::*;
// --------------------------------------------------
// external
// --------------------------------------------------
use proptest::prelude::*;

proptest! {
    #[test]
    /// Full u32-range roundtrip via the typed struct API
    fn ber_length_roundtrip_u32_range(val in 0u32..u32::MAX) {
        let encoded = BerLength::new(val as u64).encode_value();
        let decoded = BerLength::<u64>::decode_value(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val as u128, decoded.as_u128());
    }

    #[test]
    /// Short-form values (< 128) encode as a single byte equal to the value
    fn ber_length_short_form_is_single_byte(val in 0u8..128) {
        let encoded = BerLength::new(val as u64).encode_value();
        prop_assert_eq!(encoded.len(), 1);
        prop_assert_eq!(encoded[0], val);
    }

    #[test]
    /// Long-form values (>= 128) must set the MSB of the first byte
    fn ber_length_long_form_msb_set(val in 128u16..u16::MAX) {
        let encoded = BerLength::new(val as u64).encode_value();
        prop_assert!(encoded[0] & 0x80 != 0, "long-form first byte must have MSB set");
    }

    #[test]
    /// The encoded output is never empty for any value
    fn ber_length_never_empty(val in 0u32..u32::MAX) {
        let encoded = BerLength::new(val as u64).encode_value();
        prop_assert!(!encoded.is_empty());
    }

    #[test]
    /// After decode, the input slice is fully consumed (no trailing bytes)
    fn ber_length_decode_consumes_all(val in 0u32..u32::MAX) {
        let encoded = BerLength::new(val as u64).encode_value();
        let mut slice = encoded.as_slice();
        let _ = BerLength::<u64>::decode_value(&mut slice).unwrap();
        prop_assert!(slice.is_empty(), "decode must consume all encoded bytes");
    }
}

proptest! {
    #[test]
    /// Function-API roundtrip: enc::ber_length -> dec::ber_length
    ///
    /// `dec::ber_length` erases to usize, so comparison is via usize
    fn ber_length_fn_api_roundtrip(val in 0u32..u32::MAX) {
        let encoded = tinyklv::enc::ber::ber_length(val as u64);
        let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val as usize, decoded);
    }
}

proptest! {
    #[test]
    /// Full u32-range OID roundtrip via the typed struct API
    fn ber_oid_roundtrip_u32_range(val in 1u32..u32::MAX) {
        let encoded = BerOid::new(val as u64).encode_value();
        let decoded = BerOid::<u64>::decode_value(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val as u64, decoded.value());
    }

    #[test]
    /// The final byte of any OID encoding must have MSB clear (continuation bit = 0)
    fn ber_oid_final_byte_msb_clear(val in 1u32..u32::MAX) {
        let encoded = BerOid::new(val as u64).encode_value();
        let last = *encoded.last().unwrap();
        prop_assert_eq!(last & 0x80, 0, "last OID byte must have MSB clear");
    }

    #[test]
    /// All bytes except the last must have MSB set (continuation = 1) for multi-byte OIDs
    fn ber_oid_continuation_bytes_have_msb_set(val in 128u32..u32::MAX) {
        let encoded = BerOid::new(val as u64).encode_value();
        prop_assert!(encoded.len() >= 2, "value >= 128 must produce multi-byte OID");
        for &b in &encoded[..encoded.len() - 1] {
            prop_assert!(b & 0x80 != 0, "non-final OID byte must have MSB set");
        }
    }

    #[test]
    /// OID encoding is never empty for nonzero values
    fn ber_oid_encoding_never_empty(val in 1u32..u32::MAX) {
        let encoded = BerOid::new(val as u64).encode_value();
        prop_assert!(!encoded.is_empty());
    }

    #[test]
    /// After OID decode, the input slice is fully consumed
    fn ber_oid_decode_consumes_all(val in 1u32..u32::MAX) {
        let encoded = BerOid::new(val as u64).encode_value();
        let mut slice = encoded.as_slice();
        let _ = BerOid::<u64>::decode_value(&mut slice).unwrap();
        prop_assert!(slice.is_empty(), "decode must consume all encoded bytes");
    }

    #[test]
    /// Single-byte OIDs (1..127) encode as exactly one byte equal to the value
    fn ber_oid_single_byte_range(val in 1u8..128) {
        let encoded = BerOid::new(val as u64).encode_value();
        prop_assert_eq!(encoded.len(), 1);
        prop_assert_eq!(encoded[0], val);
    }
}

proptest! {
    #[test]
    /// Function-API OID roundtrip: `enc::ber_oid` followed by `dec::ber_oid` recovers the original value
    fn ber_oid_fn_api_roundtrip(val in 1u32..u32::MAX) {
        let encoded = tinyklv::enc::ber::ber_oid(val as u64);
        let decoded: u64 = tinyklv::dec::ber::ber_oid(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val as u64, decoded);
    }
}
