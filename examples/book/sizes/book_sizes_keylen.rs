// ANCHOR: all
//! book 3.7 - framing per key
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*; // Klv + its traits

#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(
        dec = dec_header,
        enc = enc_header,
    ),
)]
/// One sensor reading
struct Sensor {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Frame counter
    seq: u8,

    #[klv(
        key = 0x82, // top bit set: a length follows
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Temperature, in 0.01 °C
    temp: u16,
}

// ANCHOR: header
/// The key's top bit: a length byte follows the key
const HAS_LEN: u8 = 0x80;

/// Reads a key, then a length byte if the key's top bit
/// is set
fn dec_header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    let key = decb::u8(input)?;
    if key & HAS_LEN == 0 {
        // no length: the field's decoder stops itself
        return Ok((key, Framing::Inline));
    }
    let len = decb::u8_as_usize(input)?;
    // the field's decoder gets the next `len` bytes
    Ok((key, Framing::Sliced(len)))
}

/// Writes a key, then the value's length if the key's top
/// bit is set
fn enc_header((key, len): (u8, usize), out: &mut Vec<u8>) {
    encb::u8(key, out);
    if key & HAS_LEN != 0 {
        encb::u8_from_usize(len, out);
    }
}
// ANCHOR_END: header

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x2A,             // seq:  key, 42, no len
        0x82, 0x02, 0x09, 0x2E, // temp: key, len, 2350
    ];

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    // before the asserts, so an edited value still prints
    println!("{sensor:?} -> {encoded:02X?}");

    let mut input = wire.as_slice();
    let decoded = Sensor::decode_value(&mut input).unwrap();
    assert_eq!(encoded, wire);
    assert_eq!(decoded, sensor);
}
// ANCHOR_END: all
