//! UTF-8 string codec tests (`to_string_utf8`, `to_string_utf8_strict`, `from_string_utf8`)
//!
//! Tests the lossy and strict UTF-8 decoders and the encoder. Covers
//! ASCII decoding, multi-byte Unicode (Latin-1, CJK, emoji), empty-string
//! encode/decode, lossy acceptance of invalid bytes with U+FFFD replacement,
//! strict rejection of invalid bytes, cursor advancement (partial consumption),
//! and the difference between lossy and strict behavior on invalid sequences
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::codecs::string::dec::{to_string_utf8, to_string_utf8_strict};
use tinyklv::codecs::string::enc::from_string_utf8;

#[test]
/// Tests that `to_string_utf8(6)` decodes ASCII bytes to `"AF-101"`
fn utf8_decode_af101() {
    let mut input: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
    let result = to_string_utf8(6)(&mut input).unwrap();
    assert_eq!(result, "AF-101");
}

#[test]
/// Tests that `to_string_utf8(9)` decodes ASCII bytes to `"MISSION01"`
fn utf8_decode_mission01() {
    let mut input: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
    let result = to_string_utf8(9)(&mut input).unwrap();
    assert_eq!(result, "MISSION01");
}

#[test]
/// Tests that `from_string_utf8("AF-101")` encodes to the expected ASCII byte sequence
fn utf8_encode_af101() {
    let mut encoded = Vec::new();
    from_string_utf8("AF-101", &mut encoded);
    assert_eq!(encoded, vec![0x41, 0x46, 0x2D, 0x31, 0x30, 0x31]);
}

#[test]
/// Tests ASCII string encode/decode roundtrip
fn utf8_ascii_roundtrip() {
    let text = "Hello, World!";
    let mut encoded = Vec::new();
    from_string_utf8(text, &mut encoded);
    let decoded = to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
/// Tests UTF-8 roundtrip preserves Latin-1 extended characters (`"Héllo"`)
fn utf8_unicode_roundtrip() {
    let text = "Héllo";
    let mut encoded = Vec::new();
    from_string_utf8(text, &mut encoded);
    let decoded = to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
/// Tests UTF-8 roundtrip preserves CJK characters (`"你好世界"`)
fn utf8_cjk_roundtrip() {
    let text = "你好世界";
    let mut encoded = Vec::new();
    from_string_utf8(text, &mut encoded);
    let decoded = to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
/// Tests UTF-8 roundtrip preserves 4-byte emoji sequences
fn utf8_emoji_roundtrip() {
    let text = "Hello 🌍";
    let mut encoded = Vec::new();
    from_string_utf8(text, &mut encoded);
    let decoded = to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
/// Tests that `to_string_utf8(0)` decodes an empty input to an empty string
fn utf8_empty_string_decode() {
    let mut input: &[u8] = &[];
    let result = to_string_utf8(0)(&mut input).unwrap();
    assert_eq!(result, "");
}

#[test]
/// Tests that `from_string_utf8("")` produces an empty byte vector
fn utf8_empty_string_encode() {
    let mut encoded = Vec::new();
    from_string_utf8("", &mut encoded);
    assert!(encoded.is_empty());
}

#[test]
/// Tests that lossy `to_string_utf8` returns Ok with U+FFFD for invalid bytes `0xFF 0xFE`
fn utf8_lossy_invalid_bytes_ok() {
    // 0xFF, 0xFE are not valid UTF-8
    let mut input: &[u8] = &[0xFF, 0xFE];
    let result = to_string_utf8(2)(&mut input);
    assert!(
        result.is_ok(),
        "lossy decoder should succeed on invalid bytes"
    );
    let s = result.unwrap();
    // The replacement character U+FFFD is the expected output
    assert!(
        s.contains('\u{FFFD}'),
        "should contain replacement character for invalid bytes"
    );
}

#[test]
/// Tests that lossy `to_string_utf8` preserves valid leading ASCII and replaces following invalid bytes
fn utf8_lossy_mixed_valid_invalid() {
    // 'A' followed by invalid 0xFF
    let mut input: &[u8] = &[0x41, 0xFF];
    let result = to_string_utf8(2)(&mut input).unwrap();
    assert!(result.starts_with('A'));
    assert!(result.contains('\u{FFFD}'));
}

#[test]
/// Tests that `to_string_utf8_strict(6)` succeeds on valid ASCII bytes
fn utf8_strict_valid_bytes_ok() {
    let mut input: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
    let result = to_string_utf8_strict(6)(&mut input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "AF-101");
}

#[test]
/// Tests that `to_string_utf8_strict(2)` errors on invalid UTF-8 bytes `0xFF 0xFE`
fn utf8_strict_invalid_bytes_err() {
    let mut input: &[u8] = &[0xFF, 0xFE];
    let result = to_string_utf8_strict(2)(&mut input);
    assert!(
        result.is_err(),
        "strict decoder should fail on invalid UTF-8"
    );
}

#[test]
/// Tests that strict UTF-8 decoder accepts valid multi-byte Unicode sequences
fn utf8_strict_unicode_ok() {
    let text = "Héllo";
    let bytes = text.as_bytes();
    let result = to_string_utf8_strict(bytes.len())(&mut &bytes[..]).unwrap();
    assert_eq!(result, text);
}

#[test]
/// Tests that `to_string_utf8(6)` consumes exactly 6 bytes and leaves the rest in the input stream
fn utf8_partial_consumption() {
    // 6 bytes of "AF-101" followed by extra bytes
    let mut input: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31, 0xAA, 0xBB];
    let result = to_string_utf8(6)(&mut input).unwrap();
    assert_eq!(result, "AF-101");
    // Remaining bytes still in input
    assert_eq!(input, &[0xAA, 0xBB]);
}
