//! Regression guard for the byte-array conversion behind the fixed-width decoders in
//! `src/codecs/binary/dec.rs`, and the record of how that choice was made
//!
//! Two arms per precision, both doing the identical `winnow::token::take(N)` read and differing only
//! in how the resulting `&[u8]` of known length `N` becomes a `[u8; N]`:
//!
//! * `shipped_*` - the real `dec::binary::be_*` function as it ships (unchecked pointer cast for the
//!   <= 16-bit integer widths, checked `try_from` for the wider integers and the floats)
//! * `checked_*` - a local `<[u8; N]>::try_from(bytes).expect(..)` reference, the safe form the
//!   decision is measured against
//!
//! **Outcome (decided from this bench):** the unchecked cast wins only at <= 16-bit widths (u16/i16,
//! non-overlapping CIs). At wider integer widths (u32/u64/i32/i64/..) it no longer wins, and it
//! *regresses* `f32` (wash on `f64`). So `dec.rs` ships the unchecked cast only for the <= 16-bit
//! integer decoders (`shipped_u16/i16 < checked_u16/i16`) and keeps the checked `try_from` for the
//! wider integers and the floats. The per-decode delta is tiny (~0.01ns); this file exists to keep
//! that decision honest if the toolchain changes. Lower is faster
//!
//! Author: aav
#![allow(
    clippy::indexing_slicing,
    reason = "fixture arrays are [u8; 8] and every `width` passed in is <= 8, so `arr[..width]` is \
              always in-bounds; kept as a slice (not `.get`) so the timed loop stays branch-free"
)]
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime};
use std::{hint::black_box, time::Duration};
use winnow::{Parser, token::take};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Number of distinct byte patterns decoded per measured unit, sized so each sample times many
/// sub-nanosecond conversions rather than a single one
const FIXTURE_LEN: usize = 256;

/// Criterion samples per benchmark; far above the default 100 so sub-ns deltas clear the noise floor
const SAMPLE_SIZE: usize = 1000;

/// Builds a deterministic fixture of byte patterns wide enough for every precision under test
///
/// # Returns
///
/// A `Vec<[u8; 8]>` of [`FIXTURE_LEN`] pseudo-varied arrays; the first `N` bytes of each feed the
/// `N`-wide decoders
fn fixture() -> Vec<[u8; 8]> {
    (0..FIXTURE_LEN)
        .map(|i| {
            let mut arr = [0u8; 8];
            for (j, byte) in arr.iter_mut().enumerate() {
                *byte = (i.wrapping_mul(31).wrapping_add(j)) as u8;
            }
            arr
        })
        .collect()
}

/// Times a single decoder over the whole fixture, fencing the fixture and each decoded scalar
/// against elision
///
/// The fixture is black-boxed *once* outside the timed loop so the optimizer cannot const-fold the
/// known byte patterns, while leaving each decoder's internal `take(N)` free to specialize on its
/// const width `N`. Each decoded value `T` is black-boxed directly (not the surrounding `Result`/
/// `Option`) so the measured cost is the conversion itself, with no `Option` construct-and-discard
/// churn polluting the narrow widths
///
/// # Arguments
///
/// * `group` - The criterion group to register under
/// * `name` - The benchmark id (e.g. `shipped_u32`)
/// * `fixture` - The shared byte-pattern fixture
/// * `width` - Number of leading bytes of each pattern to decode (the precision's byte width)
/// * `decode` - The decoder under test
fn bench_arm<T>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    fixture: &[[u8; 8]],
    width: usize,
    decode: impl Fn(&mut &[u8]) -> winnow::Result<T>,
) {
    let fixture = black_box(fixture);
    group.bench_function(name, |b| {
        b.iter(|| {
            for arr in fixture {
                let mut input: &[u8] = &arr[..width];
                if let Ok(value) = decode(&mut input) {
                    black_box(value);
                }
            }
        });
    });
}

/// Registers the `shipped_<ty>` (real `dec` function) and `checked_<ty>` (local reference) arm
/// for one big-endian precision inside a criterion benchmark group
///
/// Calls [`bench_arm`] twice: once with the shipped decoder path (the function exported from
/// `tinyklv::dec::binary`) and once with an inline checked reference that calls
/// `<[u8; N]>::try_from(bytes).expect(..)`. The two benchmark ids are
/// `shipped_<ty>` and `checked_<ty>` respectively, so criterion produces a side-by-side
/// comparison without any manual naming
///
/// # Arguments
///
/// * `$group` - The mutable [`BenchmarkGroup`] to register both arms under
/// * `$fixture` - A reference to the shared `&[[u8; 8]]` byte-pattern fixture
/// * `$ty` - The Rust primitive type being decoded (e.g. `u32`, `f64`)
/// * `$n` - The byte width of the precision as a literal (e.g. `4` for `u32`)
/// * `$shipped` - Path to the shipped big-endian decoder (e.g. `decb::be_u32`)
macro_rules! bench_ty {
    ($group:expr, $fixture:expr, $ty:ty, $n:literal, $shipped:path) => {{
        bench_arm::<$ty>($group, concat!("shipped_", stringify!($ty)), $fixture, $n, $shipped);
        bench_arm::<$ty>(
            $group,
            concat!("checked_", stringify!($ty)),
            $fixture,
            $n,
            |input: &mut &[u8]| {
                let bytes = take::<usize, &[u8], winnow::error::ContextError>($n).parse_next(input)?;
                let array = <[u8; $n]>::try_from(bytes).expect("take(N) yields exactly N bytes");
                Ok(<$ty>::from_be_bytes(array))
            },
        );
    }};
}

/// Sweeps shipped-vs-checked big-endian conversion across the seven multi-byte precisions
///
/// Creates a single criterion group named `int_decode`, raises the sample count to
/// [`SAMPLE_SIZE`], extends warm-up and measurement windows, then invokes [`bench_ty!`]
/// for `u16`, `u32`, `u64`, `i16`, `i32`, `f32`, and `f64`. Single-byte widths (`u8`/`i8`)
/// are excluded because a 1-byte `from_be_bytes` is a pure register move with nothing to compare
///
/// # Arguments
///
/// * `c` - The criterion driver to register the benchmark group under
fn int_decode(c: &mut Criterion) {
    let fx = fixture();
    let mut group = c.benchmark_group("int_decode");
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));
    // --------------------------------------------------
    // skip u8/i8: a 1-byte `from_be_bytes` is a pure move, nothing to compare
    // --------------------------------------------------
    bench_ty!(&mut group, &fx, u16, 2, decb::be_u16);
    bench_ty!(&mut group, &fx, u32, 4, decb::be_u32);
    bench_ty!(&mut group, &fx, u64, 8, decb::be_u64);
    bench_ty!(&mut group, &fx, i16, 2, decb::be_i16);
    bench_ty!(&mut group, &fx, i32, 4, decb::be_i32);
    bench_ty!(&mut group, &fx, f32, 4, decb::be_f32);
    bench_ty!(&mut group, &fx, f64, 8, decb::be_f64);
    group.finish();
}

criterion_group!(benches, int_decode);
criterion_main!(benches);
