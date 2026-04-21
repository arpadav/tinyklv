// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA\xBB",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct SentinelPacket {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = *tinyklv::enc::binary::be_u16)]
    id: u16,
    #[klv(
        key = 0x02,
        varlen = true,
        dec = tinyklv::dec::binary::to_string_utf8,
        enc = &tinyklv::enc::string::from_string_utf8
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
