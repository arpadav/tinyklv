//! The decode of sentinel-framed streams, in two groups:
//!
//! * `streaming_compaction` - the streaming [`tinyklv::Decoder`] against the one-shot
//!   `decode_frame` over a hand-managed buffer
//! * `seek` - [`SeekSentinel::seek_sentinel`] alone, the sentinel search every framed decode runs
//!
//! `streaming_compaction` compares two ways of decoding a chunked KLV stream with the tinyklv API:
//! the streaming [`tinyklv::Decoder`] (`feed` + `next`) against calling the one-shot
//! `Sample::decode_frame` in a loop over a hand-managed buffer, on the small-chunk streaming
//! pattern (short reads where a packet straddles several feeds), across a growing packet count.
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
//! straddling-feed pattern.
//!
//! `seek` times the seek of a struct per sentinel length, 1, 4, 5, 8 and 16 bytes (each a prefix
//! of a SMPTE universal label), on four buffers:
//!
//! * `at_0_frame` - one frame, the buffer of a one-shot `decode_frame`
//! * `at_0_stream` - [`STREAM_FRAMES`] frames back to back, the buffer of a streaming `Decoder`
//! * `at_1k` - [`LEAD_LEN`] bytes of garbage, then one frame
//! * `absent_64k` - [`ABSENT_LEN`] bytes of garbage, no sentinel in them
//!
//! The garbage is pseudo-random, so a sentinel's first byte appears in it about once in 256 bytes,
//! and holds no whole sentinel. Each buffer is checked to seek to its expected body before anything
//! is timed. Lower is faster.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::Klv;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;
use tinyklv::traits::SeekSentinel;

// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
    measurement::WallTime,
};
use std::hint::black_box;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Packet counts swept by the bench; the `decoder` series should stay flat per byte across these
const PACKET_COUNTS: &[usize] = &[64, 256, 1024, 4096];

/// Bytes per fed chunk - deliberately small so a packet straddles several feeds, exercising the
/// per-feed buffer management repeatedly rather than decoding a whole buffer in one shot
const CHUNK_LEN: usize = 4;

/// Criterion samples per `seek` benchmark: far above the default 100, so the confidence intervals
/// order arms a few nanoseconds apart
const SAMPLE_SIZE: usize = 1000;

/// Frames back to back in the `at_0_stream` buffer
const STREAM_FRAMES: usize = 64;

/// Bytes of garbage ahead of the frame in the `at_1k` buffer
const LEAD_LEN: usize = 1024;

/// Bytes of garbage in the `absent_64k` buffer
const ABSENT_LEN: usize = 64 * 1024;

/// Seed of the garbage, fixed so every run seeks through the same bytes
const GARBAGE_SEED: u64 = 0x9E37_79B9_7F4A_7C15;

/// The body of every `seek` frame, the field `a = 0x2A`: its key, its length, its value
const SEEK_BODY: &[u8] = &[0x01, 0x01, 0x2A];

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"SMPL",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Sample {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    a: u8,
    #[klv(key = 0x02, dec = decb::be_u16, enc = *encb::be_u16)]
    b: u16,
}

/// A `seek` struct, whose sentinel the fixtures are built around
trait SeekCase: for<'a> SeekSentinel<&'a [u8]> {
    /// The struct's `sentinel`
    const SENTINEL: &'static [u8];
}

/// Declares a one-field `seek` struct framed by `$sentinel`, and its [`SeekCase`] impl
// rustfmt indents a multi-line attribute in a `macro_rules!` body 8 columns deeper on every run
#[rustfmt::skip]
macro_rules! seek_case {
    ($name:ident, $sentinel:tt) => {
        #[doc = concat!("A one-field struct framed by the `", stringify!($sentinel), "` sentinel")]
        #[derive(Klv, Debug)]
        #[klv(
            stream = &[u8],
            sentinel = $sentinel,
            key(dec = decb::u8, enc = encb::u8),
            len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
        )]
        struct $name {
            #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
            a: u8,
        }

        impl SeekCase for $name {
            const SENTINEL: &'static [u8] = $sentinel;
        }
    };
}

seek_case!(Seek1, b"\x06");
seek_case!(Seek4, b"\x06\x0E\x2B\x34");
seek_case!(Seek5, b"\x06\x0E\x2B\x34\x02");
seek_case!(Seek8, b"\x06\x0E\x2B\x34\x02\x0B\x01\x01");
seek_case!(
    Seek16,
    b"\x06\x0E\x2B\x34\x02\x0B\x01\x01\x0E\x01\x03\x01\x01\x00\x00\x00"
);

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
        Sample {
            a: i as u8,
            b: i as u16,
        }
        .encode_frame(&mut buf);
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

/// Builds `len` bytes of pseudo-random garbage holding no `sentinel`
///
/// An xorshift64 stream, so every byte value appears about equally often, the sentinel's first
/// byte included; a byte that would complete a `sentinel` is drawn again
///
/// # Arguments
///
/// * `len` - The number of bytes to build
/// * `sentinel` - The sentinel the garbage must not hold
///
/// # Returns
///
/// The `len` garbage bytes
fn garbage(len: usize, sentinel: &[u8]) -> Vec<u8> {
    let mut state = GARBAGE_SEED;
    let mut bytes = Vec::with_capacity(len);
    while bytes.len() < len {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        bytes.push((state >> 24) as u8);
        if bytes.ends_with(sentinel) {
            bytes.pop();
        }
    }
    bytes
}

/// Times `T::seek_sentinel` on `buffer`, after checking that it seeks to `expected`
///
/// # Arguments
///
/// * `group` - The `seek` group to register the arm under
/// * `case` - The buffer's name, the arm's id with the sentinel length
/// * `buffer` - The bytes to seek through
/// * `expected` - The body the seek returns, or `None` for a miss
fn bench_seek<T: SeekCase>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    case: &str,
    buffer: &[u8],
    expected: Option<&[u8]>,
) {
    let mut input = buffer;
    assert_eq!(
        T::seek_sentinel(&mut input).ok(),
        expected,
        "`{case}` of a {}-byte sentinel seeks to its expected body",
        T::SENTINEL.len(),
    );
    group.bench_with_input(
        BenchmarkId::new(case, T::SENTINEL.len()),
        buffer,
        |b, buffer| {
            b.iter(|| {
                let mut input = black_box(buffer);
                T::seek_sentinel(&mut input).map(<[u8]>::len)
            });
        },
    );
}

/// Registers the four `seek` arms of `T`, one per buffer
///
/// # Arguments
///
/// * `group` - The `seek` group to register the arms under
fn bench_seek_case<T: SeekCase>(group: &mut BenchmarkGroup<'_, WallTime>) {
    // the sentinel, the frame's length, its body
    let frame = [T::SENTINEL, &[SEEK_BODY.len() as u8], SEEK_BODY].concat();
    let lead = [garbage(LEAD_LEN, T::SENTINEL).as_slice(), &frame].concat();
    bench_seek::<T>(group, "at_0_frame", &frame, Some(SEEK_BODY));
    bench_seek::<T>(
        group,
        "at_0_stream",
        &frame.repeat(STREAM_FRAMES),
        Some(SEEK_BODY),
    );
    bench_seek::<T>(group, "at_1k", &lead, Some(SEEK_BODY));
    bench_seek::<T>(group, "absent_64k", &garbage(ABSENT_LEN, T::SENTINEL), None);
}

/// Registers the `seek` arms of every sentinel length
///
/// No throughput is set: a seek reads up to the sentinel, not the buffer, so the time per seek is
/// the figure to compare
///
/// # Arguments
///
/// * `c` - The criterion driver to register the benchmark group under
fn seek(c: &mut Criterion) {
    let mut group = c.benchmark_group("seek");
    group.sample_size(SAMPLE_SIZE);
    bench_seek_case::<Seek1>(&mut group);
    bench_seek_case::<Seek4>(&mut group);
    bench_seek_case::<Seek5>(&mut group);
    bench_seek_case::<Seek8>(&mut group);
    bench_seek_case::<Seek16>(&mut group);
    group.finish();
}

criterion_group!(benches, streaming_compaction, seek);
criterion_main!(benches);
