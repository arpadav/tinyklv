//! book reference - field attributes: `key = oneof(..)`, `dec` and
//! `enc` as closures, `default`, `skip_if_default`, `skip_if` and
//! `latebind`
use std::fmt::Debug;

use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
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
    default(
        typ = u16,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    ),
)]
/// `key = oneof(..)`: keys 02 and 03 both fill temp
struct OneOf {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = oneof(
            0x02 => Temp::Celsius(u16),
            0x03 => Temp::Fahrenheit(u16),
        ),
    )]
    temp: Temp,
}

#[derive(Debug, PartialEq)]
/// A temperature, in one of two units
enum Temp {
    /// Key 02, in 0.01 °C
    Celsius(u16),
    /// Key 03, in 0.01 °F
    Fahrenheit(u16),
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
)]
/// `dec` and `enc` as closures: temp in whole °C
struct Closures {
    #[klv(
        key = 0x02,
        dec = (|input: &mut &[u8]| {
            decb::be_u16(input).map(|centi| centi / 100)
        }),
        enc = (|temp: &u16, out: &mut Vec<u8>| {
            encb::be_u16(temp * 100, out);
        }),
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
)]
/// `default = 2000`: a missing temp is 2000
struct Fallback {
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
        default = 2000,
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
)]
/// `skip_if_default` and `skip_if`: no item for a temp of 0
/// or an empty label
struct Skipping {
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
        skip_if_default,
    )]
    temp: u16,

    #[klv(
        key = 0x03,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
        skip_if = String::is_empty,
    )]
    label: String,
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
)]
/// `latebind = &mut clamp`: temp is at most 12500 once
/// decoded
struct Clamped {
    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        latebind = &mut clamp,
    )]
    temp: u16,
}

/// The sensor reads at most 125 °C
fn clamp(temp: &mut u16) {
    *temp = (*temp).min(12_500);
}

/// Encodes `value`, then prints it and its bytes
fn encode(value: &(impl EncodeValue + Debug)) -> Vec<u8> {
    let mut encoded = Vec::new();
    value.encode_value(&mut encoded);
    println!("{value:?} -> {encoded:02X?}");
    encoded
}

/// Decodes `wire`, which must be read whole
fn decode<T>(wire: &[u8]) -> T
where
    T: for<'a> DecodeValue<&'a [u8]>,
{
    let mut input = wire;
    let decoded = T::decode_value(&mut input).unwrap();
    assert_eq!(input, []);
    decoded
}

fn main() {
    // oneof: key 03 holds the Fahrenheit variant
    let one_of = OneOf {
        seq: 42,
        temp: Temp::Fahrenheit(7430),
    };
    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x03, 0x02, 0x1D, 0x06, // temp: key 03, len, 7430
    ];
    assert_eq!(encode(&one_of), wire);
    assert_eq!(decode::<OneOf>(&wire), one_of);

    // closures: 23 °C is written as 2300
    let closures = Closures { temp: 23 };
    let wire = [0x02, 0x02, 0x08, 0xFC];
    assert_eq!(encode(&closures), wire);
    assert_eq!(decode::<Closures>(&wire), closures);

    // default: temp is missing, so it is 2000; the encoder
    // still writes it
    let fallback = Fallback {
        seq: 42,
        temp: 2000,
    };
    assert_eq!(decode::<Fallback>(&[0x01, 0x01, 0x2A]), fallback);
    #[rustfmt::skip]
    assert_eq!(encode(&fallback), [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x07, 0xD0, // temp: key, len, 2000
    ]);

    // skip_if_default, skip_if: only seq is written, and a
    // missing item decodes to its default
    let skipping = Skipping {
        seq: 42,
        temp: 0,
        label: String::new(),
    };
    let wire = [0x01, 0x01, 0x2A];
    assert_eq!(encode(&skipping), wire);
    assert_eq!(decode::<Skipping>(&wire), skipping);

    // latebind: 13000 decodes as 12500; the encoder writes
    // the field as it is
    let wire = [0x02, 0x02, 0x32, 0xC8];
    assert_eq!(decode::<Clamped>(&wire), Clamped { temp: 12_500 });
    assert_eq!(encode(&Clamped { temp: 13_000 }), wire);
}
