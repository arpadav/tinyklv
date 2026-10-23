//! book 2.5 - latebind (the &mut form)
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
        latebind = &mut clamp,
    )]
    /// Temperature, in 0.01 °C
    temp: u16,
}

/// The sensor reads at most 125 °C
fn clamp(temp: &mut u16) {
    *temp = (*temp).min(12_500);
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 13_000,
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x32, 0xC8, // temp: key, len, 13000
    ];

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    // before the asserts, so an edited value still prints
    println!("{sensor:?} -> {encoded:02X?}");

    let mut input = wire.as_slice();
    let decoded = Sensor::decode_value(&mut input).unwrap();
    println!("{wire:02X?} -> {decoded:?}");
    // the encoder writes 13000 as it is; the decoder clamps it
    assert_eq!(encoded, wire);
    assert_eq!(
        decoded,
        Sensor {
            seq: 42,
            temp: 12_500,
        }
    );
}
