// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WithRequiredAndOptional {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    required_a: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = &tinyklv::enc::binary::be_u32)]
    required_b: u32,
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_u8, enc = &tinyklv::enc::binary::u8)]
    optional_c: Option<u8>,
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    optional_d: Option<u16>,
}

#[test]
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
fn all_optionals_absent_succeeds() {
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x2A, 0x02, 0x04, 0x00, 0x01, 0x00, 0x00];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.optional_c, None);
    assert_eq!(result.optional_d, None);
}

#[test]
fn one_optional_present_other_absent() {
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x2A, 0x02, 0x04, 0x00, 0x01, 0x00, 0x00, 0x03, 0x01, 0xFF,
    ];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.optional_c, Some(0xFF));
    assert_eq!(result.optional_d, None);
}

#[test]
fn required_a_missing_fails() {
    let data: &[u8] = &[0x02, 0x04, 0x00, 0x00, 0x00, 0x01];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]);
    assert!(
        result.is_err(),
        "missing required field `required_a` should fail"
    );
}

#[test]
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
fn both_required_missing_fails() {
    let data: &[u8] = &[0x03, 0x01, 0x07, 0x04, 0x02, 0xFF, 0xFF];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]);
    assert!(result.is_err());
}

#[test]
fn empty_stream_fails() {
    let data: &[u8] = &[];
    let result = WithRequiredAndOptional::decode_value(&mut &data[..]);
    assert!(result.is_err());
}
