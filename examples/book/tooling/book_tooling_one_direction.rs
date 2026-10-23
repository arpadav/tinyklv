//! book 8.2 - decode-only and encode-only
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*; // Klv + its traits

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
    ),
    allow_unimplemented_encode,
)]
/// One sensor reading
struct Sensor {
    #[klv(
        key = 0x01,
        dec = decb::u8,
    )]
    /// Frame counter
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
    )]
    /// Temperature, in 0.01 °C
    temp: u16,
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
    ];

    let mut input = wire.as_slice();
    let decoded = Sensor::decode_value(&mut input).unwrap();
    // no encoder: print the bytes and what they decode to
    println!("{wire:02X?} -> {decoded:?}");
    assert_eq!(decoded, sensor);
}
