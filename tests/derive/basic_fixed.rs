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
struct BasicFixed {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = *tinyklv::enc::binary::u8)]
    byte_val: u8,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = *tinyklv::enc::binary::be_u16)]
    short_val: u16,
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_u32, enc = *tinyklv::enc::binary::be_u32)]
    int_val: u32,
    #[klv(key = 0x04, dec = tinyklv::dec::binary::be_u64, enc = *tinyklv::enc::binary::be_u64)]
    long_val: u64,
}

#[test]
/// Verifies that a struct of fixed-width unsigned integer fields decodes from a well-formed KLV byte sequence.
fn decode_basic_fixed() {
    let data: &[u8] = &[
        0x01, 0x01, 0x42, 0x02, 0x02, 0x01, 0x02, 0x03, 0x04, 0x00, 0x01, 0x02, 0x03, 0x04, 0x08,
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
    ];
    let result = BasicFixed::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.byte_val, 0x42);
    assert_eq!(result.short_val, 0x0102);
    assert_eq!(result.int_val, 0x00010203);
    assert_eq!(result.long_val, 0x0001020304050607);
}

#[test]
/// Ensures fixed-width integer fields decode correctly when every value is zero.
fn decode_basic_fixed_all_zeros() {
    let data: &[u8] = &[
        0x01, 0x01, 0x00, 0x02, 0x02, 0x00, 0x00, 0x03, 0x04, 0x00, 0x00, 0x00, 0x00, 0x04, 0x08,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let result = BasicFixed::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.byte_val, 0);
    assert_eq!(result.short_val, 0);
    assert_eq!(result.int_val, 0);
    assert_eq!(result.long_val, 0);
}

#[test]
/// Ensures fixed-width integer fields decode correctly when every value is at its type maximum.
fn decode_basic_fixed_max_values() {
    let data: &[u8] = &[
        0x01, 0x01, 0xFF, 0x02, 0x02, 0xFF, 0xFF, 0x03, 0x04, 0xFF, 0xFF, 0xFF, 0xFF, 0x04, 0x08,
        0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    ];
    let result = BasicFixed::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.byte_val, u8::MAX);
    assert_eq!(result.short_val, u16::MAX);
    assert_eq!(result.int_val, u32::MAX);
    assert_eq!(result.long_val, u64::MAX);
}

#[test]
/// Verifies that `encode_value` followed by `decode_value` reproduces the original fixed-field struct.
fn encode_value_roundtrip() {
    let packet = BasicFixed {
        byte_val: 0x42,
        short_val: 0x0102,
        int_val: 0x00010203,
        long_val: 0x0001020304050607,
    };
    let encoded = packet.encode_value();
    let decoded = BasicFixed::decode_value(&mut &encoded[..]).unwrap();
    assert_eq!(decoded, packet);
}

#[test]
/// Tests that fields arriving in reverse key order (04, 03, 02, 01) still populate the correct struct slots.
fn decode_fields_reversed_order() {
    let data: &[u8] = &[
        0x04, 0x08, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE, 0x03, 0x04, 0x00, 0x00, 0x00,
        0x01, 0x02, 0x02, 0x00, 0x02, 0x01, 0x01, 0x03,
    ];
    let result = BasicFixed::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.byte_val, 3);
    assert_eq!(result.short_val, 2);
    assert_eq!(result.int_val, 1);
    assert_eq!(result.long_val, 0xFFFFFFFFFFFFFFFE);
}

#[test]
/// Tests that decoding returns an error when required fields are absent from the input.
fn decode_missing_required_field_fails() {
    let data: &[u8] = &[0x01, 0x01, 0x42, 0x02, 0x02, 0x01, 0x02];
    let result = BasicFixed::decode_value(&mut &data[..]);
    assert!(result.is_err(), "missing required fields should return Err");
}

#[test]
/// Tests last-wins semantics when two back-to-back frames are decoded as a single buffer without a sentinel/length wrapper, so duplicate keys from the second packet overwrite the first.
fn decode_two_packets_back_to_back() {
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
    let r_full = BasicFixed::decode_value(&mut &stream[..]).unwrap();
    // Last-wins: p2 values overwrite p1 values when reading the whole stream
    assert_eq!(r_full, p2);
    // Decoding only p2's slice yields p2
    let offset = p1.encode_value().len();
    let r2 = BasicFixed::decode_value(&mut &stream[offset..]).unwrap();
    assert_eq!(r2, p2);
}
