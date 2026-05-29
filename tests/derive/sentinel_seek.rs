use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA\xBB",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct SentinelPacket {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    id: u16,
    #[klv(
        key = 0x02,
        varlen = true,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8
    )]
    name: String,
}

#[test]
/// Tests that `decode_frame` skips leading garbage bytes, locks onto the sentinel `0xAA 0xBB`, and decodes the framed payload.
fn extract_finds_sentinel_and_decodes() {
    let name = b"KLV";
    let mut body: Vec<u8> = vec![
        0x01, 0x02, 0x00, 0x2A, // key=1 len=2 val=42
        0x02, 0x03, // key=2 len=3
    ];
    body.extend_from_slice(name);
    let packet_len = body.len() as u8;

    let mut stream: Vec<u8> = vec![
        0x00, 0xFF, 0x11, 0x22, // garbage
        0xAA, 0xBB, // sentinel
        packet_len,
    ];
    stream.extend_from_slice(&body);

    let result = SentinelPacket::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(result.id, 42);
    assert_eq!(result.name, "KLV");
}

#[test]
/// Verifies that `decode_frame` errors when the sentinel bytes never appear in the input.
fn extract_no_sentinel_fails() {
    let data: &[u8] = &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
    assert!(SentinelPacket::decode_frame(&mut &data[..]).is_err());
}

#[test]
/// Tests that `decode_value` parses the KLV body directly without requiring the sentinel/length prefix.
fn decode_without_seek_works_directly() {
    let name = b"KLV";
    let mut data: Vec<u8> = vec![0x01, 0x02, 0x00, 0x42, 0x02, name.len() as u8];
    data.extend_from_slice(name);
    let result = SentinelPacket::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.id, 0x42);
    assert_eq!(result.name, "KLV");
}

#[test]
/// Verifies that `encode_frame` prepends the configured sentinel bytes to the output.
fn encode_prepends_sentinel() {
    let packet = SentinelPacket {
        id: 100,
        name: String::from("AB"),
    };
    let encoded = packet.encode_frame();
    assert_eq!(&encoded[..2], b"\xAA\xBB", "encode() must prepend sentinel");
}

#[test]
/// Verifies a full `encode_frame` -> `decode_frame` roundtrip over the sentinel-framed `SentinelPacket`.
fn extract_roundtrip() {
    let original = SentinelPacket {
        id: 999,
        name: String::from("TEST"),
    };
    let encoded = original.encode_frame();
    let decoded = SentinelPacket::decode_frame(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
/// Inline short-needle seek: a stray first sentinel byte (`0xAA`) not followed by `0xBB` must be
/// skipped (the `base = at + 1` retry) and the real sentinel found at a later offset.
fn inline_seek_skips_false_first_byte() {
    let name = b"KLV";
    let mut body: Vec<u8> = vec![0x01, 0x02, 0x00, 0x2A, 0x02, 0x03];
    body.extend_from_slice(name);
    let mut stream: Vec<u8> = vec![
        0xAA,
        0x00, // false first byte: 0xAA NOT followed by 0xBB
        0xAA,
        0x99, // another false 0xAA
        0xAA,
        0xBB, // the real sentinel
        body.len() as u8,
    ];
    stream.extend_from_slice(&body);
    let result = SentinelPacket::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(result.id, 42);
    assert_eq!(result.name, "KLV");
}

#[test]
/// Inline short-needle seek: the first sentinel byte appears repeatedly but the full sentinel
/// never does, so the loop exhausts and `decode_frame` errors (no false positive).
fn inline_seek_first_byte_without_full_sentinel_errors() {
    let data: &[u8] = &[0xAA, 0x00, 0xAA, 0x11, 0xAA, 0x22, 0xAA];
    assert!(SentinelPacket::decode_frame(&mut &data[..]).is_err());
}

/// A packet with a 5-byte sentinel (> `SHORT_SENTINEL_MAX`), so it exercises the retained
/// cached-`Finder` seek path rather than the inline scan.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA\xBB\xCC\xDD\xEE",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct LongSentinelPacket {
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    id: u16,
}

#[test]
/// Finder-path (long sentinel) seek finds the sentinel after garbage and decodes the body,
/// including skipping a false `0xAA` lead byte - parity with the inline path.
fn finder_path_seek_skips_false_first_byte() {
    let body: Vec<u8> = vec![0x01, 0x02, 0x12, 0x34]; // key=1 len=2 val=0x1234
    let mut stream: Vec<u8> = vec![
        0xAA,
        0xBB,
        0x00, // false partial sentinel (0xAA 0xBB then breaks)
        0xAA,
        0xBB,
        0xCC,
        0xDD,
        0xEE, // the real 5-byte sentinel
        body.len() as u8,
    ];
    stream.extend_from_slice(&body);
    let result = LongSentinelPacket::decode_frame(&mut stream.as_slice()).unwrap();
    assert_eq!(result.id, 0x1234);
}

#[test]
/// Finder-path roundtrip, confirming the long-sentinel branch encodes + decodes consistently.
fn finder_path_roundtrip() {
    let original = LongSentinelPacket { id: 0xBEEF };
    let encoded = original.encode_frame();
    assert_eq!(&encoded[..5], b"\xAA\xBB\xCC\xDD\xEE");
    let decoded = LongSentinelPacket::decode_frame(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
