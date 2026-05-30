#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 02 - out-of-order key decoding
//! See `book/tutorial/02-out-of-order.md` for the full narrative
//!
//! Shows that `DecodeValue` does not require keys to appear in declaration
//! order. The same `Heartbeat` struct from tutorial 01 is decoded from a
//! stream where the temperature key arrives before the sequence key - the
//! generated loop collects them in any order and assembles the struct at the
//! end
use tinyklv::prelude::*;            // Klv and DecodeValue are already imported here
use tinyklv::dec::binary as decb;   // binary decoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
)]
/// Minimal sensor heartbeat used to demonstrate out-of-order key tolerance
struct Heartbeat {
    #[klv(
        key = 0x01,
        dec = decb::u8,
    )]
    /// Monotonic frame counter
    sequence: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
    )]
    /// Temperature in 0.01 C units (big-endian u16)
    temperature_centideg: u16,
}

fn main() {
    // manually construct the packet
    let out_of_order_stream = [
        0x02,       // temperature key
        0x02,       // value len = 2
        0x09, 0x2E, // temperature value = 2350
        0x01,       // sequence key
        0x01,       // value len = 1
        0x2A,       // sequence value = 42
    ];

    // manually construct the value
    let original_constructed = Heartbeat {
        sequence: 42,
        temperature_centideg: 2350,
    };

    // decode the value
    let decoded = Heartbeat::decode_value(
        &mut out_of_order_stream.as_slice()
    ).unwrap();

    // they equal!
    assert_eq!(decoded, original_constructed);
}
