//! A user encoder named like the crate's big-endian codecs, under a fixed-width length slot
//!
//! A `be_` / `le_` name is taken for the crate's encoder on any path. A user's encoder of such a
//! name must not stand in for the crate's: its `be_u32` here writes a varint, so its value's
//! length is back-filled from the bytes it wrote, and the frame decodes back. A generic one, its
//! `be_u16`, is inferred from the value it is called with, as any encoder is
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// A user codec module, its `be_u32` a varint and its `be_u16` a varint of any value that widens
/// to a `u64`
mod codec {
    pub(super) use crate::common::varint::{dec_u32, enc_u32 as be_u32};
    use crate::common::varint::{dec_u64, enc_u64};

    use winnow::Parser;

    /// Encodes a value that widens to a `u64` as a varint
    pub(super) fn be_u16<T: Into<u64>>(value: T, out: &mut Vec<u8>) {
        enc_u64(value.into(), out);
    }

    /// Decodes a varint as a `u16`, an error on a value past `u16::MAX`
    pub(super) fn dec_u16(input: &mut &[u8]) -> tinyklv::Result<u16> {
        dec_u64.try_map(u16::try_from).parse_next(input)
    }
}

/// A count under a one-byte length slot, written by the user's `codec::be_u32`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Odometer {
    /// key 0x01, a varint
    #[klv(key = 0x01, dec = codec::dec_u32, enc = *codec::be_u32)]
    count: u32,
}

/// A lap count under a one-byte length slot, written by the user's generic `codec::be_u16`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Laps {
    /// key 0x01, a varint
    #[klv(key = 0x01, dec = codec::dec_u16, enc = *codec::be_u16)]
    count: u16,
}

/// The count's varint is two bytes, not the four a big-endian `u32` takes: its length is the two
/// bytes written, and the frame decodes back to the count
#[test]
fn a_user_be_name_takes_the_measured_length() {
    #[rustfmt::skip]
    let wire = [
        // count = 300, a two-byte varint
        0x01, 0x02, 0xAC, 0x02,
    ];
    let odometer = Odometer { count: 300 };
    assert_eq!(odometer.encoded(), wire);
    odometer.assert_decodes_from(&wire);
}

/// The generic encoder takes the `u16` it is called with, and its three-byte varint, not the two
/// bytes a big-endian `u16` takes, is the measured length
#[test]
fn a_generic_user_be_name_takes_the_measured_length() {
    #[rustfmt::skip]
    let wire = [
        // count = 20000, a three-byte varint
        0x01, 0x03, 0xA0, 0x9C, 0x01,
    ];
    let laps = Laps { count: 20_000 };
    assert_eq!(laps.encoded(), wire);
    laps.assert_decodes_from(&wire);
}
