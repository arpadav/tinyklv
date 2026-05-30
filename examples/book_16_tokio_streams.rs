#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/16-tokio-streams.md` for full example
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

/// Encode packets into KLV frames/packets, split each in two, return all chunks.
fn make_chunks(packets: &[Heartbeat]) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
    // junk / zeros before the first sentinel - the decoder skips them
    chunks.push(vec![0xDE, 0xAD, 0x00, 0x00, 0xFF, 0x00]);
    for p in packets {
        let mut frame = Vec::new();
        p.encode_frame(&mut frame);
        let (head, tail) = frame.split_at(frame.len() / 2);
        chunks.push(head.to_vec());
        chunks.push(tail.to_vec());
    }
    chunks
}

#[tokio::main]
async fn main() {
    let originals = vec![
        Heartbeat { sequence: 1, temperature_centideg: 2300, uptime_s: 10 },
        Heartbeat { sequence: 2, temperature_centideg: 2310, uptime_s: 20 },
        Heartbeat { sequence: 3, temperature_centideg: 2340, uptime_s: 30 },
    ];
    let chunks = make_chunks(&originals);

    // Pattern 1: real-time feed/iter driven by channel receives.
    // Each await point may deliver a partial frame; the decoder reassembles.
    {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);

        let producer_chunks = chunks.clone();
        let producer = tokio::spawn(async move {
            for chunk in producer_chunks {
                tx.send(chunk).await.unwrap();
            }
        });

        let consumer = tokio::spawn(async move {
            let mut dec = Heartbeat::decoder();
            let mut decoded: Vec<Heartbeat> = Vec::new();
            while let Some(chunk) = rx.recv().await {
                dec.feed(&chunk);
                for pkt in dec.iter() {
                    decoded.push(pkt);
                }
            }
            decoded
        });

        producer.await.unwrap();
        let decoded = consumer.await.unwrap();
        assert_eq!(decoded, originals, "pattern 1 (feed + iter)");
        println!("pattern 1: decoded {} packets in real-time", decoded.len());
    }
}
