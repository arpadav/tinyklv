use tinyklv::codecs::binary::dec::{to_string_utf16_be, to_string_utf16_le};
use tinyklv::codecs::string::enc::{from_string_utf16_be, from_string_utf16_le};

// --------------------------------------------------
// UTF-16 LE
// --------------------------------------------------

#[test]
fn utf16_le_known_ab() {
    // "AB" in UTF-16 LE
    let mut input: &[u8] = &[0x41, 0x00, 0x42, 0x00];
    let result = to_string_utf16_le(4)(&mut input).unwrap();
    assert_eq!(result, "AB");
}

#[test]
fn utf16_le_encode_ab() {
    let encoded = from_string_utf16_le("AB");
    assert_eq!(encoded, vec![0x41, 0x00, 0x42, 0x00]);
}

#[test]
fn utf16_le_roundtrip_ascii() {
    let text = "Hello";
    let encoded = from_string_utf16_le(text);
    let decoded = to_string_utf16_le(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
fn utf16_le_empty_string() {
    let encoded = from_string_utf16_le("");
    assert!(encoded.is_empty());
    let decoded = to_string_utf16_le(0)(&mut [].as_slice()).unwrap();
    assert_eq!(decoded, "");
}

#[test]
fn utf16_le_odd_length_err() {
    // 3 bytes is odd - not a valid UTF-16 length
    let mut input: &[u8] = &[0x41, 0x00, 0x42];
    let result = to_string_utf16_le(3)(&mut input);
    assert!(result.is_err(), "odd byte length should fail for UTF-16 LE");
}

#[test]
fn utf16_le_emoji_roundtrip() {
    // Emoji U+1F600 encodes as a surrogate pair in UTF-16 (4 bytes)
    let text = "\u{1F600}";
    let encoded = from_string_utf16_le(text);
    assert_eq!(encoded.len(), 4, "surrogate pair should produce 4 bytes");
    let decoded = to_string_utf16_le(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
fn utf16_le_unicode_roundtrip() {
    let text = "Héllo";
    let encoded = from_string_utf16_le(text);
    assert_eq!(encoded.len() % 2, 0, "UTF-16 encoding must be even-length");
    let decoded = to_string_utf16_le(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

// --------------------------------------------------
// UTF-16 BE
// --------------------------------------------------

#[test]
fn utf16_be_known_ab() {
    // "AB" in UTF-16 BE
    let mut input: &[u8] = &[0x00, 0x41, 0x00, 0x42];
    let result = to_string_utf16_be(4)(&mut input).unwrap();
    assert_eq!(result, "AB");
}

#[test]
fn utf16_be_encode_ab() {
    let encoded = from_string_utf16_be("AB");
    assert_eq!(encoded, vec![0x00, 0x41, 0x00, 0x42]);
}

#[test]
fn utf16_be_roundtrip_ascii() {
    let text = "KLV";
    let encoded = from_string_utf16_be(text);
    let decoded = to_string_utf16_be(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

#[test]
fn utf16_be_empty_string() {
    let encoded = from_string_utf16_be("");
    assert!(encoded.is_empty());
    let decoded = to_string_utf16_be(0)(&mut [].as_slice()).unwrap();
    assert_eq!(decoded, "");
}

#[test]
fn utf16_be_odd_length_err() {
    let mut input: &[u8] = &[0x00, 0x41, 0x00];
    let result = to_string_utf16_be(3)(&mut input);
    assert!(result.is_err(), "odd byte length should fail for UTF-16 BE");
}

#[test]
fn utf16_be_unicode_roundtrip() {
    let text = "Héllo";
    let encoded = from_string_utf16_be(text);
    assert_eq!(encoded.len() % 2, 0, "UTF-16 encoding must be even-length");
    let decoded = to_string_utf16_be(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(text, decoded);
}

// --------------------------------------------------
// LE vs BE produce different bytes
// --------------------------------------------------

#[test]
fn utf16_le_vs_be_differ() {
    let text = "A";
    let le = from_string_utf16_le(text);
    let be = from_string_utf16_be(text);
    assert_eq!(le, vec![0x41, 0x00]);
    assert_eq!(be, vec![0x00, 0x41]);
    assert_ne!(le, be);
}

#[test]
fn utf16_cross_decode_differs() {
    // "AB" encoded as LE, then decoded as BE gives wrong result
    let text = "AB";
    let le_encoded = from_string_utf16_le(text);
    let as_be = to_string_utf16_be(le_encoded.len())(&mut le_encoded.as_slice()).unwrap();
    assert_ne!(
        as_be, text,
        "mismatched endian decode should produce garbage"
    );
}
