// ANCHOR: all
//! book reference - sigils: no sigil, `*` and `&`, and an
//! `EncodeAs` impl of your own
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

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
/// One field per sigil
struct Sensor {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8, // a copy of seq
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = enc_temp, // a reference to temp
    )]
    temp: u16,

    #[klv(
        key = 0x03,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8, // label's EncodeAs view
        size(lengthed),
        latebind = Label,
    )]
    label: Label,
}

/// Writes temp as a big-endian `u16`
fn enc_temp(temp: &u16, out: &mut Vec<u8>) {
    encb::be_u16(*temp, out);
}

// ANCHOR: label
#[derive(Debug, PartialEq)]
/// Where the sensor sits
struct Label(String);

/// A `Label` is written as its text
impl EncodeAs for Label {
    type Borrowed<'a> = &'a str;

    fn encode_as(&self) -> &str {
        &self.0
    }
}
// ANCHOR_END: label

fn main() {
    let sensor = Sensor {
        seq: 42,
        temp: 2350,
        label: Label("roof".to_string()),
    };

    #[rustfmt::skip]
    let wire = [
        0x01, 0x01, 0x2A,             // seq:   key, len, 42
        0x02, 0x02, 0x09, 0x2E,       // temp:  key, len, 2350
        0x03, 0x04,                   // label: key, len
        0x72, 0x6F, 0x6F, 0x66,       // "roof"
    ];

    let mut encoded = Vec::new();
    sensor.encode_value(&mut encoded);
    println!("{sensor:?} -> {encoded:02X?}");
    assert_eq!(encoded, wire);
    let decoded = Sensor::decode_value(&mut wire.as_slice());
    assert_eq!(decoded.unwrap(), sensor);
}
// ANCHOR_END: all
