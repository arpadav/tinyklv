//! Multi-packet sentinel extraction tests for `#[derive(Klv)]`
//!
//! Tests two structs with distinct sentinels (`PacketA` and `PacketB`) sharing
//! the same byte stream. Verifies that each type's `decode_frame` finds its
//! own sentinel independently, that both types can be extracted from the same
//! stream without interference, and that decoding fails when the expected
//! sentinel is absent
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x01\x01",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct PacketA {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    value: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\x02\x02",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct PacketB {
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    value: u32,
}

/// Encode a `PacketA` followed by a `PacketB` into a single byte stream
///
/// Each packet is framed with its own sentinel using `encode_frame`. The
/// resulting stream contains both frames back-to-back and is used by tests
/// that extract each type independently using its sentinel
fn build_stream(a_val: u16, b_val: u32) -> Vec<u8> {
    let a = PacketA { value: a_val };
    let b = PacketB { value: b_val };
    let mut stream = Vec::new();
    a.encode_frame(&mut stream);
    b.encode_frame(&mut stream);
    stream
}

#[test]
/// Tests that `PacketA` is extracted from a stream containing both `PacketA` and `PacketB` by matching its sentinel
fn extract_packet_a_from_stream() {
    let stream = build_stream(0x1234, 0xDEAD_BEEF);
    let decoded = PacketA::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(decoded.value, 0x1234);
}

#[test]
/// Tests that `PacketB` is extracted from the same mixed stream by locking onto its distinct sentinel
fn extract_packet_b_from_stream() {
    let stream = build_stream(0x1234, 0xDEAD_BEEF);
    let decoded = PacketB::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(decoded.value, 0xDEAD_BEEF);
}

#[test]
/// Verifies that two distinct packet types can be extracted from the same byte stream without interference
fn both_packets_independent() {
    let stream = build_stream(999, 123456);
    let a = PacketA::decode_frame(&mut stream.as_slice()).unwrap();
    let b = PacketB::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(a.value, 999);
    assert_eq!(b.value, 123456);
}

#[test]
/// Tests that extracting `PacketB` fails when the stream contains only `PacketA` bytes, since `PacketB`'s sentinel is absent
fn packet_a_missing_sentinel_fails() {
    let mut a_only = Vec::new();
    PacketA { value: 1 }.encode_frame(&mut a_only);
    assert!(PacketB::decode_frame(&mut a_only.as_slice()).is_err());
}
