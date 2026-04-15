// --------------------------------------------------
// external
// --------------------------------------------------
use proptest::prelude::*;
use tinyklv::codecs::ber::{BerLength, BerOid};
use tinyklv::prelude::*;

// --------------------------------------------------
// BerLength roundtrips
// --------------------------------------------------

proptest! {
    /// Full u32-range roundtrip via the typed struct API
    #[test]
    fn ber_length_roundtrip_u32_range(val in 0u32..u32::MAX) {
        let encoded = BerLength::new(&(val as u64)).encode_value();
        let decoded = BerLength::<u64>::decode(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val as u128, decoded.as_u128());
    }

    /// Short-form values (< 128) encode as a single byte equal to the value
    #[test]
    fn ber_length_short_form_is_single_byte(val in 0u8..128) {
        let encoded = BerLength::new(&(val as u64)).encode_value();
        prop_assert_eq!(encoded.len(), 1);
        prop_assert_eq!(encoded[0], val);
    }

    /// Long-form values (>= 128) must set the MSB of the first byte
    #[test]
    fn ber_length_long_form_msb_set(val in 128u16..u16::MAX) {
        let encoded = BerLength::new(&(val as u64)).encode_value();
        prop_assert!(encoded[0] & 0x80 != 0, "long-form first byte must have MSB set");
    }

    /// The encoded output is never empty for any value
    #[test]
    fn ber_length_never_empty(val in 0u32..u32::MAX) {
        let encoded = BerLength::new(&(val as u64)).encode_value();
        prop_assert!(!encoded.is_empty());
    }

    /// After decode, the input slice is fully consumed (no trailing bytes)
    #[test]
    fn ber_length_decode_consumes_all(val in 0u32..u32::MAX) {
        let encoded = BerLength::new(&(val as u64)).encode_value();
        let mut slice = encoded.as_slice();
        let _ = BerLength::<u64>::decode(&mut slice).unwrap();
        prop_assert!(slice.is_empty(), "decode must consume all encoded bytes");
    }
}

// --------------------------------------------------
// BerLength function-API roundtrip
// (dec::ber_length / enc::ber_length)
// --------------------------------------------------

proptest! {
    /// Function-API roundtrip: enc::ber_length -> dec::ber_length
    ///
    /// `dec::ber_length` erases to usize, so comparison is via usize
    #[test]
    fn ber_length_fn_api_roundtrip(val in 0u32..u32::MAX) {
        let encoded = tinyklv::enc::ber::ber_length(&(val as u64));
        let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val as usize, decoded);
    }
}

// --------------------------------------------------
// BerOid roundtrips
// --------------------------------------------------

proptest! {
    /// Full u32-range OID roundtrip via the typed struct API
    #[test]
    fn ber_oid_roundtrip_u32_range(val in 1u32..u32::MAX) {
        let encoded = BerOid::new(&(val as u64)).encode_value();
        let decoded = BerOid::<u64>::decode(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val as u64, decoded.value);
    }

    /// The final byte of any OID encoding must have MSB clear (continuation bit = 0)
    #[test]
    fn ber_oid_final_byte_msb_clear(val in 1u32..u32::MAX) {
        let encoded = BerOid::new(&(val as u64)).encode_value();
        let last = *encoded.last().unwrap();
        prop_assert_eq!(last & 0x80, 0, "last OID byte must have MSB clear");
    }

    /// All bytes except the last must have MSB set (continuation = 1) for multi-byte OIDs
    #[test]
    fn ber_oid_continuation_bytes_have_msb_set(val in 128u32..u32::MAX) {
        let encoded = BerOid::new(&(val as u64)).encode_value();
        prop_assert!(encoded.len() >= 2, "value >= 128 must produce multi-byte OID");
        for &b in &encoded[..encoded.len() - 1] {
            prop_assert!(b & 0x80 != 0, "non-final OID byte must have MSB set");
        }
    }

    /// OID encoding is never empty for nonzero values
    #[test]
    fn ber_oid_encoding_never_empty(val in 1u32..u32::MAX) {
        let encoded = BerOid::new(&(val as u64)).encode_value();
        prop_assert!(!encoded.is_empty());
    }

    /// After OID decode, the input slice is fully consumed
    #[test]
    fn ber_oid_decode_consumes_all(val in 1u32..u32::MAX) {
        let encoded = BerOid::new(&(val as u64)).encode_value();
        let mut slice = encoded.as_slice();
        let _ = BerOid::<u64>::decode(&mut slice).unwrap();
        prop_assert!(slice.is_empty(), "decode must consume all encoded bytes");
    }

    /// Single-byte OIDs (1..127) encode as exactly one byte equal to the value
    #[test]
    fn ber_oid_single_byte_range(val in 1u8..128) {
        let encoded = BerOid::new(&(val as u64)).encode_value();
        prop_assert_eq!(encoded.len(), 1);
        prop_assert_eq!(encoded[0], val);
    }
}

// --------------------------------------------------
// BerOid function-API roundtrip
// (dec::ber_oid / enc::ber_oid)
// --------------------------------------------------

proptest! {
    #[test]
    fn ber_oid_fn_api_roundtrip(val in 1u32..u32::MAX) {
        let encoded = tinyklv::enc::ber::ber_oid(&(val as u64));
        let decoded: u64 = tinyklv::dec::ber::ber_oid(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val as u64, decoded);
    }
}
