//! book 2.3 - sigils
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
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
    )]
    /// Temperature, in 0.01 °C
    temp: u16,

    #[klv(
        key = 0x03,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8, // label, as a &str
        size(lengthed),                // dec takes the length
    )]
    /// Where the sensor is
    label: String,
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
        label: String::from("roof"),
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:   key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp:  key, len, 2350
        0x03, 0x04,             // label: key, len
        0x72, 0x6F, 0x6F, 0x66, // "roof"
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
