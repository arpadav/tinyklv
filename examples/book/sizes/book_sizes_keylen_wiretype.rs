// ANCHOR: all
//! book 3.7 - framing per key (a wire type)
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
        key = 0x0A, // field 1, wire type 2
        dec = decb::u8,
        enc = *encb::u8,
    )]
    /// Frame counter
    seq: u8,

    #[klv(
        key = 0x10, // field 2, wire type 0
        dec = dec_varint,
        enc = *enc_varint,
    )]
    /// Temperature, in 0.01 °C
    temp: u16,
}

// ANCHOR: header
/// The low 3 bits of a key: its wire type
const WIRE_TYPE: u8 = 0b111;

/// Wire type 0: a varint, which stops itself
const STOPS_ITSELF: u8 = 0;

/// Wire type 2: a length byte, then that many bytes
const WITH_LEN: u8 = 2;

/// Reads a key, then the framing its wire type names
fn dec_header(input: &mut &[u8]) -> tinyklv::Result<(u8, Framing)> {
    let key = decb::u8(input)?;
    let framing = match key & WIRE_TYPE {
        STOPS_ITSELF => Framing::Inline,
        WITH_LEN => Framing::Sliced(decb::u8_as_usize(input)?),
        _ => return Err(tinyklv::winnow::error::ParserError::from_input(input)),
    };
    Ok((key, framing))
}

/// Writes a key, then the value's length if its wire type
/// has one
fn enc_header((key, len): (u8, usize), out: &mut Vec<u8>) {
    encb::u8(key, out);
    if key & WIRE_TYPE == WITH_LEN {
        encb::u8_from_usize(len, out);
    }
}
// ANCHOR_END: header

/// The bit of a varint byte that says another byte follows
const MORE: u8 = 0x80;

/// Reads a LEB128 varint, low 7 bits first. Cut short, it
/// leaves the cursor at the end; too long, it rewinds
fn dec_varint(input: &mut &[u8]) -> tinyklv::Result<u16> {
    let start = input.checkpoint();
    let mut value = 0;
    for shift in (0..u16::BITS).step_by(7) {
        let byte = decb::u8(input)?;
        let bits = u16::from(byte & !MORE);
        // bits past the top of a `u16`: too long
        if bits > u16::MAX >> shift {
            break;
        }
        value |= bits << shift;
        if byte & MORE == 0 {
            return Ok(value);
        }
    }
    // too long for a `u16`: rewind, so it reads as
    // malformed instead of cut short
    input.reset(&start);
    Err(tinyklv::winnow::error::ParserError::from_input(input))
}

/// Writes a LEB128 varint, low 7 bits first, `MORE` set
/// on every byte but the last
fn enc_varint(mut value: u16, out: &mut Vec<u8>) {
    while value >= u16::from(MORE) {
        out.push(value as u8 | MORE);
        value >>= 7;
    }
    out.push(value as u8);
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
    };

    #[rustfmt::skip]
    let wire = [
        0x0A, 0x01, 0x2A,       // seq:  key, len, 42
        0x10, 0xAE, 0x12,       // temp: key, 2350, no len
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
