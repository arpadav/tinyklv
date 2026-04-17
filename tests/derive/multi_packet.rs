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

// Two different struct types extracted from one contiguous byte stream.

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x01\x01",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct PacketA {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    value: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x02\x02",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct PacketB {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    value: u32,
}

fn build_stream(a_val: u16, b_val: u32) -> Vec<u8> {
    let a = PacketA { value: a_val };
    let b = PacketB { value: b_val };
    let mut stream = a.encode_frame();
    stream.extend(b.encode_frame());
    stream
}

#[test]
fn extract_packet_a_from_stream() {
    let stream = build_stream(0x1234, 0xDEAD_BEEF);
    let decoded = PacketA::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(decoded.value, 0x1234);
}

#[test]
fn extract_packet_b_from_stream() {
    let stream = build_stream(0x1234, 0xDEAD_BEEF);
    let decoded = PacketB::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(decoded.value, 0xDEAD_BEEF);
}

#[test]
fn both_packets_independent() {
    let stream = build_stream(999, 123456);
    let a = PacketA::decode_frame(&mut stream.as_slice()).unwrap();
    let b = PacketB::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(a.value, 999);
    assert_eq!(b.value, 123456);
}

#[test]
fn packet_a_missing_sentinel_fails() {
    // Stream only contains PacketA; extracting PacketB should fail
    let a_only = PacketA { value: 1 }.encode_frame();
    assert!(PacketB::decode_frame(&mut a_only.as_slice()).is_err());
}
