//! book 6.3 - break on a field value
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
        break_on = check_temp,
    )]
    /// Temperature, in 0.01 °C
    temp: u16,
}

/// Rejects the sensor's fault value
fn check_temp(temp: &u16) -> BreakType {
    if *temp == 0xFFFF {
        BreakType::Abort("temp: the sensor faulted")
    } else {
        BreakType::Proceed
    }
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 0xFFFF,
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0xFF, 0xFF, // temp: key, len, the fault value
    ];

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    // before the asserts, so an edited value still prints
    println!("{sensor:?} -> {encoded:02X?}");

    let mut input = wire.as_slice();
    let error = Sensor::decode_value(&mut input).unwrap_err();
    println!("{error}");
    // the encoder runs no break_on: it writes the fault value
    assert_eq!(encoded, wire);
    assert!(error.to_string().ends_with("temp: the sensor faulted"));
}
