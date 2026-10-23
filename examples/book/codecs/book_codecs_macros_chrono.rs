//! book 2.4 - macros (chrono)
use chrono::NaiveTime;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
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
        dec = tinyklv::as_time!(
            decs::to_string_utf8,  // reads text
            "%H:%M:%S",            // in this format
            8,                     // 8 bytes of it
        ),
        enc = enc_time,
    )]
    /// Time of day of the reading
    time: NaiveTime,
}

/// Writes the time as text, the format `as_time!` reads
fn enc_time(time: &NaiveTime, out: &mut Vec<u8>) {
    let text = time.format("%H:%M:%S").to_string();
    out.extend_from_slice(text.as_bytes());
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
        time: NaiveTime::from_hms_opt(12, 30, 0).unwrap(),
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
        0x03, 0x08,             // time: key, len
        0x31, 0x32, 0x3A, 0x33, // "12:3"
        0x30, 0x3A, 0x30, 0x30, // "0:00"
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
