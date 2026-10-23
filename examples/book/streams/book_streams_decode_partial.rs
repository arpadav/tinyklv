// ANCHOR: all
//! book 7.3 - partial decoding
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
    // before the asserts, so an edited value still prints
    println!("{sensor:?} -> {encoded:02X?}");

    // ANCHOR: pieces
    // the body arrives in two pieces, cut inside temp
    let (first, second) = wire.split_at(5);

    // seq decodes; temp is cut: the input stays at its key
    let mut input = first;
    let packet = Sensor::decode_partial(&mut input).unwrap();
    let Packet::NeedMore(partial) = packet else {
        panic!("the body is not all here yet");
    };
    assert_eq!(input, [0x02, 0x02]);

    // what is left of the first piece, then the second
    let mut rest = input.to_vec();
    rest.extend_from_slice(second);
    let mut input = rest.as_slice();
    let packet = Sensor::resume_partial(&mut input, partial).unwrap();
    let decoded = match packet {
        Packet::Ready(sensor) => sensor,
        // the input ran out; you know the body is whole
        Packet::NeedMore(partial) => partial.finalize().unwrap(),
    };
    // ANCHOR_END: pieces

    assert_eq!(encoded, wire);
    assert_eq!(decoded, sensor);
}
// ANCHOR_END: all
