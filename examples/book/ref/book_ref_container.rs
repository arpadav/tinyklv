//! book reference - container attributes: `stream`, `sentinel`, `key`, `default`,
//! `trait_fallback`, `debug` and `allow_unimplemented_encode`
use std::fmt::Debug;

use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = "&'a [u8]",
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
)]
/// `stream`: the payload borrows its bytes from the input
struct Raw<'a> {
    #[klv(
        key = 0x03,
        dec = borrow_value,
        enc = *put_bytes,
    )]
    payload: &'a [u8],
}

/// Takes the whole value, borrowed from the input
fn borrow_value<'a>(input: &mut &'a [u8]) -> tinyklv::Result<&'a [u8]> {
    tinyklv::winnow::token::rest.parse_next(input)
}

/// Writes the bytes as they are
fn put_bytes(bytes: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(bytes);
}

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
/// `sentinel`: page 1.1's struct, framed
struct Framed {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::be_u16,
        enc = encb::be_u16,
        size(exact = 2),
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
)]
/// `key`: 2-byte keys
struct WideKey {
    #[klv(
        key = 0x0102,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,
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
    default(
        typ = String,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    ),
)]
/// `default`: every `String` field takes the codec above
struct Labeled {
    #[klv(
        key = 0x03, // dec, enc, size: the String default
    )]
    label: String,
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
    trait_fallback,
)]
/// `trait_fallback`: `temp` uses `Celsius`'s traits
struct Fallback {
    #[klv(
        key = 0x02, // dec, enc: Celsius's traits
    )]
    temp: Celsius,
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
    debug,
)]
/// `debug`: logs each item it decodes
struct Logged {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temp: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
    ),
    allow_unimplemented_encode,
)]
/// `allow_unimplemented_encode`: no encoder, so no
/// `encode_value`
struct Listener {
    #[klv(
        key = 0x01,
        dec = decb::u8,
    )]
    seq: u8,
}

/// Encodes `value`, then prints it and its bytes
fn encode(value: &(impl EncodeValue + Debug)) -> Vec<u8> {
    let mut encoded = Vec::new();
    value.encode_value(&mut encoded);
    println!("{value:?} -> {encoded:02X?}");
    encoded
}

/// Asserts `value` encodes to `wire`, and `wire` decodes
/// back to `value`
fn round_trip<T>(value: &T, wire: &[u8])
where
    T: EncodeValue + Debug + PartialEq,
    for<'a> T: DecodeValue<&'a [u8]>,
{
    assert_eq!(encode(value), wire);
    let decoded = T::decode_value(&mut &wire[..]);
    assert_eq!(&decoded.unwrap(), value);
}

fn main() {
    // stream: the payload is the input's own bytes
    let wire = [0x03, 0x04, 0x72, 0x6F, 0x6F, 0x66];
    let raw = Raw::decode_value(&mut wire.as_slice()).unwrap();
    assert_eq!(raw.payload, b"roof");
    // the same bytes in memory, not a copy
    assert_eq!(raw.payload.as_ptr(), wire[2..].as_ptr());
    assert_eq!(encode(&raw), wire);

    // sentinel: junk, then the sentinel, the body's length,
    // the body
    let framed = Framed {
        seq: 42,
        temp: 2350,
    };
    #[rustfmt::skip]
    let wire = [
        0xDE, 0xAD, 0x00, 0x00,                 // junk
        0x53, 0x45, 0x4E, 0x53, 0x4F, 0x52,     // "SENSOR"
        0x07,                                   // body length
        0x01, 0x01, 0x2A,                       // seq:  key, len, 42
        0x02, 0x02, 0x09, 0x2E,                 // temp: key, len, 2350
    ];
    let mut encoded = Vec::new();
    framed.encode_frame(&mut encoded);
    println!("{framed:?} -> {encoded:02X?}");
    assert_eq!(encoded, wire[4..]);
    let decoded = Framed::decode_frame(&mut wire.as_slice());
    assert_eq!(decoded.unwrap(), framed);

    // key: a 2-byte key, then a 1-byte length
    round_trip(&WideKey { seq: 42 }, &[0x01, 0x02, 0x01, 0x2A]);

    // default: the String default reads and writes label
    let labeled = Labeled {
        label: "roof".to_string(),
    };
    round_trip(&labeled, &[0x03, 0x04, 0x72, 0x6F, 0x6F, 0x66]);

    // trait_fallback: Celsius's traits read and write temp
    let fallback = Fallback {
        temp: Celsius(23.5),
    };
    round_trip(&fallback, &[0x02, 0x02, 0x09, 0x2E]);

    // debug: prints each key, length and value as it decodes
    let wire = [0x01, 0x01, 0x2A, 0x02, 0x02, 0x09, 0x2E];
    let logged = Logged::decode_value(&mut wire.as_slice());
    assert_eq!(
        logged.unwrap(),
        Logged {
            seq: 42,
            temp: 2350
        }
    );

    // allow_unimplemented_encode: decodes, and has no
    // encode_value
    let listener = Listener::decode_value(&mut [0x01, 0x01, 0x2A].as_slice());
    assert_eq!(listener.unwrap(), Listener { seq: 42 });
}
