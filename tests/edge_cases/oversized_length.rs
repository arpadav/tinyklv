//! Oversized declared-length handling tests
//!
//! Verifies behavior when the KLV length field claims more bytes than the
//! remaining input provides. Tests the 2-arm `Packet` contract: required-field
//! structs fail; optional-only structs succeed with `None`; required fields
//! that landed before the truncation are preserved. Also tests the `_lengthed`
//! function codecs directly when the requested length exceeds available input
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
/// Required u16 field - decode must fail when value bytes are absent/insufficient
struct RequiredU16 {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    value: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Optional u32 - decode must succeed with None when value bytes are absent
struct OptionalU32 {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    value: Option<u32>,
}

/// One required + one optional field - validates partial decode followed by oversized
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct MixedFields {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    required: u16,
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    optional: Option<u32>,
}

#[test]
/// Tests that a declared length far exceeding available bytes errors on a required field
fn oversized_inner_length_causes_break() {
    // key=0x01, len=50 (much larger than remaining data), only 2 bytes of value.
    // The inner take(50) fails -> loop breaks -> required field never decoded -> Err.
    let data: &[u8] = &[0x01, 50, 0x00, 0x01];
    let result = RequiredU16::decode_value(&mut &data[..]);
    assert!(
        result.is_err(),
        "oversized length exceeding available data should fail for required field"
    );
}

#[test]
/// Tests that a declared length just one byte over available data errors on a required field
fn slightly_oversized_length_fails() {
    // key=0x01, len=3 but only 2 bytes available for the value.
    let data: &[u8] = &[0x01, 3, 0x00, 0x01];
    let result = RequiredU16::decode_value(&mut &data[..]);
    assert!(result.is_err());
}

#[test]
/// Sanity check that an exact-length TLV decodes successfully
fn correct_length_succeeds() {
    let data: &[u8] = &[0x01, 2, 0xAB, 0xCD];
    let result = RequiredU16::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.value, 0xABCD);
}

#[test]
/// Tests that a declared length of 255 with no trailing value bytes errors for a required field
fn oversized_length_with_zero_value_bytes_fails_required() {
    // key=0x01, len=255, but nothing follows.
    let data: &[u8] = &[0x01, 0xFF];
    assert!(RequiredU16::decode_value(&mut &data[..]).is_err());
}

#[test]
/// A truncated frame with only an optional field surfaces as
/// `Packet::NeedMore` from `decode_partial` (recoverable per the
/// new 2-arm `Packet` contract). `decode_value` finalises the
/// partial; with no required fields, finalisation succeeds and the
/// optional field stays `None`. Callers wanting fail-loud must
/// implement a `Done` break condition or drive `Decoder::finish`
fn oversized_length_with_zero_value_bytes_optional_yields_none() {
    let data: &[u8] = &[0x01, 0xFF];
    let v = OptionalU32::decode_value(&mut &data[..]).expect("optional-only finalises");
    assert!(v.value.is_none(), "optional must remain None on truncation");
}

#[test]
/// Optional field with declared length overrunning input -> recoverable
/// `NeedMore` -> finalise leaves the optional `None`
fn optional_oversized_yields_none() {
    let data: &[u8] = &[0x01, 100, 0x00, 0x00, 0x00, 0x01];
    let v = OptionalU32::decode_value(&mut &data[..]).expect("optional-only finalises");
    assert!(v.value.is_none());
}

#[test]
/// Valid first field followed by a second-field overlong length:
/// recoverable truncation. The required first field landed before the
/// truncation, so finalise succeeds with the optional still `None`
fn first_field_valid_second_oversized_keeps_required() {
    let data: &[u8] = &[0x01, 0x02, 0x12, 0x34, 0x02, 200];
    let v = MixedFields::decode_value(&mut &data[..]).expect("required landed");
    assert_eq!(v.required, 0x1234);
    assert!(v.optional.is_none());
}

#[test]
/// Sanity check that a well-formed stream with correct lengths for both required and optional fields decodes successfully
fn both_fields_valid_succeeds() {
    let data: &[u8] = &[0x01, 0x02, 0xBE, 0xEF, 0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let result = MixedFields::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0xBEEF);
    assert_eq!(result.optional, Some(0xDEADBEEF));
}

#[test]
/// Tests that `be_u16_lengthed(100)` errors when only 4 bytes are available
fn be_u16_lengthed_oversized_request_fails() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03, 0x04];
    assert!(decb::be_u16_lengthed(100)(&mut input).is_err());
}

#[test]
/// Tests that `be_u32_lengthed(10)` errors when only 4 bytes are available
fn be_u32_lengthed_oversized_request_fails() {
    let mut input: &[u8] = &[0x00, 0x00, 0x00, 0x01];
    assert!(decb::be_u32_lengthed(10)(&mut input).is_err());
}

#[test]
/// Tests that `be_u64_lengthed(20)` errors when only 8 bytes are available
fn be_u64_lengthed_oversized_request_fails() {
    let mut input: &[u8] = &[0u8; 8];
    assert!(decb::be_u64_lengthed(20)(&mut input).is_err());
}

#[test]
/// Tests that `be_u16_lengthed(2)` decodes cleanly when exactly 2 bytes are available
fn be_u16_lengthed_exact_length_succeeds() {
    let mut input: &[u8] = &[0x00, 0x01];
    let result = decb::be_u16_lengthed(2)(&mut input).unwrap();
    assert_eq!(result, 1_u16);
}

#[test]
/// Tests that `be_u16_lengthed(1)` zero-pads a single-byte input on the left and returns the expected value
fn be_u16_lengthed_one_byte_zero_padded_succeeds() {
    // len=1 for u16: single byte is zero-padded on the left.
    let mut input: &[u8] = &[0xAB];
    let result = decb::be_u16_lengthed(1)(&mut input).unwrap();
    assert_eq!(result, 0x00AB_u16);
}
