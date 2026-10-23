// ANCHOR: all
//! book reference - `break_on`: on the container, a key literal or
//! a function; on a field, a function
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

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
    break_on = 0xFF,
)]
/// A key literal: decoding stops right after key FF
struct UntilKey {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: u16,
}

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
    break_on = end_item,
)]
/// A function: decoding stops after the whole end item
struct UntilEnd {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: u16,
}

// ANCHOR: end_item
/// The key of the item that ends a packet
const END: u8 = 0xFF;

/// Stops at key FF, past its length and value
fn end_item(key: &u8, input: &mut &[u8]) -> BreakType {
    if *key != END {
        return BreakType::Proceed;
    }
    let Ok(len) = decb::u8_as_usize(input) else {
        // cut short: the item reads as cut, and a stream
        // waits for more bytes
        return BreakType::Proceed;
    };
    match input.split_at_checked(len) {
        Some((_, rest)) => {
            *input = rest;
            BreakType::Done
        }
        None => BreakType::Proceed,
    }
}
// ANCHOR_END: end_item

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
)]
/// A field's function: checks temp once it decodes
struct Checked {
    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        break_on = check_temp,
    )]
    temp: u16,

    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,
}

/// The sensor reads at most 125 °C
fn check_temp(temp: &u16) -> BreakType {
    if *temp > 12_500 {
        return BreakType::Abort("temp above 125 °C");
    }
    BreakType::Proceed
}

fn main() {
    let sensor = UntilKey {
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

    // a key literal: the end item's length is left
    let mut input = wire.as_slice();
    let decoded = UntilKey::decode_value(&mut input);
    assert_eq!(decoded.unwrap(), sensor);
    assert_eq!(input, [0x00, 0x01, 0x01, 0x2B]);

    // the encoder writes no end item
    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    println!("{sensor:?} -> {encoded:02X?}");
    assert_eq!(encoded, wire[..7]);

    // a function: the whole end item is read
    let mut input = wire.as_slice();
    let decoded = UntilEnd::decode_value(&mut input).unwrap();
    assert_eq!(
        decoded,
        UntilEnd {
            seq: 42,
            temp: 2350,
        }
    );
    assert_eq!(input, [0x01, 0x01, 0x2B]);

    // a field's function: encoded last, though declared
    // first
    let checked = Checked {
        temp: 2350,
        seq: 42,
    };
    let mut encoded = Vec::new();
    checked.encode_value(&mut encoded);
    println!("{checked:?} -> {encoded:02X?}");
    assert_eq!(encoded, wire[..7]);

    // a temp of 13000 is an error
    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x32, 0xC8, // temp: key, len, 13000
    ];
    let error = Checked::decode_value(&mut wire.as_slice()).unwrap_err();
    assert_eq!(ErrorKind::of(&error), Some(ErrorKind::Aborted));
}
// ANCHOR_END: all
