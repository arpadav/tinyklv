//! Benchmark scaffold: the approach traits and the criterion driver.
//!
//! Each approach is a zero-sized unit struct implementing [`Approach`] (its identity /
//! criterion bar label) and [`Codec<R>`] for each record type `R` (how it round-trips
//! that record). The [`run`] driver builds four criterion groups per record shape, each
//! covering all four approaches, and gates on [`checks::verify`] before timing anything.
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
use std::hint::black_box;
use criterion::measurement::WallTime;
use criterion::{BenchmarkGroup, Criterion};
use rand::rngs::SmallRng;
use rand::SeedableRng;

/// Shared seed for sample generation. Each `reg_*` function creates its own [`SmallRng`]
/// from this seed so every approach sees an identical sequence of random samples.
const RNG_SEED: [u8; 32] = [0x5A; 32];

/// Identity of a benchmarked approach: the criterion bar label. Declared once per
/// approach (not once per record), so a typo can't silently mislabel a bar.
pub(crate) trait Approach {
    /// The criterion function name this approach reports under.
    const NAME: &'static str;
}

/// How an [`Approach`] round-trips one record type `R`. Four benched operations in a
/// fixed order: clean encode, clean decode, framed encode, framed decode.
///
/// `encode_framed` / `decode_framed` default to the shared "seek" path that serde_klv,
/// tlv_parser, and manual all need: wrap a value body in a sentinel frame, then scan
/// past stream noise back to it. tinyklv overrides BOTH with its native `encode_frame` /
/// `decode_frame`. Invariant: override both or neither, so the framed encode and the
/// framed decode agree on the wire envelope. The seek envelope (sentinel + 1-byte length)
/// is identical for every approach - only the native-vs-shared seek differs, which is
/// exactly what the framed groups measure.
pub(crate) trait Codec<R>: Approach {
    /// Encodes the value body (no frame).
    fn encode(rec: &R) -> Vec<u8>;

    /// Decodes the value body; `None` on malformed input.
    fn decode(body: &[u8]) -> Option<R>;

    /// Encodes a full frame (sentinel + length + value body), no stream noise.
    fn encode_framed(rec: &R) -> Vec<u8> {
        framing::frame(&Self::encode(rec))
    }

    /// Seeks past stream noise to the frame, then decodes the value body.
    fn decode_framed(noisy: &[u8]) -> Option<R> {
        framing::seek(noisy).and_then(Self::decode)
    }
}

/// Benches `A::decode` over `A`'s own clean value body.
///
/// Generates a fresh sample via `R::rng_sample` on every iteration so the decoder
/// is exercised on varied data, not a single canned fixture.
fn reg_decode<A, R>(group: &mut BenchmarkGroup<'_, WallTime>)
where
    A: Codec<R>,
    R: RngSample + core::fmt::Debug,
{
    group.bench_function(A::NAME, |b| {
        let mut rng = black_box(SmallRng::from_seed(RNG_SEED));
        b.iter(|| {
            let body = black_box({
                let sample = R::rng_sample(&mut rng);
                A::encode(&sample)
            });
            A::decode(black_box(body.as_slice())).unwrap()
        });
    });
}

/// Benches `A::decode_framed` over `A`'s own frame embedded in stream noise.
fn reg_decode_framed<A, R>(group: &mut BenchmarkGroup<'_, WallTime>)
where
    A: Codec<R>,
    R: RngSample + core::fmt::Debug,
{
    group.bench_function(A::NAME, |b| {
        let mut rng = black_box(SmallRng::from_seed(RNG_SEED));
        b.iter(|| {
            let noisy = black_box({
                let sample = R::rng_sample(&mut rng);
                framing::make_noisy(&A::encode_framed(&sample))
            });
            A::decode_framed(black_box(noisy.as_slice())).unwrap()
        });
    });
}

/// Benches `A::encode` (clean value body).
fn reg_encode<A, R>(group: &mut BenchmarkGroup<'_, WallTime>)
where
    A: Codec<R>,
    R: RngSample,
{
    group.bench_function(A::NAME, |b| {
        let mut rng = black_box(SmallRng::from_seed(RNG_SEED));
        b.iter(|| {
            let sample = black_box(R::rng_sample(&mut rng));
            A::encode(black_box(&sample))
        });
    });
}

/// Benches `A::encode_framed` (sentinel + length + value body).
fn reg_encode_framed<A, R>(group: &mut BenchmarkGroup<'_, WallTime>)
where
    A: Codec<R>,
    R: RngSample,
{
    group.bench_function(A::NAME, |b| {
        let mut rng = black_box(SmallRng::from_seed(RNG_SEED));
        b.iter(|| {
            let sample = black_box(R::rng_sample(&mut rng));
            A::encode_framed(black_box(&sample))
        });
    });
}

/// Registers every approach under one criterion group via the given `reg_*` fn. This is
/// the single place the approach roster is enumerated, so adding an approach is a one-line
/// edit here rather than a rename of count-baked macros.
macro_rules! for_each_approach {
    ($reg:ident, $group:expr) => {{
        $reg::<Tinyklv, _>($group);
        $reg::<SerdeKlv, _>($group);
        $reg::<TlvParser, _>($group);
        $reg::<Manual, _>($group);
        $reg::<Prost, _>($group);
        $reg::<QuickProtobuf, _>($group);
        $reg::<RustProtobuf, _>($group);
        $reg::<Micropb, _>($group);
    }};
}

/// Builds the four criterion groups for one record shape, every group covering all four
/// approaches. Each benchmark creates its own [`SmallRng`] seeded from [`RNG_SEED`] so data
/// is deterministic run-to-run while still producing varied values across iterations.
/// Asserts cross-approach equivalence before timing anything.
///
/// # Arguments
///
/// * `c` - the criterion harness.
/// * `label` - the record-shape prefix for group names (`"flat"` / `"nested"`).
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
    // seed rng and assert cross-approach equivalence
    // --------------------------------------------------
    let mut rng = SmallRng::from_seed(RNG_SEED);
    let sample = R::rng_sample(&mut rng);
    checks::verify(&sample);
    // --------------------------------------------------
    // decode clean group (fresh sample per iteration)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_decode_clean"));
    for_each_approach!(reg_decode, &mut group);
    group.finish();
    // --------------------------------------------------
    // decode framed group (fresh sample per iteration)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_decode_framed"));
    for_each_approach!(reg_decode_framed, &mut group);
    group.finish();
    // --------------------------------------------------
    // encode clean group (fresh sample per iteration)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_encode_clean"));
    for_each_approach!(reg_encode, &mut group);
    group.finish();
    // --------------------------------------------------
    // encode framed group (fresh sample per iteration)
    // --------------------------------------------------
    let mut group = c.benchmark_group(format!("{label}_encode_framed"));
    for_each_approach!(reg_encode_framed, &mut group);
    group.finish();
}
