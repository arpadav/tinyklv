//! `to_string_ascii` / `from_string_ascii` codec tests
//!
//! Tests the byte-level ASCII string decoder and encoder in
//! `codecs::string`: printable payloads, the 0x7F boundary byte,
//! rejection of bytes >= 0x80, rejection of multi-byte UTF-8 sequences,
//! short-input errors, zero-length fields, and a full encode/decode
//! roundtrip
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::string as decs;
use tinyklv::enc::string as encs;

#[test]
/// Tests `to_string_ascii` decodes a printable ASCII payload
fn ascii_known_value() {
    let mut input: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
    assert_eq!(
        decs::to_string_ascii(6)(&mut input),
        Ok(String::from("AF-101"))
    );
}

#[test]
/// Tests `to_string_ascii` accepts the boundary byte `0x7F` (highest 7-bit value)
fn ascii_accepts_0x7f() {
    let mut input: &[u8] = &[0x7F];
    assert_eq!(
        decs::to_string_ascii(1)(&mut input),
        Ok(String::from("\u{7f}"))
    );
}

#[test]
/// Tests `to_string_ascii` rejects the boundary byte `0x80` (first non-ASCII byte)
fn ascii_rejects_0x80() {
    let mut input: &[u8] = &[0x80];
    assert!(decs::to_string_ascii(1)(&mut input).is_err());
}

#[test]
/// Tests `to_string_ascii` rejects valid multi-byte UTF-8 (e.g. `é`), unlike the UTF-8 decoder
fn ascii_rejects_multibyte_utf8() {
    // "é" is U+00E9 -> 0xC3 0xA9 in UTF-8; both bytes are >= 0x80
    let mut input: &[u8] = &[0xC3, 0xA9];
    assert!(decs::to_string_ascii(2)(&mut input).is_err());
}

#[test]
/// Tests `to_string_ascii` rejects input shorter than the requested length
fn ascii_short_input_errors() {
    let mut input: &[u8] = &[0x41, 0x42, 0x43];
    assert!(decs::to_string_ascii(5)(&mut input).is_err());
}

#[test]
/// Tests `to_string_ascii` decodes a zero-length field as an empty string
fn ascii_empty() {
    let mut input: &[u8] = &[];
    assert_eq!(decs::to_string_ascii(0)(&mut input), Ok(String::new()));
}

#[test]
/// Tests `from_string_ascii` emits the raw ASCII bytes
fn from_ascii_encode() {
    let mut __v = Vec::new();
    encs::from_string_ascii("HELLO", &mut __v);
    assert_eq!(__v, b"HELLO".to_vec());
}

#[test]
/// Tests `to_string_ascii`/`from_string_ascii` roundtrip over a printable string
fn ascii_roundtrip() {
    let original = "MISSION01";
    let mut encoded = Vec::new();
    encs::from_string_ascii(original, &mut encoded);
    let decoded = decs::to_string_ascii(encoded.len())(&mut encoded.as_slice()).unwrap();
    assert_eq!(original, decoded);
}
