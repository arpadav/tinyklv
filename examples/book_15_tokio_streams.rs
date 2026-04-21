#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! See: `book/tutorial/15-tokio-streams.md` for full example
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    sentinel = b"HEARTBEAT",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct HeartbeatPacket {
    #[klv(key = 0x01, dec = decb::u8,     enc = *encb::u8)]     sequence:             u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)] temperature_centideg: u16,
    #[klv(key = 0x03, dec = decb::be_u32, enc = *encb::be_u32)] uptime_s:             u32,
}

#[tokio::main]
async fn main() {
    let originals = vec![
        HeartbeatPacket { sequence: 1, temperature_centideg: 2300, uptime_s: 10 },
        HeartbeatPacket { sequence: 2, temperature_centideg: 2310, uptime_s: 20 },
        HeartbeatPacket { sequence: 3, temperature_centideg: 2340, uptime_s: 30 },
    ];

    // mpsc channel simulates a UDP socket - bytes arrive in arbitrary-size chunks
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(8);

    // producer task: serialise each frame and slice it into two chunks to
    // prove the receiver can reassemble a frame split across awaits
    let producer_originals = originals.clone();
    let producer = tokio::spawn(async move {
        for packet in &producer_originals {
            let frame = packet.encode_frame();
            let split = frame.len() / 2;
            tx.send(frame[..split].to_vec()).await.unwrap();
            tx.send(frame[split..].to_vec()).await.unwrap();
        }
    });

    // consumer task: accumulate bytes, run sync decode_frame in a loop,
    // drain whatever was consumed, keep the remainder for the next chunk
    let consumer = tokio::spawn(async move {
        let mut buffer: Vec<u8> = Vec::new();
        let mut decoded: Vec<HeartbeatPacket> = Vec::new();
        while let Some(chunk) = rx.recv().await {
            buffer.extend_from_slice(&chunk);

            // decode everything that is currently complete. decode_frame may
            // advance the cursor partially on a mid-frame split and then fail;
            // save the cursor before each attempt so we can restore it and
            // leave the in-progress frame bytes untouched for the next chunk
            let mut cursor = buffer.as_slice();
            loop {
                let saved = cursor;
                match HeartbeatPacket::decode_frame(&mut cursor) {
                    Ok(packet) => decoded.push(packet),
                    Err(_) => {
                        cursor = saved;
                        break;
                    }
                }
            }

            // drop the bytes of every fully-decoded frame; leave the rest
            let consumed = buffer.len() - cursor.len();
            buffer.drain(..consumed);
        }
        decoded
    });

    producer.await.unwrap();
    let decoded = consumer.await.unwrap();

    // all three frames were reassembled across split chunks
    assert_eq!(decoded, originals);
}
