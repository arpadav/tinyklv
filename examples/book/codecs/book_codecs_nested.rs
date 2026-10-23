//! book 2.7 - nested packets
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
    )]
    /// Temperature, in 0.01 °C
    temp: u16,

    #[klv(
        key = 0x03,
        dec = Gps::decode_value,
        enc = Gps::encode_value,
    )]
    /// Where the sensor is
    gps: Gps,
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
/// A position
struct Gps {
    #[klv(
        key = 0x01,
        dec = decb::be_i32,
        enc = *encb::be_i32,
    )]
    /// Latitude, in micro-degrees
    lat: i32,

    #[klv(
        key = 0x02,
        dec = decb::be_i32,
        enc = *encb::be_i32,
    )]
    /// Longitude, in micro-degrees
    lon: i32,
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
        gps: Gps {
            lat: 51_477_200,
            lon: -126_772,
        },
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
        0x03, 0x0C,             // gps:  key, len 12
        0x01, 0x04,             //   lat: key, len
        0x03, 0x11, 0x7A, 0xD0, //   51477200
        0x02, 0x04,             //   lon: key, len
        0xFF, 0xFE, 0x10, 0xCC, //   -126772
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
