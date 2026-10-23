//! book 3.6 - fixed-width lengths
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
        size(exact = 1),
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
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    /// Where the sensor sits
    name: String,
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
        // 300 bytes: more than a 1-byte length can say
        name: "x".repeat(300),
    };

    // a debug build panics at the encode: the name does not
    // fit its 1-byte length budget
    if cfg!(debug_assertions) {
        let encode = std::panic::catch_unwind(|| {
            sensor.encode_value(&mut Vec::new());
        });
        assert!(encode.is_err());
        return;
    }

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
    ];

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    // before the assert, so an edited value still prints
    println!("name of {} bytes -> {encoded:02X?}", sensor.name.len());

    // a release build leaves name's item out
    assert_eq!(encoded, wire);
}
