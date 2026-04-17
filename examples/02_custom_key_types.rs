#![allow(clippy::unwrap_used)]
//! Three structs showing different key-codec choices: u8, big-endian u16, and
//! little-endian u16. Swapping the key codec changes how the tag bytes are
//! interpreted on the wire but leaves the value fields identical, so the same
//! domain data can be framed three different ways. This example encodes each
//! struct, prints its raw bytes, decodes it back, and confirms the roundtrip.
//! It also shows that a u8-keyed and a be_u16-keyed packet produce different
//! byte layouts even for the same logical key value (0x01).
use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u8(v: &u8) -> Vec<u8> {
    tinyklv::enc::binary::u8(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

#[derive(Klv, Debug, PartialEq)]
/// IoT soil-moisture reading keyed with single-byte tags
#[klv(
    stream = &[u8],
    // 1-byte key - the most compact representation
    key(
        dec = tinyklv::dec::binary::be_u8,
        enc = tinyklv::enc::binary::u8,
    ),
    len(
        dec = tinyklv::dec::binary::be_u8_as_usize,
        enc = tinyklv::enc::binary::u8_from_usize,
    ),
)]
struct SoilSensorU8Key {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    node_id: u8,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    moisture_ppb: u32,
}

#[derive(Klv, Debug, PartialEq)]
/// Same sensor payload, but the tag field is 2 bytes big-endian, allowing
/// 65 535 distinct field codes - useful for extensible protocols.
#[klv(
    stream = &[u8],
    // 2-byte BE key - wider namespace
    key(
        dec = tinyklv::dec::binary::be_u16,
        enc = tinyklv::enc::binary::be_u16,
    ),
    len(
        dec = tinyklv::dec::binary::be_u8_as_usize,
        enc = tinyklv::enc::binary::u8_from_usize,
    ),
)]
struct SoilSensorBeU16Key {
    #[klv(key = 0x0001_u16, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    node_id: u8,
    #[klv(key = 0x0002_u16, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    moisture_ppb: u32,
}

/// Same again with LE u16 tags - demonstrates that the endianness of the key
/// is controlled entirely by the key(dec/enc) codec pair.
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    // 2-byte LE key
    key(
        dec = tinyklv::dec::binary::le_u16,
        enc = tinyklv::enc::binary::le_u16,
    ),
    len(
        dec = tinyklv::dec::binary::be_u8_as_usize,
        enc = tinyklv::enc::binary::u8_from_usize,
    ),
)]
struct SoilSensorLeU16Key {
    #[klv(key = 0x0001_u16, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    node_id: u8,
    #[klv(key = 0x0002_u16, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    moisture_ppb: u32,
}

fn main() {
    let node_id: u8 = 7;
    let moisture_ppb: u32 = 42_000;

    // --- u8 key ---
    let a = SoilSensorU8Key {
        node_id,
        moisture_ppb,
    };
    let enc_a = a.encode_value();
    println!("u8-key   bytes: {:02X?}", enc_a);
    let dec_a = SoilSensorU8Key::decode_value(&mut enc_a.as_slice()).unwrap();
    assert_eq!(dec_a, a);

    // --- BE u16 key ---
    let b = SoilSensorBeU16Key {
        node_id,
        moisture_ppb,
    };
    let enc_b = b.encode_value();
    println!("BE-u16-key bytes: {:02X?}", enc_b);
    let dec_b = SoilSensorBeU16Key::decode_value(&mut enc_b.as_slice()).unwrap();
    assert_eq!(dec_b, b);

    // --- LE u16 key ---
    let c = SoilSensorLeU16Key {
        node_id,
        moisture_ppb,
    };
    let enc_c = c.encode_value();
    println!("LE-u16-key bytes: {:02X?}", enc_c);
    let dec_c = SoilSensorLeU16Key::decode_value(&mut enc_c.as_slice()).unwrap();
    assert_eq!(dec_c, c);

    // The three wire formats must be distinct (different key widths/endianness)
    assert_ne!(enc_a, enc_b);
    assert_ne!(enc_b, enc_c);

    println!("SUCCESS");
}
