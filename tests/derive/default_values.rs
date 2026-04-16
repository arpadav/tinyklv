// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u8(v: &u8) -> Vec<u8> {
    tinyklv::enc::binary::u8(*v)
}
fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

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
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16, init = 42)]
    with_init: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    without_init: u32,
}

#[test]
fn decode_with_init_key_present() {
    let data: &[u8] = &[0x01, 0x02, 0x01, 0x00, 0x02, 0x04, 0x00, 0x00, 0x00, 0xFF];
    let result = WithInit::decode(&mut &data[..]).unwrap();
    assert_eq!(result.with_init, 256);
    assert_eq!(result.without_init, 255);
}

#[test]
fn decode_with_init_key_absent_uses_default() {
    let data: &[u8] = &[0x02, 0x04, 0x00, 0x00, 0x00, 0xFF];
    let result = WithInit::decode(&mut &data[..]).unwrap();
    assert_eq!(
        result.with_init, 42,
        "init default should be used when key is absent"
    );
    assert_eq!(result.without_init, 255);
}

#[test]
fn decode_without_init_still_required() {
    // Key 0x02 is absent and has no init - should fail
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x10];
    let result = WithInit::decode(&mut &data[..]);
    assert!(result.is_err(), "field without init is required");
}

#[test]
fn roundtrip_with_init() {
    let original = WithInit {
        with_init: 999,
        without_init: 0xDEAD_BEEF,
    };
    let encoded = original.encode_value();
    let decoded = WithInit::decode(&mut &encoded[..]).unwrap();
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
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    klv_field: u8,
    // No #[klv] - uses Default::default()
    extra: String,
}

#[test]
fn decode_non_klv_field_is_default() {
    let data: &[u8] = &[0x01, 0x01, 0x07];
    let result = WithExtraField::decode(&mut &data[..]).unwrap();
    assert_eq!(result.klv_field, 7);
    assert_eq!(
        result.extra,
        String::default(),
        "non-KLV field should be Default"
    );
}

#[test]
fn decode_non_klv_field_unchanged_by_stream() {
    // Even if stream has extra bytes with unknown keys, the non-KLV field stays at default
    let data: &[u8] = &[0x01, 0x01, 0xAB, 0xFF, 0x01, 0x00];
    let result = WithExtraField::decode(&mut &data[..]).unwrap();
    assert_eq!(result.klv_field, 0xAB);
    assert_eq!(result.extra, "");
}
