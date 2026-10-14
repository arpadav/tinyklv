//! A key + len struct that only encodes or only decodes, and a setting only the other impl rejects
//!
//! A key two fields share and a sentinel on a `&str` stream break only the decode, and a variant
//! two `oneof(..)` arms share only the encode, so a struct without that impl compiles and works:
//! this file failing to compile is the regression
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// Two readings written under one key
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(enc = encb::u8),
    len(enc = encb::u8_from_usize),
    allow_unimplemented_decode,
)]
struct Echo {
    /// key 0x10
    #[klv(key = 0x10, enc = *encb::u8)]
    first: u8,

    /// key 16, the same key
    #[klv(key = 16, enc = *encb::u8)]
    second: u8,
}

/// A reading framed by a sentinel, its decode stream a `&str`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &str,
    sentinel = b"KLV",
    key(enc = encb::u8),
    len(enc = encb::u8_from_usize),
    allow_unimplemented_decode,
)]
struct Banner {
    /// key 0x01
    #[klv(key = 0x01, enc = *encb::u8)]
    id: u8,
}

/// A census payload, whichever key carries it
#[derive(Debug, PartialEq)]
enum Payload {
    /// A head count
    Count(u8),
}

/// A payload two keys decode into the one variant
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
    default(typ = u8, dec = decb::u8),
)]
struct Census {
    /// keys 0x02 and 0x03, both a `Count`
    #[klv(key = oneof(0x02 => Payload::Count(u8), 0x03 => Payload::Count(u8)))]
    payload: Option<Payload>,
}

/// The encode-only structs write each field, the shared key twice, and the sentinel before a
/// framed body
#[test]
fn an_encode_only_struct_takes_no_decode_check() {
    #[rustfmt::skip]
    let echo = [
        // first = 5, then second = 7, both under key 0x10
        0x10, 0x01, 0x05,
        0x10, 0x01, 0x07,
    ];
    let (first, second) = (5, 7);
    assert_eq!(Echo { first, second }.encoded(), echo);
    #[rustfmt::skip]
    let banner = [
        // the sentinel, then the body's length
        0x4B, 0x4C, 0x56, 0x03,
        // id = 7
        0x01, 0x01, 0x07,
    ];
    let mut out = Vec::new();
    Banner { id: 7 }.encode_frame(&mut out);
    assert_eq!(out, banner);
}

/// The decode-only struct decodes either key into the variant both arms name
#[test]
fn a_decode_only_struct_takes_no_encode_check() {
    for key in [0x02, 0x03] {
        let census = Census {
            payload: Some(Payload::Count(9)),
        };
        census.assert_decodes_from(&[key, 0x01, 0x09]);
    }
}
