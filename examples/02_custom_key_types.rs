#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 02 - non-`u8` key types.
//!
//! The key codec is configured once at the container level and every field
//! key is interpreted through it. Three mirrored structs carry the same two
//! logical fields but are keyed with `u8`, big-endian `u16`, and
//! little-endian `u16` respectively - demonstrating that the layout of
//! the tag is a property of the codec pair, not the field.
//!
//! Showcases:
//! * `key(dec = ..., enc = ...)` with different integer widths
//! * Typed key literals like `0x0001_u16` on the field attribute
//! * Endianness selection (`be_u16` vs `le_u16`) at the container level
//!
//! See also: book Tutorial 02.
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8,          enc = encb::u8),                // 1-byte key, most compact
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Soil-moisture reading keyed with single-byte tags
struct SoilSensorU8Key {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Node identifier on the mesh
    node_id: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    /// Moisture in parts-per-billion
    moisture_ppb: u32,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::be_u16,      enc = encb::be_u16),            // 2-byte BE key
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Same payload keyed with a big-endian 2-byte tag (wider namespace)
struct SoilSensorBeU16Key {
    #[klv(
        key = 0x0001_u16,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    node_id: u8,

    #[klv(
        key = 0x0002_u16,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    moisture_ppb: u32,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::le_u16,      enc = encb::le_u16),            // 2-byte LE key
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Same payload keyed with a little-endian 2-byte tag
struct SoilSensorLeU16Key {
    #[klv(
        key = 0x0001_u16,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    node_id: u8,

    #[klv(
        key = 0x0002_u16,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    moisture_ppb: u32,
}

fn main() {
    let node_id: u8 = 7;
    let moisture_ppb: u32 = 42_000;

    // build + encode + decode each of the three keyings
    let a = SoilSensorU8Key   { node_id, moisture_ppb };
    let b = SoilSensorBeU16Key { node_id, moisture_ppb };
    let c = SoilSensorLeU16Key { node_id, moisture_ppb };

    let mut enc_a = Vec::new(); a.encode_value(&mut enc_a);
    let mut enc_b = Vec::new(); b.encode_value(&mut enc_b);
    let mut enc_c = Vec::new(); c.encode_value(&mut enc_c);

    // decode - each uses its own container key codec
    let dec_a = SoilSensorU8Key::decode_value(
        &mut enc_a.as_slice(),
    ).unwrap();
    let dec_b = SoilSensorBeU16Key::decode_value(
        &mut enc_b.as_slice(),
    ).unwrap();
    let dec_c = SoilSensorLeU16Key::decode_value(
        &mut enc_c.as_slice(),
    ).unwrap();

    // assert - each format round-trips cleanly
    assert_eq!(dec_a, a);
    assert_eq!(dec_b, b);
    assert_eq!(dec_c, c);

    // different key widths / endianness produce distinct byte layouts
    assert_ne!(enc_a, enc_b);
    assert_ne!(enc_b, enc_c);
}
