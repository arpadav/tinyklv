// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

fn ber_key_enc(v: u64) -> Vec<u8> {
    tinyklv::enc::ber::ber_oid(&v)
}
fn ber_len_enc(v: usize) -> Vec<u8> {
    tinyklv::enc::ber::ber_length(&v)
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::ber::ber_oid::<u64>, enc = ber_key_enc),
    len(dec = tinyklv::dec::ber::ber_length, enc = ber_len_enc),
)]
struct BerPacket {
    // Key 0x01 (single-byte BER OID, value < 128)
    #[klv(key = 0x01_u64, dec = tinyklv::dec::binary::be_u8, enc = &tinyklv::enc::binary::u8)]
    small_key_field: u8,
    // Key 0x02 (single-byte BER OID)
    #[klv(key = 0x02_u64, dec = tinyklv::dec::binary::be_u16, enc = &tinyklv::enc::binary::be_u16)]
    word_field: u16,
    // Key 0x03 (single-byte BER OID)
    #[klv(key = 0x03_u64, dec = tinyklv::dec::binary::be_u32, enc = &tinyklv::enc::binary::be_u32)]
    dword_field: u32,
}

// BER OID encoding for values < 128 is a single byte equal to the value.
// BER length < 128 is a single byte equal to the length.
fn ber_packet_bytes(small: u8, word: u16, dword: u32) -> Vec<u8> {
    // key=0x01, len=1, val
    let mut v1 = vec![0x01, 0x01, small];
    // key=0x02, len=2, val
    let mut v2 = vec![0x02, 0x02];
    v2.extend_from_slice(&word.to_be_bytes());
    // key=0x03, len=4, val
    let mut v3 = vec![0x03, 0x04];
    v3.extend_from_slice(&dword.to_be_bytes());
    // acc
    v1.extend_from_slice(&v2);
    v1.extend_from_slice(&v3);
    v1
}

#[test]
fn decode_ber_single_byte_keys() {
    let data = ber_packet_bytes(0xAB, 0x1234, 0xDEAD_BEEF);
    let result = BerPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.small_key_field, 0xAB);
    assert_eq!(result.word_field, 0x1234);
    assert_eq!(result.dword_field, 0xDEAD_BEEF);
}

#[test]
fn decode_ber_zero_values() {
    let data = ber_packet_bytes(0x00, 0x0000, 0x0000_0000);
    let result = BerPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.small_key_field, 0);
    assert_eq!(result.word_field, 0);
    assert_eq!(result.dword_field, 0);
}

#[test]
fn decode_ber_max_values() {
    let data = ber_packet_bytes(u8::MAX, u16::MAX, u32::MAX);
    let result = BerPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.small_key_field, u8::MAX);
    assert_eq!(result.word_field, u16::MAX);
    assert_eq!(result.dword_field, u32::MAX);
}

#[test]
fn encode_ber_roundtrip() {
    let original = BerPacket {
        small_key_field: 0x7F,
        word_field: 0x0100,
        dword_field: 0x0001_0203,
    };
    let encoded = original.encode_value();
    let decoded = BerPacket::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn encode_ber_roundtrip_all_zeros() {
    let original = BerPacket {
        small_key_field: 0,
        word_field: 0,
        dword_field: 0,
    };
    let encoded = original.encode_value();
    let decoded = BerPacket::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

// --------------------------------------------------
// BER multi-byte length (>= 128 bytes of payload)
// --------------------------------------------------
// For BER lengths >= 128, the first byte is 0x80 | num_len_bytes,
// followed by the actual length bytes

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::ber::ber_oid::<u64>, enc = ber_key_enc),
    len(dec = tinyklv::dec::ber::ber_length, enc = ber_len_enc),
)]
struct BerLargePayload {
    #[klv(
        key = 0x01_u64,
        varlen = true,
        dec = tinyklv::dec::binary::to_string_utf8,
        enc = tinyklv::enc::string::from_string_utf8,
    )]
    payload: String,
}

#[test]
fn encode_ber_large_length_roundtrip() {
    // Build a string with 200 bytes - forces a multi-byte BER length (>= 128)
    let s: String = "A".repeat(200);
    let original = BerLargePayload { payload: s };
    let encoded = original.encode_value();
    // BER length for 200: 0x81 0xC8 (long form: 1 extra byte, value 200)
    // Verify the length encoding byte is the long-form marker
    assert_eq!(
        encoded[1], 0x81,
        "expected BER long-form length marker 0x81"
    );
    assert_eq!(encoded[2], 200, "expected BER length byte 200");
    let decoded = BerLargePayload::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn decode_ber_missing_required_fails() {
    let data: &[u8] = &[
        0x02, 0x02, 0x00, 0x01, // wrong key - 0x01 absent
    ];
    let result = BerLargePayload::decode_value(&mut &data[..]);
    assert!(result.is_err());
}

#[test]
fn decode_ber_fields_reversed_order() {
    // Fields in reverse order - decoder must still match by key
    let data = {
        let mut v = vec![];
        v.push(0x03_u8);
        v.push(0x04);
        v.extend_from_slice(&0xDEAD_BEEF_u32.to_be_bytes());
        v.push(0x02);
        v.push(0x02);
        v.extend_from_slice(&0x1234_u16.to_be_bytes());
        v.push(0x01);
        v.push(0x01);
        v.push(0xAB);
        v
    };
    let result = BerPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.small_key_field, 0xAB);
    assert_eq!(result.word_field, 0x1234);
    assert_eq!(result.dword_field, 0xDEAD_BEEF);
}
