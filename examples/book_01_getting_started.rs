#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/01-getting-started.md` for full example
use tinyklv::Klv;                   // proc-macro
use tinyklv::DecodeValue;           // decode the value from bytes
use tinyklv::prelude::*;            // additional prelude - helps with decoder loop
use tinyklv::dec::binary as decb;   // binary decoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    allow_unimplemented_encode,
)]
struct HeartbeatPacket {
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
    // manually construct the packet
    let original_stream = [
        0x01,       // sequence key
        0x01,       // value len = 1
        0x2A,       // sequence value = 42
        0x02,       // temperature key
        0x02,       // value len = 2
        0x09, 0x2E, // temperature value = 2350
    ];

    // manually construct the value
    let original_constructed = HeartbeatPacket {
        sequence: 42,
        temperature_centideg: 2350,
    };

    // decode the value
    // `winnow` is used internally, which requires a &mut Stream, in
    // this case a &mut &[u8]. this allows for the slice to be borrowed
    // and changing the pointer in a zero-copy manner
    let decoded = HeartbeatPacket::decode_value(
        &mut original_stream.as_slice()
    ).unwrap();

    // they equal!
    assert_eq!(decoded, original_constructed);
}
