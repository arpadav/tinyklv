// ANCHOR: all
//! book reference - `keylen(..)`: a header codec, and the keys
//! no field declares
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(
        dec = dec_header,
        enc = enc_header,
    ),
)]
/// Page 1.1's struct, with one header codec
struct Sensor {
    #[klv(
        key = 0x01, // top bit clear: no length
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x82, // top bit set: a length follows
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: Option<u16>,
}

// ANCHOR: header
/// The key's top bit: a length byte follows the key
const HAS_LEN: u8 = 0x80;

/// Reads a key, then a length byte if the key's top bit
/// is set
fn dec_header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    let key = decb::u8(input)?;
    if key & HAS_LEN == 0 {
        return Ok((key, Framing::Inline));
    }
    let len = decb::u8_as_usize(input)?;
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
        temp: Some(2350),
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x2A,             // seq:  key, 42
        0x82, 0x02, 0x09, 0x2E, // temp: key, len, 2350
    ];

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    println!("{sensor:?} -> {encoded:02X?}");
    assert_eq!(encoded, wire);
    let decoded = Sensor::decode_value(&mut wire.as_slice());
    assert_eq!(decoded.unwrap(), sensor);

    // key 83 names no field, and has a length: skipped
    #[rustfmt::skip]
    let wire = [
        0x83, 0x01, 0xFF,       // no field: key, len, FF
        0x01, 0x2A,             // seq:  key, 42
        0x82, 0x02, 0x09, 0x2E, // temp: key, len, 2350
    ];
    let decoded = Sensor::decode_value(&mut wire.as_slice());
    assert_eq!(decoded.unwrap(), sensor);

    // key 03 names no field, and has no length: decoding
    // stops before it
    #[rustfmt::skip]
    let wire = [
        0x01, 0x2A,             // seq:  key, 42
        0x03, 0xFF,             // no field: key, FF
        0x82, 0x02, 0x09, 0x2E, // temp: key, len, 2350
    ];
    let mut input = wire.as_slice();
    let decoded = Sensor::decode_value(&mut input).unwrap();
    assert_eq!(
        decoded,
        Sensor {
            seq: 42,
            temp: None,
        }
    );
    assert_eq!(input, [0x03, 0xFF, 0x82, 0x02, 0x09, 0x2E]);
}
// ANCHOR_END: all
