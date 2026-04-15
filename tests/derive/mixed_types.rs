// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u8(v: &u8) -> Vec<u8> {
    tinyklv::enc::binary::u8(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
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
struct Mixed {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    byte_val: u8,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    int_val: u32,
    #[klv(
        key = 0x03,
        var = true,
        dec = tinyklv::dec::binary::to_string_utf8,
        enc = tinyklv::enc::string::from_string_utf8
    )]
    name: String,
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    optional_short: Option<u16>,
}

#[test]
fn decode_all_fields_present() {
    let name = b"KLV";
    let mut data = vec![
        0x01_u8,
        0x01,
        0xAB,
        0x02,
        0x04,
        0x00,
        0x01,
        0x02,
        0x03,
        0x03,
        name.len() as u8,
    ];
    data.extend_from_slice(name);
    data.extend_from_slice(&[0x04, 0x02, 0x12, 0x34]);
    let result = Mixed::decode(&mut data.as_slice()).unwrap();
    assert_eq!(result.byte_val, 0xAB);
    assert_eq!(result.int_val, 0x00010203);
    assert_eq!(result.name, "KLV");
    assert_eq!(result.optional_short, Some(0x1234));
}

#[test]
fn decode_optional_absent() {
    let name = b"TEST";
    let mut data = vec![
        0x01_u8,
        0x01,
        0x01,
        0x02,
        0x04,
        0xFF,
        0xFF,
        0xFF,
        0xFF,
        0x03,
        name.len() as u8,
    ];
    data.extend_from_slice(name);
    let result = Mixed::decode(&mut data.as_slice()).unwrap();
    assert_eq!(result.byte_val, 0x01);
    assert_eq!(result.int_val, u32::MAX);
    assert_eq!(result.name, "TEST");
    assert_eq!(result.optional_short, None);
}

#[test]
fn roundtrip_mixed_types() {
    let original = Mixed {
        byte_val: 0xAB,
        int_val: 0xDEAD_BEEF,
        name: String::from("MISSION"),
        optional_short: Some(0x1234),
    };
    let encoded = original.encode_value();
    let decoded = Mixed::decode(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn roundtrip_mixed_types_no_optional() {
    let original = Mixed {
        byte_val: 0,
        int_val: 0,
        name: String::new(),
        optional_short: None,
    };
    let encoded = original.encode_value();
    let decoded = Mixed::decode(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn decode_missing_required_fails() {
    // byte_val (key 0x01) absent
    let name = b"X";
    let mut data = vec![0x02_u8, 0x04, 0x00, 0x00, 0x00, 0x01, 0x03, 1];
    data.extend_from_slice(name);
    let result = Mixed::decode(&mut data.as_slice());
    assert!(result.is_err());
}

#[test]
fn decode_reversed_field_order() {
    let name = b"rev";
    let mut data = vec![0x04_u8, 0x02, 0x00, 0x07, 0x03, name.len() as u8];
    data.extend_from_slice(name);
    data.extend_from_slice(&[0x02, 0x04, 0x00, 0x00, 0x00, 0x05, 0x01, 0x01, 0x09]);
    let result = Mixed::decode(&mut data.as_slice()).unwrap();
    assert_eq!(result.byte_val, 9);
    assert_eq!(result.int_val, 5);
    assert_eq!(result.name, "rev");
    assert_eq!(result.optional_short, Some(7));
}
