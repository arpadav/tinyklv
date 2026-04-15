// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA\xBB",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct SentinelPacket {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    id: u16,
    #[klv(
        key = 0x02,
        var = true,
        dec = tinyklv::dec::binary::to_string_utf8,
        enc = tinyklv::enc::string::from_string_utf8
    )]
    name: String,
}

#[test]
fn extract_finds_sentinel_and_decodes() {
    // Garbage bytes, then sentinel 0xAA 0xBB, then packet length byte, then fields
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

    let result = SentinelPacket::extract(&mut stream.as_slice()).unwrap();
    assert_eq!(result.id, 42);
    assert_eq!(result.name, "KLV");
}

#[test]
fn extract_no_sentinel_fails() {
    let data: &[u8] = &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05];
    assert!(SentinelPacket::extract(&mut &data[..]).is_err());
}

#[test]
fn decode_without_seek_works_directly() {
    let name = b"KLV";
    let mut data: Vec<u8> = vec![0x01, 0x02, 0x00, 0x42, 0x02, name.len() as u8];
    data.extend_from_slice(name);
    let result = SentinelPacket::decode(&mut data.as_slice()).unwrap();
    assert_eq!(result.id, 0x42);
    assert_eq!(result.name, "KLV");
}

#[test]
fn encode_prepends_sentinel() {
    let packet = SentinelPacket {
        id: 100,
        name: String::from("AB"),
    };
    let encoded = packet.encode();
    assert_eq!(&encoded[..2], b"\xAA\xBB", "encode() must prepend sentinel");
}

#[test]
fn extract_roundtrip() {
    let original = SentinelPacket {
        id: 999,
        name: String::from("TEST"),
    };
    let encoded = original.encode();
    let decoded = SentinelPacket::extract(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
