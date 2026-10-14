//! A key + len derive in a `#![no_implicit_prelude]` module, where `u8` and `usize` name the
//! user's own types
//!
//! The generated decode code names no prelude item, and spells each primitive it names by its
//! `core` path, the `&[u8]` default stream's element included: [`bare::Gauge`], a sentinel struct
//! with a length-checked field, must decode there in both drivers and from its frame
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::traits::DecodeFrame;

/// The module with no prelude, and its own `u8` and `usize`
mod bare {
    #![no_implicit_prelude]

    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use ::tinyklv::dec::binary as decb;

    /// Named like the primitive the default stream's bytes are
    #[expect(non_camel_case_types, reason = "named like the primitive")]
    #[expect(dead_code, reason = "the derive must not name it")]
    struct u8;

    /// Named like the primitive the derive's lengths are
    #[expect(non_camel_case_types, reason = "named like the primitive")]
    #[expect(dead_code, reason = "the derive must not name it")]
    struct usize;

    /// A gauge level and mode behind a sentinel, the level's length checked
    #[derive(::tinyklv::Klv, ::core::fmt::Debug, ::core::cmp::PartialEq)]
    #[klv(
        sentinel = b"\xAA",
        key(dec = decb::u8),
        len(dec = decb::u8_as_usize),
        allow_unimplemented_encode,
    )]
    pub(super) struct Gauge {
        /// key `0x01`, two bytes, its length checked
        #[klv(key = 0x01, dec = decb::be_u16, size(exact = 2))]
        pub(super) level: ::core::primitive::u16,

        /// key `0x02`, one byte
        #[klv(key = 0x02, dec = decb::u8)]
        pub(super) mode: ::core::primitive::u8,
    }
}

/// [`bare::Gauge`] decodes in both drivers and from its sentinel frame
#[test]
fn a_derive_needs_no_implicit_prelude() {
    let gauge = bare::Gauge { level: 42, mode: 5 };
    #[rustfmt::skip]
    let body = [
        // level = 42
        0x01, 0x02, 0x00, 0x2A,
        // mode = 5
        0x02, 0x01, 0x05,
    ];
    let frame = [&[0xAA, 0x07], &body[..]].concat();
    gauge.assert_decodes_from(&body);
    assert_eq!(bare::Gauge::decode_frame(&mut &frame[..]), Ok(gauge));
}
