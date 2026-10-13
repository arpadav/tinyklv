//! Field-level `default = <expr>` and non-KLV field tests for `#[derive(Klv)]`
//!
//! Tests the `default = <expr>` field attribute (fallback when the key is
//! absent) and the non-`#[klv]` field behavior (resolved to `Default::default()`
//! at decode time). Covers: present key overriding the default, absent key
//! using the fallback, required-field enforcement on fields without a default,
//! and non-KLV fields staying at `Default::default()` even when the stream
//! contains unknown keys
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct WithDefault {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        default = 42,
    )]
    with_default: u16,
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    without_default: u32,
}

#[test]
/// Tests that when a field with `default = 42` has its key present in the stream, the decoded value wins over the default fallback
fn decode_with_default_key_present() {
    let data: &[u8] = &[0x01, 0x02, 0x01, 0x00, 0x02, 0x04, 0x00, 0x00, 0x00, 0xFF];
    let result = WithDefault::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.with_default, 256);
    assert_eq!(result.without_default, 255);
}

#[test]
/// Verifies that a missing key for a `default`-annotated field falls back to the inline expression
fn decode_with_default_key_absent_uses_default() {
    let data: &[u8] = &[0x02, 0x04, 0x00, 0x00, 0x00, 0xFF];
    let result = WithDefault::decode_value(&mut &data[..]).unwrap();
    assert_eq!(
        result.with_default, 42,
        "`default = 42` should be used when key is absent"
    );
    assert_eq!(result.without_default, 255);
}

#[test]
/// Ensures a field without a `default` attribute remains required and errors if its key is absent
fn decode_without_default_still_required() {
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x10];
    let result = WithDefault::decode_value(&mut &data[..]);
    assert!(result.is_err(), "field without `default` is required");
}

#[test]
/// Tests encode/decode roundtrip preserving values for a struct that has a `default`-annotated field
fn roundtrip_with_default() {
    let original = WithDefault {
        with_default: 999,
        without_default: 0xDEAD_BEEF,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = WithDefault::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    allow_unimplemented_encode,
)]
struct WithExtraField {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    klv_field: u8,
    // No #[klv] - uses Default::default()
    extra: String,
}

#[test]
/// Verifies that a struct field not annotated with `#[klv(...)]` is populated with `Default::default()` on decode
fn decode_non_klv_field_is_default() {
    let data: &[u8] = &[0x01, 0x01, 0x07];
    let result = WithExtraField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.klv_field, 7);
    assert_eq!(
        result.extra,
        String::default(),
        "non-KLV field should be Default"
    );
}

#[test]
/// Tests that non-KLV fields remain at their `Default` value even when the stream contains extra unknown-key bytes
fn decode_non_klv_field_unchanged_by_stream() {
    let data: &[u8] = &[0x01, 0x01, 0xAB, 0xFF, 0x01, 0x00];
    let result = WithExtraField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.klv_field, 0xAB);
    assert_eq!(result.extra, "");
}
