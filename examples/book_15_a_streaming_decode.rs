#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Book tutorial 15a - streaming decode via `Decoder`
//!
//! Demonstrates three patterns for incrementally decoding a sentinel-framed
//! KLV stream: (1) feeding the whole buffer then draining with `IntoIterator`,
//! (2) drip-feeding 3-byte chunks and calling `iter()` after each feed, and
//! (3) low-level `decode_partial` / `resume_partial` for callers that manage
//! their own in-flight partial state. All three patterns are asserted to yield
//! the same result
//!
//! See `book/tutorial/15-streaming-decode.md` for the full narrative.
//!
//! Author: aav
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders
use tinyklv::ResumePartial;

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
    buf.extend(want.iter().flat_map(tinyklv::EncodeFrame::encode_frame));

    // Pattern 1: feed the whole stream into `::decoder()`, then drain with
    // IntoIterator on &mut Decoder.
    {
        let mut dec = Heartbeat::decoder();
        dec.feed(&buf);
        let got: Vec<Heartbeat> = (&mut dec).into_iter().collect();
        assert_eq!(got, want, "pattern 1 (::decoder + IntoIterator)");
    }

    // Pattern 2: drip-feed framed bytes and drain with iter() after each feed.
    // 3 bytes is well below one framed packet, so every frame is split across
    // several feeds - the decoder keeps the partial packet internally.
    {
        let mut dec = Heartbeat::decoder();
        let mut got: Vec<Heartbeat> = Vec::new();
        for chunk in buf.chunks(3) {
            dec.feed(chunk);
            for pkt in dec.iter() {
                got.push(pkt);
            }
        }
        assert_eq!(got, want, "pattern 2 (::decoder + iter)");
        assert!(dec.buffered().is_empty(), "no bytes left behind");
    }

    // Pattern 3: direct `decode_partial` / `resume_partial` on one body.
    // This bypasses frame seeking completely - the caller already knows the
    // body boundaries and manages the in-flight partial explicitly.
    {
        let original = want[0].clone();
        let body = original.encode_value();
        let split = 5;

        let mut first_half: &[u8] = &body[..split];
        let partial = match Heartbeat::decode_partial(&mut first_half).unwrap() {
            Packet::Ready(_) => panic!("body should not be complete yet"),
            Packet::NeedMore(partial) => partial,
        };

        let second_half: &[u8] = &body[split..];
        let mut resumed_body = Vec::from(first_half);
        resumed_body.extend_from_slice(second_half);

        let mut resumed_input: &[u8] = &resumed_body;
        let got = match Heartbeat::resume_partial(&mut resumed_input, partial).unwrap() {
            Packet::Ready(pkt) => pkt,
            Packet::NeedMore(partial) => partial.finalize().unwrap(),
        };

        assert_eq!(got, original, "pattern 3 (decode_partial + resume_partial)");
        assert!(resumed_input.is_empty(), "resumed input fully consumed");
    }

    println!(
        "all 3 patterns decoded {} packets correctly",
        want.len(),
    );
}
