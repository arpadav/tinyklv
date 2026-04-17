// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::codecs::binary::dec::{
    to_string_utf16_be, to_string_utf16_le, to_string_utf8, to_string_utf8_strict,
};
// --------------------------------------------------
// strict UTF-8 decoder rejects invalid bytes
// --------------------------------------------------
#[test]
/// Tests that `to_string_utf8_strict` rejects the invalid byte pair `0xFF 0xFE`.
fn strict_utf8_0xff_0xfe_fails() {
    let mut input: &[u8] = &[0xFF, 0xFE];
    let result = to_string_utf8_strict(2)(&mut input);
    assert!(
        result.is_err(),
        "0xFF 0xFE is not valid UTF-8 - strict decoder should fail"
    );
}

#[test]
/// Tests that `to_string_utf8_strict` rejects a lone continuation byte (`0x80`) without a leading byte.
fn strict_utf8_lone_continuation_fails() {
    // 0x80 is a UTF-8 continuation byte without a leading byte
    let mut input: &[u8] = &[0x80];
    let result = to_string_utf8_strict(1)(&mut input);
    assert!(result.is_err());
}

#[test]
/// Tests that `to_string_utf8_strict` rejects a truncated multi-byte sequence (leading `0xC3` without continuation).
fn strict_utf8_truncated_multibyte_fails() {
    // First byte 0xC3 (start of 2-byte sequence) but no continuation
    let mut input: &[u8] = &[0xC3];
    let result = to_string_utf8_strict(1)(&mut input);
    assert!(result.is_err());
}

#[test]
/// Tests that `to_string_utf8_strict` rejects the overlong NUL encoding `0xC0 0x80`.
fn strict_utf8_overlong_fails() {
    // 0xC0 0x80 is an overlong encoding of NUL - invalid UTF-8
    let mut input: &[u8] = &[0xC0, 0x80];
    let result = to_string_utf8_strict(2)(&mut input);
    assert!(result.is_err());
}
// --------------------------------------------------
// lossy UTF-8 decoder replaces invalid bytes with U+FFFD
// --------------------------------------------------
#[test]
/// Tests that lossy `to_string_utf8` accepts invalid `0xFF 0xFE` by emitting U+FFFD replacement characters.
fn lossy_utf8_0xff_0xfe_ok() {
    let mut input: &[u8] = &[0xFF, 0xFE];
    let result = to_string_utf8(2)(&mut input);
    assert!(
        result.is_ok(),
        "lossy decoder should return Ok for any byte sequence"
    );
    let s = result.unwrap();
    assert!(
        s.contains('\u{FFFD}'),
        "invalid bytes should be replaced with U+FFFD"
    );
}

#[test]
/// Tests that lossy `to_string_utf8` produces a string containing U+FFFD when every input byte is invalid.
fn lossy_utf8_all_invalid_ok() {
    let mut input: &[u8] = &[0x80, 0x81, 0x82, 0xFF];
    let result = to_string_utf8(4)(&mut input);
    assert!(result.is_ok());
    // Should contain replacement characters
    let s = result.unwrap();
    assert!(s.contains('\u{FFFD}'));
}
// --------------------------------------------------
// UTF-16 odd-length errors
// --------------------------------------------------
#[test]
/// Tests that `to_string_utf16_be(3)` errors because UTF-16 requires an even byte count.
fn utf16_be_odd_length_3_fails() {
    let mut input: &[u8] = &[0x00, 0x41, 0x00];
    let result = to_string_utf16_be(3)(&mut input);
    assert!(
        result.is_err(),
        "odd byte count (3) should fail for UTF-16 BE"
    );
}

#[test]
/// Tests that `to_string_utf16_le(1)` errors because one byte is not a valid UTF-16 code unit.
fn utf16_le_odd_length_1_fails() {
    let mut input: &[u8] = &[0x41];
    let result = to_string_utf16_le(1)(&mut input);
    assert!(
        result.is_err(),
        "odd byte count (1) should fail for UTF-16 LE"
    );
}

#[test]
/// Tests that `to_string_utf16_be(5)` errors on five bytes (odd, can't form 2-byte code units).
fn utf16_be_odd_length_5_fails() {
    let mut input: &[u8] = &[0x00, 0x41, 0x00, 0x42, 0x00];
    let result = to_string_utf16_be(5)(&mut input);
    assert!(
        result.is_err(),
        "odd byte count (5) should fail for UTF-16 BE"
    );
}

#[test]
/// Tests that `to_string_utf16_le(7)` errors on seven bytes (odd, can't form 2-byte code units).
fn utf16_le_odd_length_7_fails() {
    let mut input: &[u8] = &[0x41, 0x00, 0x42, 0x00, 0x43, 0x00, 0x44];
    let result = to_string_utf16_le(7)(&mut input);
    assert!(
        result.is_err(),
        "odd byte count (7) should fail for UTF-16 LE"
    );
}
