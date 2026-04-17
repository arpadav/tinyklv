// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

// --------------------------------------------------
// with deny_unknown_keys: unknown key in stream → Err
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    deny_unknown_keys,
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Strict {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    known: u16,
}

#[test]
/// Tests that a `deny_unknown_keys` struct decodes successfully when the stream contains only known keys.
fn strict_known_key_only_ok() {
    let data: &[u8] = &[0x01, 0x02, 0xAB, 0xCD];
    let result = Strict::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.known, 0xABCD);
}

#[test]
/// Verifies that `deny_unknown_keys` produces an error when an unrecognized key appears alongside valid keys.
fn strict_unknown_key_fails() {
    let data: &[u8] = &[
        0x01, 0x02, 0xAB, 0xCD, // known key 0x01
        0xFF, 0x01, 0x00, // unknown key 0xFF
    ];
    let result = Strict::decode_value(&mut &data[..]);
    assert!(
        result.is_err(),
        "deny_unknown_keys should fail on unknown key 0xFF"
    );
}

#[test]
/// Tests that `deny_unknown_keys` errors when the stream contains only an unknown key.
fn strict_only_unknown_key_fails() {
    let data: &[u8] = &[0xFF, 0x02, 0x00, 0x00];
    let result = Strict::decode_value(&mut &data[..]);
    assert!(result.is_err());
}

// --------------------------------------------------
// without deny_unknown_keys: unknown key is silently skipped
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Permissive {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    known: u16,
}

#[test]
/// Verifies that without `deny_unknown_keys` an unknown key is skipped and the known field still decodes.
fn permissive_unknown_key_is_skipped() {
    let data: &[u8] = &[
        0xFF, 0x01, 0x00, // unknown key 0xFF, len=1, val=0x00
        0x01, 0x02, 0xAB, 0xCD, // known key 0x01
    ];
    let result = Permissive::decode_value(&mut &data[..]).unwrap();
    assert_eq!(
        result.known, 0xABCD,
        "known field should be decoded even after unknown key"
    );
}

#[test]
/// Tests that a permissive struct skips multiple consecutive unknown keys before decoding the known field.
fn permissive_multiple_unknown_keys_skipped() {
    let data: &[u8] = &[
        0xAA, 0x02, 0x00, 0x00, // unknown key 0xAA
        0xBB, 0x01, 0x00, // unknown key 0xBB
        0x01, 0x02, 0x00, 0x07, // known key 0x01 = 7
    ];
    let result = Permissive::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.known, 7);
}
