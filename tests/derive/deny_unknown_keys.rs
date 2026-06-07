//! `deny_unknown_keys` container-attribute tests for `#[derive(Klv)]`
//!
//! Tests two structs side by side: `Strict` (with `deny_unknown_keys`) and
//! `Permissive` (without it). Verifies that a strict struct errors on any
//! unrecognized key while a permissive struct silently skips it, and that
//! multiple consecutive unknown keys are all skipped by a permissive struct
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
    deny_unknown_keys,
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Strict {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    known: u16,
}

#[test]
/// Tests that a `deny_unknown_keys` struct decodes successfully when the stream contains only known keys
fn strict_known_key_only_ok() {
    let data: &[u8] = &[0x01, 0x02, 0xAB, 0xCD];
    let result = Strict::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.known, 0xABCD);
}

#[test]
/// Verifies that `deny_unknown_keys` produces an error when an unrecognized key appears alongside valid keys
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
/// Tests that `deny_unknown_keys` errors when the stream contains only an unknown key
fn strict_only_unknown_key_fails() {
    let data: &[u8] = &[0xFF, 0x02, 0x00, 0x00];
    let result = Strict::decode_value(&mut &data[..]);
    assert!(result.is_err());
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Permissive {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    known: u16,
}

#[test]
/// Verifies that without `deny_unknown_keys` an unknown key is skipped and the known field still decodes
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
/// Tests that a permissive struct skips multiple consecutive unknown keys before decoding the known field
fn permissive_multiple_unknown_keys_skipped() {
    let data: &[u8] = &[
        0xAA, 0x02, 0x00, 0x00, // unknown key 0xAA
        0xBB, 0x01, 0x00, // unknown key 0xBB
        0x01, 0x02, 0x00, 0x07, // known key 0x01 = 7
    ];
    let result = Permissive::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.known, 7);
}
