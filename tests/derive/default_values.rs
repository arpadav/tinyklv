// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// `init` attribute: field has a compile-time default
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WithInit {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16, init = 42)]
    with_init: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = &tinyklv::enc::binary::be_u32)]
    without_init: u32,
}

#[test]
/// Tests that when a field with `init = 42` has its key present in the stream, the decoded value wins over the init default.
fn decode_with_init_key_present() {
    let data: &[u8] = &[0x01, 0x02, 0x01, 0x00, 0x02, 0x04, 0x00, 0x00, 0x00, 0xFF];
    let result = WithInit::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.with_init, 256);
    assert_eq!(result.without_init, 255);
}

#[test]
/// Verifies that a missing key for an `init`-annotated field falls back to the compile-time default value.
fn decode_with_init_key_absent_uses_default() {
    let data: &[u8] = &[0x02, 0x04, 0x00, 0x00, 0x00, 0xFF];
    let result = WithInit::decode_value(&mut &data[..]).unwrap();
    assert_eq!(
        result.with_init, 42,
        "init default should be used when key is absent"
    );
    assert_eq!(result.without_init, 255);
}

#[test]
/// Ensures a field without an `init` attribute remains required and errors if its key is absent.
fn decode_without_init_still_required() {
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x10];
    let result = WithInit::decode_value(&mut &data[..]);
    assert!(result.is_err(), "field without init is required");
}

#[test]
/// Tests encode/decode roundtrip preserving values for a struct that has an `init`-annotated field.
fn roundtrip_with_init() {
    let original = WithInit {
        with_init: 999,
        without_init: 0xDEAD_BEEF,
    };
    let encoded = original.encode_value();
    let decoded = WithInit::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// non-KLV fields use Default::default()
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
    allow_unimplemented_encode,
)]
struct WithExtraField {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = &tinyklv::enc::binary::u8)]
    klv_field: u8,
    // No #[klv] - uses Default::default()
    extra: String,
}

#[test]
/// Verifies that a struct field not annotated with `#[klv(...)]` is populated with `Default::default()` on decode.
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
/// Tests that non-KLV fields remain at their `Default` value even when the stream contains extra unknown-key bytes.
fn decode_non_klv_field_unchanged_by_stream() {
    let data: &[u8] = &[0x01, 0x01, 0xAB, 0xFF, 0x01, 0x00];
    let result = WithExtraField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.klv_field, 0xAB);
    assert_eq!(result.extra, "");
}
