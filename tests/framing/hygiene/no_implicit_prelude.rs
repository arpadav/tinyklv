//! Key + len derives in a `#![no_implicit_prelude]` module, where `u8`, `u64` and `usize` name the
//! user's own types
//!
//! The generated code names no prelude item, and spells each primitive it names by its `core`
//! path, the `&[u8]` default stream's element included: [`bare::Gauge`], a sentinel struct with a
//! one-byte length and a length-checked field, must decode there in both drivers and from its
//! frame and encode back to both, and [`bare::Count`], whose varint length stages each value before
//! writing it, must decode and encode
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::traits::{DecodeFrame, EncodeFrame};

/// The module with no prelude, and its own `u8`, `u64` and `usize`
mod bare {
    #![no_implicit_prelude]

    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use crate::common::varint;
    use ::tinyklv::dec::binary as decb;
    use ::tinyklv::enc::binary as encb;

    /// Named like the primitive the default stream's bytes are
    #[expect(non_camel_case_types, reason = "named like the primitive")]
    #[expect(dead_code, reason = "the derive must not name it")]
    struct u8;

    /// Named like the primitive a back-filled length is bounded as
    #[expect(non_camel_case_types, reason = "named like the primitive")]
    #[expect(dead_code, reason = "the derive must not name it")]
    struct u64;

    /// Named like the primitive the derive's lengths are
    #[expect(non_camel_case_types, reason = "named like the primitive")]
    #[expect(dead_code, reason = "the derive must not name it")]
    struct usize;

    /// A gauge level and mode behind a sentinel, every length one byte, the level's checked
    #[derive(::tinyklv::Klv, ::core::fmt::Debug, ::core::cmp::PartialEq)]
    #[klv(
        sentinel = b"\xAA",
        key(dec = decb::u8, enc = encb::u8),
        len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    )]
    pub(super) struct Gauge {
        /// key `0x01`, two bytes, its length checked
        #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16, size(exact = 2))]
        pub(super) level: ::core::primitive::u16,

        /// key `0x02`, one byte
        #[klv(key = 0x02, dec = decb::u8, enc = *encb::u8)]
        pub(super) mode: ::core::primitive::u8,
    }

    /// A count behind a varint length
    #[derive(::tinyklv::Klv, ::core::fmt::Debug, ::core::cmp::PartialEq)]
    #[klv(
        key(dec = decb::u8, enc = encb::u8),
        len(dec = varint::dec_usize, enc = varint::enc_usize),
    )]
    pub(super) struct Count {
        /// key `0x01`, two bytes
        #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
        pub(super) total: ::core::primitive::u16,
    }
}

/// [`bare::Gauge`] decodes in both drivers and from its sentinel frame and encodes back to both,
/// and [`bare::Count`] decodes in both drivers and encodes back
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
    assert_eq!(gauge.encoded(), body);
    let mut out = Vec::new();
    gauge.encode_frame(&mut out);
    assert_eq!(out, frame);
    assert_eq!(bare::Gauge::decode_frame(&mut &frame[..]), Ok(gauge));
    let count = bare::Count { total: 300 };
    #[rustfmt::skip]
    let wire = [
        // total = 300
        0x01, 0x02, 0x01, 0x2C,
    ];
    count.assert_decodes_from(&wire);
    assert_eq!(count.encoded(), wire);
}
