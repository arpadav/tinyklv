//! book 2.2 - codec traits
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
        dec = Celsius::decode_value,
        enc = Celsius::encode_value,
    )]
    /// Temperature
    temp: Celsius,
}

#[derive(Debug, PartialEq)]
/// A temperature, in °C
struct Celsius(f32);

/// Reads 0.01 °C as a big-endian `u16`
impl DecodeValue<&[u8]> for Celsius {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let centi = decb::be_u16(input)?;
        Ok(Celsius(f32::from(centi) / 100.0))
    }
}

/// Writes 0.01 °C as a big-endian `u16`
impl EncodeValue for Celsius {
    fn encode_value(&self, out: &mut Vec<u8>) {
        encb::be_u16((self.0 * 100.0).round() as u16, out);
    }
}

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: Celsius(23.5),
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
