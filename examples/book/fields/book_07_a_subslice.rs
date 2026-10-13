#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 07a - sub-slice length: decoder reads fewer bytes than `len` indicates
//! See `book/tutorial/07-val-lengths.md` for the full narrative
//!
//! Illustrates that the `len` field is an upper bound on how many bytes the
//! value decoder may consume, not a strict contract. When the stream says the
//! value region is 4 bytes but the `be_u16` decoder only needs 2, it reads
//! the first 2 and leaves the rest. This lets senders pad fields for alignment
//! without breaking decoders that know only the narrower wire type
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
)]
/// Single-field struct that reads a `u16` from a 4-byte padded value region
struct U16OnlyTakeTwo {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
    )]
    /// Big-endian u16 read from the first two bytes of the value region
    value: u16,
}

fn main() {
    // manually construct the stream
    let stream = [
        0x01, 0x04, // key = 1, but len is 4 for a u16?
        0x09, 0x2E, 0x00, 0x00, // value = 2350
        // and for little-endian, you can use:
        // 0x00, 0x00, 0x2E, 0x09
    ];

    // manually construct the expected value
    let expected = U16OnlyTakeTwo {
        value: 2350,
    };

    // seek sentinel, decode the value
    let decoded = U16OnlyTakeTwo::decode_value(
        &mut stream.as_slice(),
    ).unwrap();

    // they equal!
    assert_eq!(decoded, expected);
}
