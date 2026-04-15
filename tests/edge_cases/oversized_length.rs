// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

/// Required u16 field — decode must fail when value bytes are absent/insufficient
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct RequiredU16 {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    value: u16,
}

/// Optional u32 — decode must succeed with None when value bytes are absent
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct OptionalU32 {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    value: Option<u32>,
}

/// One required + one optional field — validates partial decode followed by oversized
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct MixedFields {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    required: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    optional: Option<u32>,
}

#[test]
fn oversized_inner_length_causes_break() {
    // key=0x01, len=50 (much larger than remaining data), only 2 bytes of value.
    // The inner take(50) fails -> loop breaks -> required field never decoded -> Err.
    let data: &[u8] = &[0x01, 50, 0x00, 0x01];
    let result = RequiredU16::decode(&mut &data[..]);
    assert!(
        result.is_err(),
        "oversized length exceeding available data should fail for required field"
    );
}

#[test]
fn slightly_oversized_length_fails() {
    // key=0x01, len=3 but only 2 bytes available for the value.
    let data: &[u8] = &[0x01, 3, 0x00, 0x01];
    let result = RequiredU16::decode(&mut &data[..]);
    assert!(result.is_err());
}

#[test]
fn correct_length_succeeds() {
    let data: &[u8] = &[0x01, 2, 0xAB, 0xCD];
    let result = RequiredU16::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, 0xABCD);
}

#[test]
fn oversized_length_with_zero_value_bytes_fails_required() {
    // key=0x01, len=255, but nothing follows.
    let data: &[u8] = &[0x01, 0xFF];
    assert!(RequiredU16::decode(&mut &data[..]).is_err());
}

#[test]
fn oversized_length_with_zero_value_bytes_optional_none() {
    // Same stream but the field is optional — should decode to None.
    let data: &[u8] = &[0x01, 0xFF];
    let result = OptionalU32::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, None);
}

#[test]
fn optional_oversized_gives_none() {
    // key=0x01, len=100, only 4 value bytes follow.
    let data: &[u8] = &[0x01, 100, 0x00, 0x00, 0x00, 0x01];
    let result = OptionalU32::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, None);
}

#[test]
fn first_field_valid_second_oversized_optional_none() {
    // key=0x01 len=2 val=0x1234 (valid), key=0x02 len=200 val=… (oversized).
    // required gets decoded, optional stays None.
    let data: &[u8] = &[0x01, 0x02, 0x12, 0x34, 0x02, 200];
    let result = MixedFields::decode(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0x1234);
    assert_eq!(result.optional, None);
}

#[test]
fn both_fields_valid_succeeds() {
    let data: &[u8] = &[0x01, 0x02, 0xBE, 0xEF, 0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let result = MixedFields::decode(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0xBEEF);
    assert_eq!(result.optional, Some(0xDEADBEEF));
}

#[test]
fn be_u16_lengthed_oversized_request_fails() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03, 0x04];
    assert!(tinyklv::dec::binary::be_u16_lengthed(100)(&mut input).is_err());
}

#[test]
fn be_u32_lengthed_oversized_request_fails() {
    let mut input: &[u8] = &[0x00, 0x00, 0x00, 0x01];
    assert!(tinyklv::dec::binary::be_u32_lengthed(10)(&mut input).is_err());
}

#[test]
fn be_u64_lengthed_oversized_request_fails() {
    let mut input: &[u8] = &[0u8; 8];
    assert!(tinyklv::dec::binary::be_u64_lengthed(20)(&mut input).is_err());
}

#[test]
fn be_u16_lengthed_exact_length_succeeds() {
    let mut input: &[u8] = &[0x00, 0x01];
    let result = tinyklv::dec::binary::be_u16_lengthed(2)(&mut input).unwrap();
    assert_eq!(result, 1_u16);
}

#[test]
fn be_u16_lengthed_one_byte_zero_padded_succeeds() {
    // len=1 for u16: single byte is zero-padded on the left.
    let mut input: &[u8] = &[0xAB];
    let result = tinyklv::dec::binary::be_u16_lengthed(1)(&mut input).unwrap();
    assert_eq!(result, 0x00AB_u16);
}
