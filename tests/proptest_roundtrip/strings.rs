// --------------------------------------------------
// external
// --------------------------------------------------
use proptest::prelude::*;

proptest! {
    /// Strict roundtrip: Rust &str is always valid UTF-8, so enc -> strict dec
        #[test]
/// must always succeed and produce the original string
    fn utf8_strict_roundtrip(s in "\\PC{0,100}") {
        let mut encoded = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf8(&s, &mut encoded);
        let decoded = tinyklv::dec::string::to_string_utf8_strict(encoded.len())(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(s, decoded);
    }

    /// Lossy roundtrip: enc -> lossy dec always succeeds and produces the
        #[test]
/// original string (since the input is already valid UTF-8)
    fn utf8_lossy_roundtrip(s in "\\PC{0,100}") {
        let mut encoded = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf8(&s, &mut encoded);
        let decoded = tinyklv::dec::string::to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(s, decoded);
    }

        #[test]
/// Encoding preserves byte length: len(enc(s)) == s.len() for UTF-8
    fn utf8_encoded_len_equals_byte_len(s in "\\PC{0,100}") {
        let mut encoded = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf8(&s, &mut encoded);
        prop_assert_eq!(encoded.len(), s.len());
    }

        #[test]
/// Empty string roundtrip
    fn utf8_empty_roundtrip(_: ()) {
        let s = "";
        let mut encoded = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf8(s, &mut encoded);
        prop_assert!(encoded.is_empty());
        let decoded = tinyklv::dec::string::to_string_utf8_strict(0)(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(s, decoded.as_str());
    }
}

proptest! {
    /// UTF-16 LE roundtrip: Rust strings that have no surrogate pairs survive
        #[test]
/// lossless encode/decode; ASCII-range input guarantees no surrogates
    fn utf16_le_ascii_roundtrip(s in "[\\x20-\\x7e]{0,50}") {
        let mut encoded = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf16_le(&s, &mut encoded);
        // encoded.len() is always even for valid UTF-16
        prop_assert_eq!(encoded.len() % 2, 0);
        let decoded = tinyklv::dec::string::to_string_utf16_le(encoded.len())(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(s, decoded);
    }

        #[test]
/// UTF-16 LE encoding produces exactly 2 * utf16_len bytes
    fn utf16_le_byte_length(s in "[\\x20-\\x7e]{0,50}") {
        let mut encoded = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf16_le(&s, &mut encoded);
        let utf16_len: usize = s.encode_utf16().count();
        prop_assert_eq!(encoded.len(), utf16_len * 2);
    }

        #[test]
/// UTF-16 LE decoder rejects odd-length slices
    fn utf16_le_odd_length_errors(data in prop::collection::vec(0u8..=255, 1usize..=11).prop_filter(
        "must be odd length",
        |v| v.len() % 2 != 0,
    )) {
        let result = tinyklv::dec::string::to_string_utf16_le(data.len())(&mut data.as_slice());
        prop_assert!(result.is_err(), "odd-length input must be rejected by utf16_le decoder");
    }
}

proptest! {
        #[test]
/// UTF-16 BE roundtrip over ASCII-safe strings
    fn utf16_be_ascii_roundtrip(s in "[\\x20-\\x7e]{0,50}") {
        let mut encoded = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf16_be(&s, &mut encoded);
        prop_assert_eq!(encoded.len() % 2, 0);
        let decoded = tinyklv::dec::string::to_string_utf16_be(encoded.len())(&mut encoded.as_slice()).unwrap();
        prop_assert_eq!(s, decoded);
    }

        #[test]
/// UTF-16 BE and LE encodings of the same string have equal byte length
    fn utf16_be_le_same_byte_length(s in "[\\x20-\\x7e]{0,50}") {
        let mut be = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf16_be(&s, &mut be);
        let mut le = Vec::new();
        tinyklv::codecs::string::enc::from_string_utf16_le(&s, &mut le);
        prop_assert_eq!(be.len(), le.len());
    }

        #[test]
/// UTF-16 BE decoder rejects odd-length slices
    fn utf16_be_odd_length_errors(data in prop::collection::vec(0u8..=255, 1usize..=11).prop_filter(
        "must be odd length",
        |v| v.len() % 2 != 0,
    )) {
        let result = tinyklv::dec::string::to_string_utf16_be(data.len())(&mut data.as_slice());
        prop_assert!(result.is_err(), "odd-length input must be rejected by utf16_be decoder");
    }
}
