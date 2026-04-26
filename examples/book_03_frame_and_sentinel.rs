#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/03-frames-and-sentinels.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro, DecodeFrame, and more
use tinyklv::dec::binary as decb;   // binary decoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],                 // default, shown explicitly for clarity
    sentinel = b"HEARTBEAT",        // readable marker at the frame boundary
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
)]
struct Heartbeat {
    #[klv(
        key = 0x01,
        dec = decb::u8,
    )]
    sequence: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
    )]
    temperature_centideg: u16,
}

fn main() {
    // hand-built stream: garbage bytes, then a real frame
    let stream = [
        0xDE, 0xAD, 0xBE, 0x00, // junk preamble, no sentinel here
        // HEARTBEAT sentinel
        0x48, 0x45, 0x41, 0x52, 0x54, 0x42, 0x45, 0x41, 0x54,
        0x07,                   // body length = 7 bytes
        0x01,                   // sequence key
        0x01,                   // value len = 1
        0x2A,                   // sequence value = 42
        0x02,                   // temperature key
        0x02,                   // value len = 2
        0x09, 0x2E,             // temperature value = 2350
    ];

    // manually constructed expected value
    let expected = Heartbeat {
        sequence: 42,
        temperature_centideg: 2350,
    };

    // decode_frame seeks past the junk, matches the sentinel, reads the
    // length, then decodes the body
    let decoded = Heartbeat::decode_frame(
        &mut stream.as_slice(),
    ).unwrap();

    // they equal!
    assert_eq!(decoded, expected);
}
