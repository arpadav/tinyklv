//! book 5.2 - keeping unknown items
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*; // Klv + its traits

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
    unknown(
        dec = dec_unknown,
        enc = &enc_unknown,
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
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    /// Temperature, in 0.01 °C
    temp: u16,

    #[klv(unknown)]
    /// Every item whose key no field declares
    extra: Vec<(u8, Vec<u8>)>,
}

/// Reads the rest of an unknown item: its length, then its value
fn dec_unknown(_key: u8) -> impl Fn(&mut &[u8]) -> tinyklv::Result<Vec<u8>> {
    |input| {
        let len = decb::u8_as_usize(input)?;
        let Some((value, rest)) = input.split_at_checked(len) else {
            // cut short: consume the rest, so a stream waits for more
            input.finish();
            return Err(tinyklv::winnow::error::ParserError::from_input(input));
        };
        *input = rest;
        Ok(value.to_vec())
    }
}

/// Writes the rest of a kept item: its length, then its value
fn enc_unknown(value: &[u8], out: &mut Vec<u8>) {
    encb::u8_from_usize(value.len(), out);
    out.extend_from_slice(value);
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
        extra: vec![(0x07, vec![0xFF])],
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x07, 0x01, 0xFF,       // key 07, no field: key, len, FF
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
    ];

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    // before the asserts, so an edited value still prints
    println!("{sensor:?} -> {encoded:02X?}");

    let mut input = wire.as_slice();
    let decoded = Sensor::decode_value(&mut input).unwrap();
    // the encoder writes the kept items after the fields
    #[rustfmt::skip]
    assert_eq!(encoded, [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
        0x07, 0x01, 0xFF,       // kept: key, len, FF
    ]);
    assert_eq!(decoded, sensor);
}
