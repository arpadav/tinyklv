//! Property-based roundtrip tests for ASCII numeric codecs
//!
//! Uses `proptest` to verify that the `enc::string` / `dec::string` codec
//! pairs satisfy roundtrip invariants for unsigned integers, signed integers,
//! hexadecimal values, and floats across wide value ranges.  Also verifies
//! that zero-padded decimal literals are rejected by the decoder
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::string as deca;
use tinyklv::enc::string as enca;
// --------------------------------------------------
// external
// --------------------------------------------------
use proptest::prelude::*;

proptest! {
    /// Canonical-form roundtrip for unsigned integers: decode(encode(n)) == n
    #[test]
    fn uint_canonical_roundtrip(val in any::<u64>()) {
        let encoded = enca::u64(val);
        let decoded = deca::u64(encoded.len())(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    /// Canonical-form roundtrip for signed integers, including negatives
    #[test]
    fn int_canonical_roundtrip(val in any::<i64>()) {
        let encoded = enca::i64(val);
        let decoded = deca::i64(encoded.len())(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    /// Canonical-form roundtrip for hexadecimal unsigned integers
    #[test]
    fn hex_canonical_roundtrip(val in any::<u128>()) {
        let encoded = enca::hex_u128(val);
        let decoded = deca::hex_u128(encoded.len())(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    /// Canonical-form roundtrip for floats over a moderate finite range
    #[test]
    fn float_canonical_roundtrip(val in -1.0e6_f64..1.0e6) {
        let encoded = enca::f64(val);
        let decoded = deca::f64(encoded.len())(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(val, decoded);
    }

    /// Zero-padded decimals are rejected: winnow's `dec_uint` accepts a bare `0`
    /// or a non-zero-led run, so any leading zero before more digits fails
    #[test]
    fn uint_zero_padded_rejected(val in 0u32..100_000, pad in 1usize..5) {
        let canonical = val.to_string();
        let width = canonical.len() + pad;
        let padded = format!("{val:0>width$}").into_bytes();
        let decoded = deca::u32(padded.len())(&mut padded.as_slice());
        prop_assert!(decoded.is_err());
    }
}
