//! book 6.2 - break function
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*; // Klv + its traits
use tinyklv::winnow::binary::length_take;

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
    break_on = check_key,
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
}

/// Stops at key 0xFF, after its length and value
fn check_key(key: &u8, input: &mut &[u8]) -> BreakType {
    if *key != 0xFF {
        return BreakType::Proceed;
    }
    match length_take(decb::u8_as_usize).parse_next(input) {
        Ok(_) => BreakType::Done,
        // cut short: a streaming decoder waits for the rest
        Err(_) => BreakType::Proceed,
    }
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
        0xFF, 0x00,             // end:  key, len 0
        0x01, 0x01, 0x2B,       // the next packet: seq 43
    ];

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    // before the asserts, so an edited value still prints
    println!("{sensor:?} -> {encoded:02X?}");

    let mut input = wire.as_slice();
    let decoded = Sensor::decode_value(&mut input).unwrap();
    // the encoder writes no end item: no field has key 0xFF
    assert_eq!(encoded, wire[..7]);
    assert_eq!(decoded, sensor);
    // check_key read the end item: only the next packet is left
    assert_eq!(input, [0x01, 0x01, 0x2B]);
}
