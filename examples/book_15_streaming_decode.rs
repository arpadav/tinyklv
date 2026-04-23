#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/15-streaming-decode.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream   = &[u8],
    sentinel = b"PKT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// Sentinel-framed packet. The `sentinel = b"PKT"` attribute is what
/// makes `Decoder<Packet>` work - it is how the decoder locates packet
/// boundaries inside a continuous byte stream
struct Packet {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    id: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    value: u16,
}

fn main() {
    // encode three packets as complete frames (sentinel + length + body)
    let want = [
        Packet { id: 1, value: 100 },
        Packet { id: 2, value: 200 },
        Packet { id: 3, value: 300 },
    ];
    let mut wire: Vec<u8> = Vec::new();
    for p in &want {
        wire.extend(p.encode_frame());
    }

    // simulate a drip-feed transport: 3-byte chunks, well below one
    // packet's framed size (3 sentinel + 1 length + 9 body = 13 bytes
    // per packet), so every packet is split across multiple feeds
    let mut dec = Packet::decoder();
    let mut got: Vec<Packet> = Vec::new();
    for chunk in wire.chunks(3) {
        dec.feed(chunk);
        for pkt in dec.by_ref() {
            got.push(pkt.unwrap());
        }
    }

    assert_eq!(got, want);
    assert!(dec.buffered().is_empty(), "no bytes left behind");
    println!("decoded {} packets across 3-byte chunks", got.len());
}
