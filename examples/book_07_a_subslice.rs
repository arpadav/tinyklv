#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/07-val-lengths.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
)]
struct U16OnlyTakeTwo {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
    )]
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
