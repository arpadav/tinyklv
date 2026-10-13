#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Example 15 - synchronous decode over a buffered Tokio channel
//!
//! Tinyklv itself is fully synchronous and operates on byte slices. That
//! makes it trivial to integrate with any async executor: the async runtime
//! handles I/O, accumulates bytes into a buffer, and hands that buffer to
//! `decode_frame` in a normal `while let Ok(...)` loop. The sentinel seeker
//! tolerates partial chunks - if the next frame is incomplete, the decoder
//! returns `Err` and the loop waits for more bytes before retrying
//!
//! This example spawns a sender task that emits randomly-padded frames with
//! decoy blocks around each real one, and a receiver task that accumulates
//! incoming chunks and drains as many complete frames as possible after
//! each receive
//!
//! Showcases:
//! * Synchronous tinyklv decoding driven by an async producer
//! * Buffer management: keep the unconsumed tail between receives
//! * The seeker skipping padding and decoys to find the real sentinel
//!
//! See also: book Tutorial 15
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders
// --------------------------------------------------
// external
// --------------------------------------------------
use rand::prelude::*;

#[derive(Klv, Debug, PartialEq, Clone)]
#[klv(
    stream = &[u8],
    sentinel = b"ENVREAD",
    key(dec = decb::u8,          enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
/// `IoT` environment reading transmitted over a UDP-like async channel
struct EnvReading {
    /// Sensor node identifier
    #[klv(
        key = 0x01,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    node_id: u32,

    /// Temperature in degrees Celsius (f32)
    #[klv(
        key = 0x02,
        dec = decb::be_f32,
        enc = *encb::be_f32,
    )]
    temperature_c: f32,

    /// Relative humidity, 0..=100 percent
    #[klv(
        key = 0x03,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    humidity_pct: u8,
}

/// Build a noisy transport chunk around one encoded frame: random leading
/// padding, an optional decoy block, the real frame, an optional trailing
/// decoy, and trailing padding. Exercises `seek_sentinel` under realistic
/// network conditions
fn wrap_with_noise(
    reading: &EnvReading,
    rng:     &mut rand::rngs::SmallRng,
) -> Vec<u8> {
    let mut chunk: Vec<u8> = Vec::new();

    // leading padding
    let pre_pad_len = rng.random_range(0..64);
    let mut pre_pad = vec![0u8; pre_pad_len];
    rng.fill_bytes(&mut pre_pad);
    chunk.extend_from_slice(&pre_pad);

    // optional leading decoy
    if rng.random_bool(0.5) {
        let decoy_len = rng.random_range(4..48);
        let mut decoy = vec![0u8; decoy_len];
        rng.fill_bytes(&mut decoy);
        chunk.extend_from_slice(&decoy);
    }

    // the real frame
    reading.encode_frame(&mut chunk);

    // optional trailing decoy
    if rng.random_bool(0.5) {
        let decoy_len = rng.random_range(4..48);
        let mut decoy = vec![0u8; decoy_len];
        rng.fill_bytes(&mut decoy);
        chunk.extend_from_slice(&decoy);
    }

    // trailing padding
    let post_pad_len = rng.random_range(0..64);
    let mut post_pad = vec![0u8; post_pad_len];
    rng.fill_bytes(&mut post_pad);
    chunk.extend_from_slice(&post_pad);

    chunk
}

#[tokio::main]
async fn main() {
    // build - generate 16..=32 distinct readings for the round-trip test
    let mut rng = rand::rng();
    let num_packets = rng.random_range(16..=32);
    let originals: Vec<EnvReading> = (0..num_packets)
        .map(|_| EnvReading {
            node_id:       rng.random_range(1000..=9999),
            temperature_c: rng.random_range(-20.0..=50.0),
            humidity_pct:  rng.random_range(0..=100),
        })
        .collect();

    // set up the byte-chunk channel between the async sender and receiver
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);

    // spawn the sender: encode each reading into a noisy transport chunk
    let originals_clone = originals.clone();
    let sender = tokio::spawn(async move {
        let mut rng = rand::rngs::SmallRng::from_seed([0u8; 32]);
        for reading in &originals_clone {
            let chunk = wrap_with_noise(reading, &mut rng);
            tx.send(chunk).await.expect("channel closed");
        }
    });

    // spawn the receiver: accumulate chunks, drain complete frames after
    // each receive, and keep any unconsumed tail for the next round
    let receiver = tokio::spawn(async move {
        let mut buffer:  Vec<u8>          = Vec::new();
        let mut decoded: Vec<EnvReading>  = Vec::new();

        while let Some(chunk) = rx.recv().await {
            buffer.extend_from_slice(&chunk);

            // decode as many complete frames as are currently buffered
            let mut slice = buffer.as_slice();
            while let Ok(reading) = EnvReading::decode_frame(&mut slice) {
                decoded.push(reading);
            }

            // keep only the unconsumed tail
            let consumed = buffer.len() - slice.len();
            buffer.drain(..consumed);
        }

        decoded
    });

    // assert - every sent reading comes back in order, with exact field equality
    sender.await.expect("sender panicked");
    let decoded = receiver.await.expect("receiver panicked");
    assert_eq!(decoded.len(), originals.len(), "frame count must match");
    for (got, want) in decoded.iter().zip(originals.iter()) {
        assert_eq!(got.node_id, want.node_id);
        assert!((got.temperature_c - want.temperature_c).abs() < 1e-10);
        assert_eq!(got.humidity_pct, want.humidity_pct);
    }
}
