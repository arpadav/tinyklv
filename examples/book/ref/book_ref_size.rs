//! book reference - `size(..)` and `len(..)`: every byte
//! representation shown
use std::fmt::Debug;

use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::ber as encber;
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
        size(exact = 1),
    ),
)]
struct Plain {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    n: u16,
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
        size(exact = 1),
    ),
)]
struct Exact {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        size(exact = 2),
    )]
    n: u16,
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
        size(exact = 1),
    ),
)]
struct Lengthed {
    #[klv(
        key = 0x01,
        dec = decb::be_u16_lengthed,
        enc = *encb::be_u16,
        size(lengthed),
    )]
    n: u16,
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
        size(exact = 1),
    ),
)]
struct ExactLengthed {
    #[klv(
        key = 0x01,
        dec = decb::be_u16_lengthed,
        enc = *encb::be_u16,
        size(exact = 2, lengthed),
    )]
    n: u16,
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
        size(exact = 1),
    ),
)]
struct Implicit {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        size(implicit),
    )]
    n: u16,
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
        size(exact = 1),
    ),
)]
struct ImplicitExact {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        size(implicit, exact = 2),
    )]
    n: u16,
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
        size(exact = 1),
    ),
)]
struct ImplicitExactLengthed {
    #[klv(
        key = 0x01,
        dec = decb::be_u16_lengthed,
        enc = *encb::be_u16,
        size(implicit, exact = 2, lengthed),
    )]
    n: u16,
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
        size(exact = 1),
    ),
    default(
        typ = u16,
        dec = decb::be_u16,
        enc = *encb::be_u16,
        size(exact = 2),
    ),
)]
struct ReserveOverDefault {
    #[klv(
        key = 0x01,
        size(reserve = 64), // in place of the default's exact = 2
    )]
    n: u16,
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
        size(exact = 1),
    ),
)]
struct Note {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    seq: u8,

    #[klv(
        key = 0x02,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    text: String,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decber::ber_length,
        enc = encber::ber_length,
        size(exact = 1),
    ),
)]
/// A BER length under `size(exact = 1)`
struct BerNote {
    #[klv(
        key = 0x01,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    text: String,
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
/// Page 3.2's struct: a length of no `size(..)`
struct Named {
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

    #[klv(
        key = 0x03,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    name: String,
}

/// Encodes `value`, then prints it and its bytes
fn encode(value: &(impl EncodeValue + Debug)) -> Vec<u8> {
    let mut encoded = Vec::new();
    value.encode_value(&mut encoded);
    println!("{value:?} -> {encoded:02X?}");
    encoded
}

/// Decodes `wire`, then prints it and the result, an error
/// as its kind
fn decode<T>(wire: &[u8]) -> Result<T, Option<ErrorKind>>
where
    T: for<'a> DecodeValue<&'a [u8]> + Debug,
{
    let decoded = T::decode_value(&mut &wire[..]);
    let decoded = decoded.map_err(|e| ErrorKind::of(&e));
    println!("{wire:02X?} -> {decoded:?}");
    decoded
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
    let n = 0x0102;
    round_trip(&Plain { n }, &[0x01, 0x02, 0x01, 0x02]);
    round_trip(&Exact { n }, &[0x01, 0x02, 0x01, 0x02]);
    round_trip(&Lengthed { n }, &[0x01, 0x02, 0x01, 0x02]);
    round_trip(&ExactLengthed { n }, &[0x01, 0x02, 0x01, 0x02]);
    round_trip(&Implicit { n }, &[0x01, 0x01, 0x02]);
    round_trip(&ImplicitExact { n }, &[0x01, 0x01, 0x02]);
    round_trip(&ImplicitExactLengthed { n }, &[0x01, 0x01, 0x02]);

    // a length of 3: `Exact` rejects it,
    // `ReserveOverDefault` does not
    assert_eq!(
        decode::<Exact>(&[0x01, 0x03, 0x01, 0x02, 0x00]),
        Err(Some(ErrorKind::WrongLength)),
    );
    assert_eq!(
        decode(&[0x01, 0x03, 0x01, 0x02, 0x00]),
        Ok(ReserveOverDefault { n }),
    );

    // a 300-byte `text` is too long for a 1-byte length: a
    // debug build panics, a release build leaves it out and
    // logs it
    let text = "a".repeat(300);
    let note = Note { seq: 42, text };
    if cfg!(debug_assertions) {
        let encode = std::panic::catch_unwind(|| {
            note.encode_value(&mut Vec::new());
        });
        assert!(encode.is_err());
    } else {
        let mut encoded = Vec::new();
        note.encode_value(&mut encoded);
        println!("text of 300 bytes -> {encoded:02X?}");
        assert_eq!(encoded, [0x01, 0x01, 0x2A]);
    }

    // a BER length of 200 is 2 bytes; a release build writes
    // the second over the value's first, which then reads as
    // cut short
    let mut wire = vec![0x01, 0x81, 0xC8];
    wire.extend([b'a'; 199]);
    #[cfg(not(debug_assertions))]
    {
        let text = "a".repeat(200);
        let mut encoded = Vec::new();
        BerNote { text }.encode_value(&mut encoded);
        assert_eq!(encoded, wire);
    }
    let error = BerNote::decode_value(&mut wire.as_slice()).unwrap_err();
    assert_eq!(ErrorKind::of(&error), Some(ErrorKind::MissingRequired));

    // with no `size(..)`, a release build writes a length of
    // 300 as 44, 300 mod 256: the name reads as 44 bytes, then
    // the rest as items of key 78, the third of them cut short
    let mut wire = vec![0x01, 0x01, 0x2A, 0x02, 0x02, 0x09, 0x2E, 0x03, 0x2C];
    wire.extend([b'x'; 300]);
    #[cfg(not(debug_assertions))]
    {
        let name = "x".repeat(300);
        let mut encoded = Vec::new();
        Named {
            seq: 42,
            temp: 2350,
            name,
        }
        .encode_value(&mut encoded);
        assert_eq!(encoded, wire);
    }
    let mut input = wire.as_slice();
    let named = Named::decode_value(&mut input).unwrap();
    println!(
        "name of 300 bytes, length 44 -> name of {} bytes",
        named.name.len()
    );
    assert_eq!((named.seq, named.temp), (42, 2350));
    assert_eq!(named.name, "x".repeat(44));
    assert_eq!(input.len(), 12);
}
