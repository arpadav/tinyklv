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

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct WithOptionals {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    required: u8,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    opt_num: Option<u16>,
    #[klv(
        key = 0x03,
        varlen = true,
        dec = tinyklv::dec::binary::to_string_utf8,
        enc = tinyklv::enc::string::from_string_utf8
    )]
    opt_str: Option<String>,
}

#[test]
fn decode_all_fields_present() {
    let data: &[u8] = &[
        0x01, 0x01, 0xAB, 0x02, 0x02, 0x12, 0x34, 0x03, 0x03, b'K', b'L', b'V',
    ];
    let result = WithOptionals::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0xAB);
    assert_eq!(result.opt_num, Some(0x1234));
    assert_eq!(result.opt_str, Some(String::from("KLV")));
}

#[test]
fn decode_optional_num_missing() {
    let data: &[u8] = &[0x01, 0x01, 0x07, 0x03, 0x05, b'H', b'e', b'l', b'l', b'o'];
    let result = WithOptionals::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0x07);
    assert_eq!(result.opt_num, None);
    assert_eq!(result.opt_str, Some(String::from("Hello")));
}

#[test]
fn decode_optional_str_missing() {
    let data: &[u8] = &[0x01, 0x01, 0x55, 0x02, 0x02, 0xFF, 0xFE];
    let result = WithOptionals::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0x55);
    assert_eq!(result.opt_num, Some(0xFFFE));
    assert_eq!(result.opt_str, None);
}

#[test]
fn decode_both_optionals_missing() {
    let data: &[u8] = &[0x01, 0x01, 0x42];
    let result = WithOptionals::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0x42);
    assert_eq!(result.opt_num, None);
    assert_eq!(result.opt_str, None);
}

#[test]
fn decode_required_missing_returns_err() {
    let data: &[u8] = &[0x02, 0x02, 0x00, 0x01, 0x03, 0x02, b'h', b'i'];
    let result = WithOptionals::decode_value(&mut &data[..]);
    assert!(result.is_err(), "missing required field must return Err");
}

#[test]
fn encode_all_fields_roundtrip() {
    let original = WithOptionals {
        required: 0xDE,
        opt_num: Some(0xBEEF),
        opt_str: Some(String::from("test")),
    };
    let encoded = original.encode_value();
    let decoded = WithOptionals::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn encode_none_optionals_roundtrip() {
    let original = WithOptionals {
        required: 0x01,
        opt_num: None,
        opt_str: None,
    };
    let encoded = original.encode_value();
    let decoded = WithOptionals::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn decode_reversed_field_order() {
    let data: &[u8] = &[
        0x03, 0x04, b'r', b'u', b's', b't', 0x02, 0x02, 0x00, 0x64, 0x01, 0x01, 0x10,
    ];
    let result = WithOptionals::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0x10);
    assert_eq!(result.opt_num, Some(100));
    assert_eq!(result.opt_str, Some(String::from("rust")));
}

// The macro uses `.ok().or(#name)` which keeps the first successful parse.
#[test]
fn decode_duplicate_required_field_last_wins() {
    // key=0x01 len=1 val=0x01, then key=0x01 len=1 val=0x02 - last-wins semantics
    let data: &[u8] = &[0x01, 0x01, 0x01, 0x01, 0x01, 0x02];
    let result = WithOptionals::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.required, 0x02);
}

#[test]
fn decode_empty_input_returns_err() {
    let result = WithOptionals::decode_value(&mut [].as_slice());
    assert!(result.is_err());
}
