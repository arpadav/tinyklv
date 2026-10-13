//! Partial decode tests - required vs optional field presence for `#[derive(Klv)]`
//!
//! Tests the `WithRequiredAndOptional` struct (two required `u16`/`u32` fields
//! and two optional `u8`/`u16` fields). Covers: all fields present, all
//! optionals absent, one optional present while the other is absent, and error
//! cases when either or both required fields are missing or when the input is
//! entirely empty
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
struct WithRequiredAndOptional {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    required_a: u16,
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    required_b: u32,
    #[klv(
        key = 0x03,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    optional_c: Option<u8>,
    #[klv(
        key = 0x04,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    optional_d: Option<u16>,
}

#[test]
/// Tests decoding when all two required and both optional fields are present
fn all_fields_present_succeeds() {
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x2A, 0x02, 0x04, 0x00, 0x01, 0x00, 0x00, 0x03, 0x01, 0x07, 0x04, 0x02,
        0x01, 0x23,
    ];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.required_a, 42);
    assert_eq!(result.required_b, 65536);
    assert_eq!(result.optional_c, Some(7));
    assert_eq!(result.optional_d, Some(0x0123));
}

#[test]
/// Verifies that both optional fields decode to `None` when absent while required fields still succeed
fn all_optionals_absent_succeeds() {
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x2A, 0x02, 0x04, 0x00, 0x01, 0x00, 0x00];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.optional_c, None);
    assert_eq!(result.optional_d, None);
}

#[test]
/// Tests that when only one of two optional fields is present, the other decodes to `None`
fn one_optional_present_other_absent() {
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x2A, 0x02, 0x04, 0x00, 0x01, 0x00, 0x00, 0x03, 0x01, 0xFF,
    ];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.optional_c, Some(0xFF));
    assert_eq!(result.optional_d, None);
}

#[test]
/// Ensures that a missing required field (`required_a`) causes decode to return `Err` even if other fields are present
fn required_a_missing_fails() {
    let data: &[u8] = &[0x02, 0x04, 0x00, 0x00, 0x00, 0x01];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]);
    assert!(
        result.is_err(),
        "missing required field `required_a` should fail"
    );
}

#[test]
/// Ensures that a missing second required field (`required_b`) causes decode to return `Err`
fn required_b_missing_fails() {
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x2A, // 0x02 absent
        0x03, 0x01, 0x07,
    ];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]);
    assert!(
        result.is_err(),
        "missing required field `required_b` should fail"
    );
}

#[test]
/// Ensures that decoding fails when both required fields are absent but optional fields are present
fn both_required_missing_fails() {
    let data: &[u8] = &[0x03, 0x01, 0x07, 0x04, 0x02, 0xFF, 0xFF];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]);
    assert!(result.is_err());
}

#[test]
/// Tests that decoding an empty stream returns `Err` because required fields cannot be satisfied
fn empty_stream_fails() {
    let data: &[u8] = &[];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]);
    assert!(result.is_err());
}
