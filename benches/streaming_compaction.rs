//! Compares two ways of decoding a chunked KLV stream with the tinyklv API: the streaming
//! [`tinyklv::Decoder`] (`feed` + `next`) against calling the one-shot `Sample::decode_frame` in
//! a loop over a hand-managed buffer, on the small-chunk streaming pattern (short reads where a
//! packet straddles several feeds), across a growing packet count.
//!
//! Both series consume the *same* chunked feed of `Sample` frames and decode the *same* body:
//!
//! * `decoder` - `Sample::decoder()`, then `feed(chunk)` + drain via `next()`. The Decoder owns
//!   the buffer and advances a consumed-prefix cursor in O(1) per packet, compacting on `feed`
//!   only when the freed prefix is at least as large as the live tail it must shift (the
//!   `self.head >= self.buf.len() - self.head` watermark in `BufCursor::extend`, src/decoder/buf.rs).
//! * `decode_frame` - the caller owns a `Vec`, appends each chunk, pulls every complete frame out
//!   with `Sample::decode_frame`, and drains the consumed prefix off the front. This is the buffer
//!   management you hand-write when you reach for the one-shot API instead of the Decoder.
//!
//! This isolates what the streaming Decoder buys over the plain `decode_frame` API on the
//! straddling-feed pattern. Lower is faster.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::Klv;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Packet counts swept by the bench; the `decoder` series should stay flat per byte across these
const PACKET_COUNTS: &[usize] = &[64, 256, 1024, 4096];

/// Bytes per fed chunk - deliberately small so a packet straddles several feeds, exercising the
/// per-feed buffer management repeatedly rather than decoding a whole buffer in one shot
const CHUNK_LEN: usize = 4;

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"SMPL",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Sample {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    a: u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)]
    b: u16,
}

/// Builds a back-to-back stream of `n` framed [`Sample`] packets
///
/// # Arguments
///
/// * `n` - The number of packets to concatenate into the stream
///
/// # Returns
///
/// A `Vec<u8>` holding `n` sentinel-framed packets end to end
fn build_stream(n: usize) -> Vec<u8> {
    let mut buf = Vec::new();
    for i in 0..n {
        buf.extend(
            Sample {
                a: i as u8,
                b: i as u16,
            }
            .encode_frame(),
        );
    }
    buf
}

/// Feeds `stream` through a [`tinyklv::Decoder`] in [`CHUNK_LEN`]-byte chunks, draining greedily
///
/// This is the O(1)-cursor series: the decoder never moves a live byte during a `next` burst.
///
/// # Arguments
///
/// * `stream` - The full multi-packet byte stream to decode
///
/// # Returns
///
/// The number of packets successfully decoded
fn decode_with_decoder(stream: &[u8]) -> usize {
    let mut dec = Sample::decoder();
    let mut count = 0usize;
    for chunk in stream.chunks(CHUNK_LEN) {
        dec.feed(chunk);
        while let Some(pkt) = dec.next::<Sample>() {
            black_box(pkt);
            count += 1;
        }
    }
    count
}

/// Decodes `stream` with the one-shot `Sample::decode_frame` API plus a hand-managed buffer
///
/// The same chunked feed, but the caller owns the buffer: append each chunk, pull out every
/// complete frame with [`Sample::decode_frame`], then drain the consumed prefix off the front and
/// keep the partial tail for the next chunk. This is what reaching for `decode_frame` instead of
/// the streaming [`tinyklv::Decoder`] looks like.
///
/// # Arguments
///
/// * `stream` - The full multi-packet byte stream to decode
///
/// # Returns
///
/// The number of packets successfully decoded
fn decode_with_decode_frame(stream: &[u8]) -> usize {
    let mut buf: Vec<u8> = Vec::new();
    let mut count = 0usize;
    for chunk in stream.chunks(CHUNK_LEN) {
        buf.extend_from_slice(chunk);
        // --------------------------------------------------
        // pull every complete frame out of the accumulated buffer, committing the
        // cursor only on a successful decode (a failed probe leaves `input` put)
        // --------------------------------------------------
        let mut input: &[u8] = &buf;
        loop {
            let mut probe = input;
            match Sample::decode_frame(&mut probe) {
                Ok(pkt) => {
                    black_box(pkt);
                    count += 1;
                    input = probe;
                }
                Err(_) => break,
            }
        }
        // --------------------------------------------------
        // drain the consumed prefix, keep the partial tail for the next chunk
        // --------------------------------------------------
        let consumed = buf.len() - input.len();
        buf.drain(..consumed);
    }
    count
}

/// Registers the `decoder` and `manual` series across [`PACKET_COUNTS`]
///
/// # Arguments
///
/// * `c` - The criterion driver to register the benchmark group under
fn streaming_compaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_compaction");
    for &n in PACKET_COUNTS {
        let stream = build_stream(n);
        group.throughput(Throughput::Bytes(stream.len() as u64));
        // --------------------------------------------------
        // the streaming Decoder vs the one-shot decode_frame API
        // --------------------------------------------------
        group.bench_with_input(BenchmarkId::new("decoder", n), &stream, |b, stream| {
            b.iter(|| decode_with_decoder(black_box(stream)));
        });
        group.bench_with_input(BenchmarkId::new("decode_frame", n), &stream, |b, stream| {
            b.iter(|| decode_with_decode_frame(black_box(stream)));
        });
    }
    group.finish();
}

criterion_group!(benches, streaming_compaction);
criterion_main!(benches);
