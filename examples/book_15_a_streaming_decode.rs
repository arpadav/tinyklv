#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/15-streaming-decode.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream   = &[u8], // default, shown for clarity
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Heartbeat {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    sequence: u8,

    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    temperature_centideg: u16,

    #[klv(
        key = 0x03,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    uptime_s: u32,
}

fn main() {
    let want = vec![
        Heartbeat { sequence: 1, temperature_centideg: 2300, uptime_s: 10 },
        Heartbeat { sequence: 2, temperature_centideg: 2310, uptime_s: 20 },
        Heartbeat { sequence: 3, temperature_centideg: 2340, uptime_s: 30 },
    ];
    let mut buf: Vec<u8> = vec![
        // junk / zeros before the first sentinel
        0xDE, 0xAD, 0x00, 0x00, 0xFF, 0x00,
    ];
    buf.extend(want.iter().flat_map(|p| p.encode_frame()));

    // Pattern 1: feed the whole buffer at once, iterate with for-loop.
    // IntoIterator on &mut Decoder drains all currently complete packets.
    {
        let mut dec = Heartbeat::decoder();
        dec.feed(&buf);
        let got: Vec<Heartbeat> = (&mut dec).into_iter().collect();
        assert_eq!(got, want, "pattern 1 (feed + for)");
    }

    // Pattern 2: drip-feed 3-byte chunks, drain with iter() after each feed.
    // 3 bytes is well below one framed packet (9 sentinel + 1 len + body),
    // so every packet is split across multiple feeds - the decoder reassembles.
    {
        let mut dec = Heartbeat::decoder();
        let mut got: Vec<Heartbeat> = Vec::new();
        for chunk in buf.chunks(3) {
            dec.feed(chunk);
            for pkt in dec.iter() {
                got.push(pkt);
            }
        }
        assert_eq!(got, want, "pattern 2 (chunked feed + iter)");
        assert!(dec.buffered().is_empty(), "no bytes left behind");
    }

    println!(
        "all 2 patterns decoded {} packets correctly",
        want.len(),
    );
}
