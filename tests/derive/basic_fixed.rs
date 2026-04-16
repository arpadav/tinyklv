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
fn enc_u64(v: &u64) -> Vec<u8> {
    tinyklv::enc::binary::be_u64(*v)
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct BasicFixed {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    byte_val: u8,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    short_val: u16,
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    int_val: u32,
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_u64, enc = enc_u64)]
    long_val: u64,
}

#[test]
fn decode_basic_fixed() {
    let data: &[u8] = &[
        0x01, 0x01, 0x42, 0x02, 0x02, 0x01, 0x02, 0x03, 0x04, 0x00, 0x01, 0x02, 0x03, 0x04, 0x08,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    ];
    let result = BasicFixed::decode(&mut &data[..]).unwrap();
    assert_eq!(result.byte_val, 0x42);
    assert_eq!(result.short_val, 0x0102);
    assert_eq!(result.int_val, 0x00010203);
    assert_eq!(result.long_val, 0x0001020304050607);
}

#[test]
fn decode_basic_fixed_all_zeros() {
    let data: &[u8] = &[
        0x01, 0x01, 0x00, 0x02, 0x02, 0x00, 0x00, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00, 0x04, 0x08,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let result = BasicFixed::decode(&mut &data[..]).unwrap();
    assert_eq!(result.byte_val, 0);
    assert_eq!(result.short_val, 0);
    assert_eq!(result.int_val, 0);
    assert_eq!(result.long_val, 0);
}

#[test]
fn decode_basic_fixed_max_values() {
    let data: &[u8] = &[
        0x01, 0x01, 0xFF, 0x02, 0x02, 0xFF, 0xFF, 0x03, 0x04, 0xFF, 0xFF, 0xFF, 0xFF, 0x04, 0x08,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];
    let result = BasicFixed::decode(&mut &data[..]).unwrap();
    assert_eq!(result.byte_val, u8::MAX);
    assert_eq!(result.short_val, u16::MAX);
    assert_eq!(result.int_val, u32::MAX);
    assert_eq!(result.long_val, u64::MAX);
}

#[test]
fn encode_value_roundtrip() {
    let packet = BasicFixed {
        byte_val: 0x42,
        short_val: 0x0102,
        int_val: 0x00010203,
        long_val: 0x0001020304050607,
    };
    let encoded = packet.encode_value();
    let decoded = BasicFixed::decode(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
fn decode_fields_reversed_order() {
    // Fields arriving in reverse order (04, 03, 02, 01) - must still decode correctly
    let data: &[u8] = &[
        0x04, 0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0x03, 0x04, 0x00, 0x00, 0x00,
        0x01, 0x02, 0x02, 0x00, 0x02, 0x01, 0x01, 0x03,
    ];
    let result = BasicFixed::decode(&mut &data[..]).unwrap();
    assert_eq!(result.byte_val, 3);
    assert_eq!(result.short_val, 2);
    assert_eq!(result.int_val, 1);
    assert_eq!(result.long_val, 0xFFFFFFFFFFFFFFFE);
}

#[test]
fn decode_missing_required_field_fails() {
    let data: &[u8] = &[0x01, 0x01, 0x42, 0x02, 0x02, 0x01, 0x02];
    let result = BasicFixed::decode(&mut &data[..]);
    assert!(result.is_err(), "missing required fields should return Err");
}

#[test]
fn decode_two_packets_back_to_back() {
    // Without a sentinel/length wrapper, decode() reads the entire buffer.
    // Duplicate keys use last-wins semantics, so decoding the full concatenated
    // stream yields p2's values (they overwrite p1's).
    let p1 = BasicFixed {
        byte_val: 1,
        short_val: 2,
        int_val: 3,
        long_val: 4,
    };
    let p2 = BasicFixed {
        byte_val: 5,
        short_val: 6,
        int_val: 7,
        long_val: 8,
    };
    let mut stream = p1.encode_value();
    stream.extend(p2.encode_value());
    let r_full = BasicFixed::decode(&mut &stream[..]).unwrap();
    // Last-wins: p2 values overwrite p1 values when reading the whole stream
    assert_eq!(r_full, p2);
    // Decoding only p2's slice yields p2
    let offset = p1.encode_value().len();
    let r2 = BasicFixed::decode(&mut &stream[offset..]).unwrap();
    assert_eq!(r2, p2);
}
