//! BER-keyed / BER-length `#[derive(Klv)]` integration tests
//!
//! Verifies the derive macro with `key(dec = ber_oid, enc = ber_oid)` and
//! `len(dec = ber_length, enc = ber_length)`. Covers single-byte OID keys
//! (values < 128), all-zero, all-max, and reversed-order streams; encode/decode
//! roundtrip for short-form values; long-form length encoding for a 200-byte
//! payload; missing-required-key errors; and reversed key order
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::ber as encber;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decber::ber_oid::<u64>, enc = encber::ber_oid),
    len(dec = decber::ber_length, enc = encber::ber_length),
)]
struct BerPacket {
    #[klv(
        key = 0x01_u64,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Key 0x01 (single-byte BER OID, value < 128)
    small_key_field: u8,

    #[klv(
        key = 0x02_u64,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Key 0x02 (single-byte BER OID)
    word_field: u16,

    #[klv(
        key = 0x03_u64,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    /// Key 0x03 (single-byte BER OID)
    dword_field: u32,
}

/// Hand-build a raw `BerPacket` byte sequence from individual field values
///
/// BER OID keys and BER lengths < 128 encode as a single byte equal to the
/// value, so `key=N, len=M` each contribute exactly one byte. Fields are
/// assembled in declaration order: `small_key_field` (1 byte value), then
/// `word_field` (2 bytes BE), then `dword_field` (4 bytes BE)
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
/// Tests decoding a BER-keyed/BER-length struct where every key and length fit in a single byte
fn decode_ber_single_byte_keys() {
    let data = ber_packet_bytes(0xAB, 0x1234, 0xDEAD_BEEF);
    let result = BerPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.small_key_field, 0xAB);
    assert_eq!(result.word_field, 0x1234);
    assert_eq!(result.dword_field, 0xDEAD_BEEF);
}

#[test]
/// Verifies BER-keyed decoding when every value is zero
fn decode_ber_zero_values() {
    let data = ber_packet_bytes(0x00, 0x0000, 0x0000_0000);
    let result = BerPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.small_key_field, 0);
    assert_eq!(result.word_field, 0);
    assert_eq!(result.dword_field, 0);
}

#[test]
/// Verifies BER-keyed decoding when every value is at its type maximum
fn decode_ber_max_values() {
    let data = ber_packet_bytes(u8::MAX, u16::MAX, u32::MAX);
    let result = BerPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.small_key_field, u8::MAX);
    assert_eq!(result.word_field, u16::MAX);
    assert_eq!(result.dword_field, u32::MAX);
}

#[test]
/// Tests encode/decode roundtrip for a BER-keyed struct with arbitrary non-trivial values
fn encode_ber_roundtrip() {
    let original = BerPacket {
        small_key_field: 0x7F,
        word_field: 0x0100,
        dword_field: 0x0001_0203,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
    let decoded = BerPacket::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Tests encode/decode roundtrip for a BER-keyed struct with all-zero values
fn encode_ber_roundtrip_all_zeros() {
    let original = BerPacket {
        small_key_field: 0,
        word_field: 0,
        dword_field: 0,
    };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
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
    key(dec = decber::ber_oid::<u64>, enc = encber::ber_oid),
    len(dec = decber::ber_length, enc = encber::ber_length),
)]
struct BerLargePayload {
    #[klv(
        key = 0x01_u64,
        size(var),
        dec = decs::to_string_utf8,
        enc = encs::from_string_utf8,
    )]
    payload: String,
}

#[test]
/// Tests BER long-form length encoding for a payload of 200 bytes, forcing the `0x81 0xC8` two-byte length header
fn encode_ber_large_length_roundtrip() {
    let s: String = "A".repeat(200);
    let original = BerLargePayload { payload: s };
    let mut encoded = Vec::new();
    original.encode_value(&mut encoded);
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
/// Tests that a BER-keyed struct errors when its required key is absent from the stream
fn decode_ber_missing_required_fails() {
    let data: &[u8] = &[
        0x02, 0x02, 0x00, 0x01, // wrong key - 0x01 absent
    ];
    let result = BerLargePayload::decode_value(&mut &data[..]);
    assert!(result.is_err());
}

#[test]
/// Verifies that BER-keyed fields arriving in reverse order still match correctly by key
fn decode_ber_fields_reversed_order() {
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
