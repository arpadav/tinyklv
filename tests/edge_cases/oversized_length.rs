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

/// Required u16 field - decode must fail when value bytes are absent/insufficient
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

/// Optional u32 - decode must succeed with None when value bytes are absent
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

/// One required + one optional field - validates partial decode followed by oversized
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
    let result = RequiredU16::decode_value(&mut &data[..]);
    assert!(
        result.is_err(),
        "oversized length exceeding available data should fail for required field"
    );
}

#[test]
fn slightly_oversized_length_fails() {
    // key=0x01, len=3 but only 2 bytes available for the value.
    let data: &[u8] = &[0x01, 3, 0x00, 0x01];
    let result = RequiredU16::decode_value(&mut &data[..]);
    assert!(result.is_err());
}

#[test]
fn correct_length_succeeds() {
    let data: &[u8] = &[0x01, 2, 0xAB, 0xCD];
    let result = RequiredU16::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.value, 0xABCD);
}

#[test]
fn oversized_length_with_zero_value_bytes_fails_required() {
    // key=0x01, len=255, but nothing follows.
    let data: &[u8] = &[0x01, 0xFF];
    assert!(RequiredU16::decode_value(&mut &data[..]).is_err());
}

#[test]
fn oversized_length_with_zero_value_bytes_optional_fails() {
    // A declared length that overruns the remaining input is a truncated frame,
    // not "field absent". Even for optional fields, decode must fail loudly so
    // the caller can distinguish malformed input from legitimate absence.
    let data: &[u8] = &[0x01, 0xFF];
    let err = OptionalU32::decode_value(&mut &data[..]).expect_err("declared len overruns input");
    assert!(format!("{err:?}").contains("truncated"));
}

#[test]
fn optional_oversized_fails() {
    let data: &[u8] = &[0x01, 100, 0x00, 0x00, 0x00, 0x01];
    let err = OptionalU32::decode_value(&mut &data[..]).expect_err("declared len overruns input");
    assert!(format!("{err:?}").contains("truncated"));
}

#[test]
fn first_field_valid_second_oversized_fails() {
    let data: &[u8] = &[0x01, 0x02, 0x12, 0x34, 0x02, 200];
    let err = MixedFields::decode_value(&mut &data[..])
        .expect_err("second field declared len overruns input");
    assert!(format!("{err:?}").contains("truncated"));
}

#[test]
fn both_fields_valid_succeeds() {
    let data: &[u8] = &[0x01, 0x02, 0xBE, 0xEF, 0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let result = MixedFields::decode_value(&mut &data[..]).unwrap();
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
