//! Benchmark scaffold: the approach traits and the criterion driver
//!
//! Each approach is a zero-sized unit struct implementing [`Approach`] (its identity /
//! criterion bar label) and [`Codec<R>`] for each record type `R` (how it round-trips
//! that record). The [`run`] driver builds five criterion groups per record shape, each
//! covering all approaches, and gates on [`checks::verify`] before timing anything
//!
//! Tiers per record shape:
//!
//! * `value` - encode / decode the bare value body (no frame)
//! * `frame` - encode / decode one full frame (sentinel + length + body), no stream noise
//! * `streamed` - decode-only: a [`framing::make_stream`] buffer of many noisy framed packets,
//!   decoded back to back. tinyklv runs this through its streaming `Decoder`; the others loop the
//!   shared seek path. This is the tier the decoder's buffer-reclamation cost shows up in
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub(crate) mod approaches;
pub(crate) mod checks;
pub(crate) mod framing;
pub(crate) mod records;

// --------------------------------------------------
// local
// --------------------------------------------------
use approaches::{
    Manual, Micropb, Prost, QuickProtobuf, RustProtobuf, SerdeKlv, Tinyklv, TlvParser,
};
use records::RngSample;

// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{BenchmarkGroup, Criterion, Throughput, measurement::WallTime};
use rand::{SeedableRng, rngs::SmallRng};
use std::hint::black_box;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Shared seed for sample generation. [`run`] draws one canonical sample from this seed per record
/// shape, so the timed inputs are deterministic run-to-run and identical for every approach
const RNG_SEED: [u8; 32] = [0x5A; 32];

/// Samples per criterion estimate. Raised well above the default so the per-bar confidence
/// intervals are tight enough to trust the cross-approach ordering
const SAMPLE_SIZE: usize = 1000;

/// Identity of a benchmarked approach: the criterion bar label. Declared once per
/// approach (not once per record), so a typo can't silently mislabel a bar
pub(crate) trait Approach {
    /// The criterion function name this approach reports under
    const NAME: &'static str;
}

/// How an [`Approach`] round-trips one record type `R`
///
/// `encode` / `decode` move the bare value body. `encode_framed` / `decode_framed` wrap that body
/// in a sentinel + 1-byte-length frame and (on decode) seek to it; `decode_streamed` pulls every
/// packet out of a multi-packet noisy buffer
///
/// The framed and streamed defaults are the shared "seek" path that serde_klv, tlv_parser, and
/// manual all need. tinyklv overrides `encode_framed` / `decode_framed` with its native
/// `encode_frame` / `decode_frame`, and `decode_streamed` with its streaming `Decoder`. Invariant:
/// the seek envelope (sentinel + 1-byte length) is identical for every approach - only the
/// native-vs-shared seek differs, which is exactly what the framed and streamed groups measure
pub(crate) trait Codec<R>: Approach {
    /// Encodes the value body (no frame)
    fn encode(rec: &R) -> Vec<u8>;

    /// Decodes the value body; `None` on malformed input
    fn decode(body: &[u8]) -> Option<R>;

    /// Encodes a full frame (sentinel + length + value body), no stream noise
    fn encode_framed(rec: &R) -> Vec<u8> {
        framing::frame(&Self::encode(rec))
    }

    /// Seeks to the frame, then decodes the value body
    fn decode_framed(frame: &[u8]) -> Option<R> {
        framing::seek(frame).and_then(Self::decode)
    }

    /// Decodes every packet from a multi-packet noisy buffer (e.g. [`framing::make_stream`])
    ///
    /// The default loops the shared seek path: scan to the next sentinel, slice its body, decode,
    /// repeat until the buffer is exhausted. tinyklv overrides this to drive its streaming
    /// `Decoder` instead, which is what the `streamed` tier exists to compare
    fn decode_streamed(buf: &[u8]) -> Option<Vec<R>> {
        let mut out = Vec::new();
        let mut rest = buf;
        while let Some((body, next)) = framing::seek_next(rest) {
            out.push(Self::decode(body)?);
            rest = next;
        }
        Some(out)
    }
}

/// Benches `A::decode` over `A`'s own clean value body
///
/// The body is encoded once from the shared canonical `sample` and reused across iterations, so
/// the timed closure measures decode alone - not the per-iteration encode it used to fold in
///
/// # Arguments
///
/// * `group` - the criterion benchmark group to register the function under
/// * `sample` - the canonical record whose encoded body is decoded on every iteration
fn reg_decode<A, R>(group: &mut BenchmarkGroup<'_, WallTime>, sample: &R)
where
    A: Codec<R>,
    R: core::fmt::Debug,
{
    let body = A::encode(sample);
    group.throughput(Throughput::Bytes(body.len() as u64));
    group.bench_function(A::NAME, |b| {
        b.iter(|| A::decode(black_box(body.as_slice())).unwrap());
    });
}

/// Benches `A::decode_framed` over `A`'s own clean frame (sentinel + length + body, no noise)
///
/// The frame is built once from `sample` via `A::encode_framed` and reused on every iteration,
/// isolating the decode-and-seek cost from encoding overhead
///
/// # Arguments
///
/// * `group` - the criterion benchmark group to register the function under
/// * `sample` - the canonical record whose framed encoding is decoded on every iteration
fn reg_decode_framed<A, R>(group: &mut BenchmarkGroup<'_, WallTime>, sample: &R)
where
    A: Codec<R>,
    R: core::fmt::Debug,
{
    let frame = A::encode_framed(sample);
    group.throughput(Throughput::Bytes(frame.len() as u64));
    group.bench_function(A::NAME, |b| {
        b.iter(|| A::decode_framed(black_box(frame.as_slice())).unwrap());
    });
}

/// Benches `A::decode_streamed` over a multi-packet noisy buffer of `A`'s own frame
///
/// The buffer holds [`framing::STREAM_PACKETS`] back-to-back noisy frames, built once and reused
/// tinyklv decodes it through its streaming `Decoder` (feed + iterate), the others loop the shared
/// seek path - so this is the tier the per-packet buffer-reclamation cost surfaces in
///
/// # Arguments
///
/// * `group` - the criterion benchmark group to register the function under
/// * `sample` - the canonical record used to build the multi-packet stream buffer
fn reg_decode_streamed<A, R>(group: &mut BenchmarkGroup<'_, WallTime>, sample: &R)
where
    A: Codec<R>,
    R: core::fmt::Debug,
{
    let stream = framing::make_stream(&A::encode_framed(sample));
    group.throughput(Throughput::Bytes(stream.len() as u64));
    group.bench_function(A::NAME, |b| {
        b.iter(|| A::decode_streamed(black_box(stream.as_slice())).unwrap());
    });
}

/// Benches `A::encode` (clean value body) over the shared canonical `sample`
///
/// Encodes `sample` on every iteration with no decoding involved, so the number
/// represents the pure serialization cost for the value body alone
///
/// # Arguments
///
/// * `group` - the criterion benchmark group to register the function under
/// * `sample` - the canonical record encoded on every iteration
fn reg_encode<A, R>(group: &mut BenchmarkGroup<'_, WallTime>, sample: &R)
where
    A: Codec<R>,
{
    group.throughput(Throughput::Bytes(A::encode(sample).len() as u64));
    group.bench_function(A::NAME, |b| {
        b.iter(|| A::encode(black_box(sample)));
    });
}

/// Benches `A::encode_framed` (sentinel + length + value body) over the shared canonical `sample`
///
/// Encodes `sample` into a full frame on every iteration, measuring the combined cost of value
/// serialization plus the sentinel + length envelope that the frame tier adds over the value tier
///
/// # Arguments
///
/// * `group` - the criterion benchmark group to register the function under
/// * `sample` - the canonical record encoded into a full frame on every iteration
fn reg_encode_framed<A, R>(group: &mut BenchmarkGroup<'_, WallTime>, sample: &R)
where
    A: Codec<R>,
{
    group.throughput(Throughput::Bytes(A::encode_framed(sample).len() as u64));
    group.bench_function(A::NAME, |b| {
        b.iter(|| A::encode_framed(black_box(sample)));
    });
}

/// Registers every approach under one criterion group via the given `reg_*` fn. This is
/// the single place the approach roster is enumerated, so adding an approach is a one-line
/// edit here rather than a rename of count-baked macros
macro_rules! for_each_approach {
    ($reg:ident, $group:expr, $sample:expr) => {{
        $reg::<Tinyklv, _>($group, $sample);
        $reg::<SerdeKlv, _>($group, $sample);
        $reg::<TlvParser, _>($group, $sample);
        $reg::<Manual, _>($group, $sample);
        $reg::<Prost, _>($group, $sample);
        $reg::<QuickProtobuf, _>($group, $sample);
        $reg::<RustProtobuf, _>($group, $sample);
        $reg::<Micropb, _>($group, $sample);
    }};
}

/// Builds the five criterion groups for one record shape, every group covering all approaches
///
/// One canonical sample is drawn from [`RNG_SEED`] and reused for every group and every approach,
/// so the timed inputs are deterministic and identical across the comparison. Cross-approach
/// equivalence is asserted via [`checks::verify`] before anything is timed
///
/// # Arguments
///
/// * `c` - the criterion harness
/// * `label` - the record-shape prefix for group names (`"simple"` / `"compound"` / `"rich"`)
pub(crate) fn run<R>(c: &mut Criterion, label: &str)
where
    R: RngSample + PartialEq + core::fmt::Debug,
    Tinyklv: Codec<R>,
    SerdeKlv: Codec<R>,
    TlvParser: Codec<R>,
    Manual: Codec<R>,
    Prost: Codec<R>,
    QuickProtobuf: Codec<R>,
    RustProtobuf: Codec<R>,
    Micropb: Codec<R>,
{
    // --------------------------------------------------
    // draw the canonical sample and assert cross-approach equivalence
    // --------------------------------------------------
    let mut rng = SmallRng::from_seed(RNG_SEED);
    let sample = R::rng_sample(&mut rng);
    checks::verify(&sample);
    // --------------------------------------------------
    // decode value group (bare body)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_decode_value"));
    group.sample_size(SAMPLE_SIZE);
    for_each_approach!(reg_decode, &mut group, &sample);
    group.finish();
    // --------------------------------------------------
    // decode frame group (single clean frame, no noise)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_decode_frame"));
    group.sample_size(SAMPLE_SIZE);
    for_each_approach!(reg_decode_framed, &mut group, &sample);
    group.finish();
    // --------------------------------------------------
    // decode streamed group (multi-packet noisy buffer; decode-only)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_decode_streamed"));
    group.sample_size(SAMPLE_SIZE);
    for_each_approach!(reg_decode_streamed, &mut group, &sample);
    group.finish();
    // --------------------------------------------------
    // encode value group (bare body)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_encode_value"));
    group.sample_size(SAMPLE_SIZE);
    for_each_approach!(reg_encode, &mut group, &sample);
    group.finish();
    // --------------------------------------------------
    // encode frame group (sentinel + length + body)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_encode_frame"));
    group.sample_size(SAMPLE_SIZE);
    for_each_approach!(reg_encode_framed, &mut group, &sample);
    group.finish();
}
