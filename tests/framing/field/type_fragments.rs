//! A field type a `macro_rules!` passes in as a `$t:ty` fragment, or an `Option` spelled by its
//! path
//!
//! Each frames as the type written out: a `$t:ty` primitive keeps its known-width write under a
//! fixed-width length slot, and a `$t:ty` `Option<u16>` or a `std::option::Option<u16>` is an
//! optional field, absent from the wire when `None`
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// Writes a length as two bytes, though the container declares a one-byte slot: a known-width
/// value's length is written as it is, where a back-filled one fails the slot's check
fn two_byte_len(len: usize, out: &mut Vec<u8>) {
    out.push(0);
    encb::u8_from_usize(len, out);
}

/// Declares a gauge named `$name`, its level a `$t`
#[rustfmt::skip]
macro_rules! gauge {
    ($name:ident, $t:ty) => {
        /// A level under a one-byte length slot, declared through [`gauge!`]
        #[derive(Klv, Debug, PartialEq)]
        #[klv(
            key(dec = decb::u8, enc = encb::u8),
            len(dec = decb::u8_as_usize, enc = two_byte_len, size(exact = 1)),
        )]
        struct $name {
            /// key 0x01
            #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
            level: $t,
        }
    };
}
gauge!(Level, u16);
gauge!(MaybeLevel, Option<u16>);

/// A level under a one-byte length slot, its `Option` spelled by its path
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = two_byte_len, size(exact = 1)),
)]
struct PathLevel {
    /// key 0x01
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    level: std::option::Option<u16>,
}

/// The level's frame, its length as [`two_byte_len`] writes it
#[rustfmt::skip]
const LEVEL: [u8; 5] = [
    // key, then the length 2 in two bytes
    0x01, 0x00, 0x02,
    // level = 0x0102
    0x01, 0x02,
];

/// A `$t:ty` `u16` is two bytes wide, as `u16` is, so its length is written before it
#[test]
fn a_type_fragment_primitive_keeps_its_known_width_write() {
    assert_eq!(Level { level: 0x0102 }.encoded(), LEVEL);
}

/// A `$t:ty` `Option<u16>` and a `std::option::Option<u16>` are optional: `Some` writes the
/// level's frame, `None` writes nothing, and an empty input decodes to `None`
#[test]
fn an_option_by_fragment_or_by_path_is_optional() {
    let level = Some(0x0102);
    assert_eq!(MaybeLevel { level }.encoded(), LEVEL);
    assert_eq!(MaybeLevel { level: None }.encoded(), []);
    MaybeLevel { level: None }.assert_decodes_from(&[]);
    assert_eq!(PathLevel { level }.encoded(), LEVEL);
    assert_eq!(PathLevel { level: None }.encoded(), []);
    PathLevel { level: None }.assert_decodes_from(&[]);
}
