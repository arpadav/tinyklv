#![allow(clippy::unwrap_used, clippy::expect_used)]
//! End-to-end async test: tokio mpsc channel simulates a network stream.
//! A sender task encodes 5 telemetry frames and sends each as `Vec<u8>` over
//! a channel; a receiver task accumulates bytes into a buffer and calls
//! `decode_frame` on it, printing each decoded struct. This demonstrates how
//! tinyklv integrates naturally into async Rust without any special async
//! support in the library - encoding and decoding are synchronous operations
//! that work on byte slices, leaving the async framing entirely to your
//! executor. Five full encode → decode roundtrips are asserted.
use rand::prelude::*;
use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}
fn enc_f32(v: &f32) -> Vec<u8> {
    tinyklv::enc::binary::be_f32(*v)
}
fn enc_u8(v: &u8) -> Vec<u8> {
    tinyklv::enc::binary::u8(*v)
}

#[derive(Klv, Debug, PartialEq, Clone)]
/// IoT environment sensor reading transmitted over a UDP-like async channel
#[klv(
    stream = &[u8],
    sentinel = b"\xE5\x5E",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct EnvReading {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    /// Sensor node ID
    node_id: u32,

    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_f32, enc = enc_f32)]
    /// Temperature in 0.01 °C units stored as f32
    temperature_c: f32,

    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    /// Relative humidity 0-100%
    humidity_pct: u8,
}

#[tokio::main]
async fn main() {
    // --------------------------------------------------
    // channel: sender pushes encoded frames; receiver accumulates + decodes
    // --------------------------------------------------
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<u8>>(16);

    // --------------------------------------------------
    // build 16-32 distinct readings that we will round-trip
    // --------------------------------------------------
    let mut rng = rand::rng();
    let num_packets = rng.random_range(16..=32);
    let originals: Vec<EnvReading> = (0..num_packets)
        .map(|_| EnvReading {
            node_id: rng.random_range(1000..=9999),
            temperature_c: rng.random_range(-20.0..=50.0),
            humidity_pct: rng.random_range(0..=100),
        })
        .collect();

    // --------------------------------------------------
    // sender task: encode each reading and push it as a framed byte vec
    // --------------------------------------------------
    let originals_clone = originals.clone();
    let sender = tokio::spawn(async move {
        let mut rng = rand::rngs::SmallRng::from_seed([0u8; 32]);
        for reading in &originals_clone {
            let mut chunk: Vec<u8> = Vec::new();
            // --------------------------------------------------
            // random-length padding before
            // --------------------------------------------------
            let pre_pad_len = rng.random_range(0..64);
            let mut pre_pad = vec![0u8; pre_pad_len];
            rng.fill_bytes(&mut pre_pad);
            chunk.extend_from_slice(&pre_pad);
            // --------------------------------------------------
            // optional decoy "packet" before (~50% of the time)
            // --------------------------------------------------
            if rng.random_bool(0.5) {
                let decoy_len = rng.random_range(4..48);
                let mut decoy = vec![0u8; decoy_len];
                rng.fill_bytes(&mut decoy);
                chunk.extend_from_slice(&decoy);
            }
            // --------------------------------------------------
            // the real frame
            // --------------------------------------------------
            chunk.extend_from_slice(&reading.encode_frame());
            // --------------------------------------------------
            // optional decoy packet after
            // --------------------------------------------------
            if rng.random_bool(0.5) {
                let decoy_len = rng.random_range(4..48);
                let mut decoy = vec![0u8; decoy_len];
                rng.fill_bytes(&mut decoy);
                chunk.extend_from_slice(&decoy);
            }
            // --------------------------------------------------
            // random-length padding after
            // --------------------------------------------------
            let post_pad_len = rng.random_range(0..64);
            let mut post_pad = vec![0u8; post_pad_len];
            rng.fill_bytes(&mut post_pad);
            chunk.extend_from_slice(&post_pad);
            // --------------------------------------------------
            // send
            // --------------------------------------------------
            tx.send(chunk).await.expect("channel closed");
        }
    });

    // Receiver task: collect all byte chunks into a buffer and decode_frame
    let receiver = tokio::spawn(async move {
        let mut buffer: Vec<u8> = Vec::new();
        let mut decoded: Vec<EnvReading> = Vec::new();

        // Receive all frames sent by the sender
        while let Some(chunk) = rx.recv().await {
            // Append incoming bytes to the accumulation buffer
            buffer.extend_from_slice(&chunk);

            // Attempt to decode as many complete frames as are available;
            // decode_frame advances the slice past the consumed bytes
            let mut slice = buffer.as_slice();
            while let Ok(reading) = EnvReading::decode_frame(&mut slice) {
                println!(
                    "Decoded: node_id={}, temp={:.2}°C, humidity={}%",
                    reading.node_id, reading.temperature_c, reading.humidity_pct
                );
                decoded.push(reading);
            }
            // Keep only the unconsumed remainder
            let consumed = buffer.len() - slice.len();
            buffer.drain(..consumed);
        }

        decoded
    });

    sender.await.expect("sender panicked");
    let decoded = receiver.await.expect("receiver panicked");
    println!(
        "\nRoundtrip summary: {} frames sent, {} decoded",
        originals.len(),
        decoded.len()
    );
    assert_eq!(
        decoded.len(),
        originals.len(),
        "must decode exactly 5 frames"
    );
    for (got, want) in decoded.iter().zip(originals.iter()) {
        assert_eq!(got.node_id, want.node_id);
        assert!((got.temperature_c - want.temperature_c).abs() < 1e-10);
        assert_eq!(got.humidity_pct, want.humidity_pct);
    }
    println!("SUCCESS");
}
