//! book 8.2 - decode-only and encode-only (encode-only)
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*; // Klv + its traits

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        enc = encb::u8,
    ),
    len(
        enc = encb::u8_from_usize,
    ),
    allow_unimplemented_decode,
)]
/// One sensor reading
struct Sensor {
    #[klv(
        key = 0x01,
        enc = *encb::u8,
    )]
    /// Frame counter
    seq: u8,

    #[klv(
        key = 0x02,
        enc = *encb::be_u16,
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

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    // before the assert, so an edited value still prints
    println!("{sensor:?} -> {encoded:02X?}");
    assert_eq!(encoded, wire);
}
