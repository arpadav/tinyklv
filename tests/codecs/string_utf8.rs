use tinyklv::codecs::binary::dec::{to_string_utf8, to_string_utf8_strict};
use tinyklv::codecs::string::enc::from_string_utf8;

// --------------------------------------------------
// ASCII strings
// --------------------------------------------------

#[test]
fn utf8_decode_af101() {
    let mut input: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
    let result = to_string_utf8(6)(&mut input).unwrap();
    assert_eq!(result, "AF-101");
}

#[test]
fn utf8_decode_mission01() {
    let mut input: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
    let result = to_string_utf8(9)(&mut input).unwrap();
    assert_eq!(result, "MISSION01");
}

#[test]
fn utf8_encode_af101() {
    let encoded = from_string_utf8("AF-101");
    assert_eq!(encoded, vec![0x41, 0x46, 0x2D, 0x31, 0x30, 0x31]);
}

#[test]
fn utf8_ascii_roundtrip() {
    let text = "Hello, World!";
    let encoded = from_string_utf8(text);
    let decoded = to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

// --------------------------------------------------
// Unicode strings
// --------------------------------------------------

#[test]
fn utf8_unicode_roundtrip() {
    let text = "Héllo";
    let encoded = from_string_utf8(text);
    let decoded = to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
fn utf8_cjk_roundtrip() {
    let text = "你好世界";
    let encoded = from_string_utf8(text);
    let decoded = to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
fn utf8_emoji_roundtrip() {
    let text = "Hello 🌍";
    let encoded = from_string_utf8(text);
    let decoded = to_string_utf8(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

// --------------------------------------------------
// empty string
// --------------------------------------------------

#[test]
fn utf8_empty_string_decode() {
    let mut input: &[u8] = &[];
    let result = to_string_utf8(0)(&mut input).unwrap();
    assert_eq!(result, "");
}

#[test]
fn utf8_empty_string_encode() {
    let encoded = from_string_utf8("");
    assert!(encoded.is_empty());
}

// --------------------------------------------------
// lossy decoder: invalid bytes replaced with U+FFFD
// --------------------------------------------------

#[test]
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
fn utf8_lossy_mixed_valid_invalid() {
    // 'A' followed by invalid 0xFF
    let mut input: &[u8] = &[0x41, 0xFF];
    let result = to_string_utf8(2)(&mut input).unwrap();
    assert!(result.starts_with('A'));
    assert!(result.contains('\u{FFFD}'));
}

// --------------------------------------------------
// strict decoder: invalid bytes return Err
// --------------------------------------------------

#[test]
fn utf8_strict_valid_bytes_ok() {
    let mut input: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
    let result = to_string_utf8_strict(6)(&mut input);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "AF-101");
}

#[test]
fn utf8_strict_invalid_bytes_err() {
    let mut input: &[u8] = &[0xFF, 0xFE];
    let result = to_string_utf8_strict(2)(&mut input);
    assert!(
        result.is_err(),
        "strict decoder should fail on invalid UTF-8"
    );
}

#[test]
fn utf8_strict_unicode_ok() {
    let text = "Héllo";
    let bytes = text.as_bytes();
    let result = to_string_utf8_strict(bytes.len())(&mut &bytes[..]).unwrap();
    assert_eq!(result, text);
}

// --------------------------------------------------
// partial consumption: decoder only consumes `len`
// bytes
// --------------------------------------------------

#[test]
fn utf8_partial_consumption() {
    // 6 bytes of "AF-101" followed by extra bytes
    let mut input: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31, 0xAA, 0xBB];
    let result = to_string_utf8(6)(&mut input).unwrap();
    assert_eq!(result, "AF-101");
    // Remaining bytes still in input
    assert_eq!(input, &[0xAA, 0xBB]);
}
