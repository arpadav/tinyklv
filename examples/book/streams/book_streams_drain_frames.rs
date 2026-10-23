//! book 7.1 - draining frames
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*; // Klv + its traits

#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel = b"SENSOR",
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
}

fn main() {
    let sensors = [
        Sensor {
            seq: 42,
            temp: 2350,
        },
        Sensor {
            seq: 43,
            temp: 2351,
        },
    ];

    #[rustfmt::skip]
    let wire = [
        0xDE, 0xAD, 0x00, 0x00, // junk
        // "SENSOR"
        0x53, 0x45, 0x4E, 0x53, 0x4F, 0x52,
        0x07,                   // length: 7 bytes
        0x01, 0x01, 0x2A,       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E, // temp: key, len, 2350
        // "SENSOR"
        0x53, 0x45, 0x4E, 0x53, 0x4F, 0x52,
        0x07,                   // length: 7 bytes
        0x01, 0x01, 0x2B,       // seq:  key, len, 43
        0x02, 0x02, 0x09, 0x2F, // temp: key, len, 2351
    ];

    // each frame goes after the last
    let mut encoded = Vec::new();
    for sensor in &sensors {
        sensor.encode_frame(&mut encoded);
    }

    let mut input = wire.as_slice();
    let decoded = Sensor::drain_frames(&mut input).unwrap();
    // before the asserts, so an edited input still prints
    for sensor in &decoded {
        println!("{sensor:?}");
    }
    // the encoder writes no junk: its first frame starts at the sentinel
    assert_eq!(encoded, wire[4..]);
    assert_eq!(decoded, sensors);
    assert_eq!(input, []);
}
