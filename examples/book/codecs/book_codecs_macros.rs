//! book 2.4 - macros
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
        dec = tinyklv::scale!(
            decb::be_u16,  // reads a u16
            f32,           // as an f32
            0.01,          // times 0.01
        ),
        enc = tinyklv::scale_enc!(
            encb::be_u16,  // writes a u16
            f32,           // from an f32
            u16,           // cast to u16
            0.01,          // after dividing by 0.01
        ),
    )]
    /// Temperature, in °C
    temp: f32,
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 23.5,
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 23.5 °C
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
