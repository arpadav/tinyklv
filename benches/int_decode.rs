//! Regression guard for the decoders in `src/codecs/` and the BER encoders, and the record of how
//! their shapes were chosen
//!
//! * `int_decode` - the byte-array conversion behind the fixed-width decoders in
//!   `src/codecs/binary/dec.rs`
//! * `ber_decode` - the shipped `ber_length` and `ber_oid`, one arm per wire shape
//! * `ber_encode` - the shipped BER encoders, one arm per wire shape `ber_decode` reads
//! * `sliced_exact` - a fixed-width read inside a `size(exact = N)` frame, the shipped decoder
//!   against a from-slice read
//!
//! `int_decode` has two arms per precision, both doing the identical `winnow::token::take(N)` read
//! and differing only in how the resulting `&[u8]` of known length `N` becomes a `[u8; N]`:
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
use tinyklv::{
    dec::{ber as decber, binary as decb},
    enc::ber as encber,
};
// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{
    BenchmarkGroup, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use std::{hint::black_box, iter, time::Duration};
use winnow::{
    Parser,
    error::{ContextError, ParserError},
    token::take,
};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Number of distinct byte patterns decoded per measured unit, sized so each sample times many
/// sub-nanosecond conversions rather than a single one
const FIXTURE_LEN: usize = 256;

/// Criterion samples per benchmark; far above the default 100 so sub-ns deltas clear the noise floor
const SAMPLE_SIZE: usize = 1000;

/// Bit 8 of a BER byte: set on a long-form length's first byte (X.690 §8.1.3.5 a)), and on a
/// BER-OID byte another follows (§8.19.2)
const BIT_8: u8 = 0x80;

/// Bits 7 to 1 of a BER byte: a short-form length (X.690 §8.1.3.4), one BER-OID group (§8.19.2)
const BITS_7_TO_1: u8 = !BIT_8;

/// The value bits one BER-OID byte carries (X.690 §8.19.2)
const GROUP_BITS: u32 = BITS_7_TO_1.count_ones();

/// The lowest bit of a byte, set so a fixture's leading length byte or first group is never
/// zero and its form stays the fewest bytes
const LOW_BIT: u8 = 1;

/// A long form's first byte counting one length byte (X.690 §8.1.3.5)
const ONE_LENGTH_BYTE: u8 = BIT_8 | 1;

/// A long form's first byte counting two length bytes (X.690 §8.1.3.5)
const TWO_LENGTH_BYTES: u8 = BIT_8 | 2;

/// The bytes of a BER-OID run around its middle continuation bytes: its first and its terminator
const RUN_ENDS: usize = 2;

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
        bench_arm::<$ty>(
            $group,
            concat!("shipped_", stringify!($ty)),
            $fixture,
            $n,
            $shipped,
        );
        bench_arm::<$ty>(
            $group,
            concat!("checked_", stringify!($ty)),
            $fixture,
            $n,
            |input: &mut &[u8]| {
                let bytes =
                    take::<usize, &[u8], winnow::error::ContextError>($n).parse_next(input)?;
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
    let mut group = configured_group(c, "int_decode");
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

/// Times the shipped `ber_length` and `ber_oid` decoders, one arm per wire shape
///
/// Each arm reads a stream of [`FIXTURE_LEN`] wires of one shape back to back, so the same
/// file times the decoders of any tree that ships them
///
/// # Arguments
///
/// * `c` - The criterion driver to register the benchmark group under
fn ber_decode(c: &mut Criterion) {
    let mut group = configured_group(c, "ber_decode");
    bench_stream(
        &mut group,
        "ber_length/1",
        &stream(|i| vec![i & BITS_7_TO_1]),
        decber::ber_length,
    );
    bench_stream(
        &mut group,
        "ber_length/2",
        &stream(|i| vec![ONE_LENGTH_BYTE, BIT_8 | i]),
        decber::ber_length,
    );
    bench_stream(
        &mut group,
        "ber_length/3",
        &stream(|i| vec![TWO_LENGTH_BYTES, LOW_BIT | i, i]),
        decber::ber_length,
    );
    bench_stream(
        &mut group,
        "ber_oid_u64/1",
        &stream(|i| vec![i & BITS_7_TO_1]),
        decber::ber_oid::<u64>,
    );
    bench_stream(
        &mut group,
        "ber_oid_u64/2",
        &stream(|i| vec![BIT_8 | i, i & BITS_7_TO_1]),
        decber::ber_oid::<u64>,
    );
    // --------------------------------------------------
    // the longest runs, their first group as wide as the type allows
    // --------------------------------------------------
    bench_stream(
        &mut group,
        "ber_oid_u64/10",
        &stream(|i| oid_run(BIT_8 | top_group(u64::BITS), run_bytes(u64::BITS), i)),
        decber::ber_oid::<u64>,
    );
    bench_stream(
        &mut group,
        "ber_oid_u128/19",
        &stream(|i| {
            oid_run(
                BIT_8 | (i & top_group(u128::BITS)),
                run_bytes(u128::BITS),
                i,
            )
        }),
        decber::ber_oid::<u128>,
    );
    group.finish();
}

/// Times the shipped `ber_length` and `ber_oid` encoders, one arm per wire shape `ber_decode`
/// reads
///
/// Each arm appends [`FIXTURE_LEN`] values of one shape back to back to one reused buffer
///
/// # Arguments
///
/// * `c` - The criterion driver to register the benchmark group under
fn ber_encode(c: &mut Criterion) {
    let mut group = configured_group(c, "ber_encode");
    bench_values(
        &mut group,
        "ber_length/1",
        &values(|i| usize::from(i & BITS_7_TO_1)),
        encber::ber_length,
    );
    bench_values(
        &mut group,
        "ber_length/2",
        &values(|i| usize::from(BIT_8 | i)),
        encber::ber_length,
    );
    bench_values(
        &mut group,
        "ber_length/3",
        &values(|i| usize::from(u16::from_be_bytes([LOW_BIT | i, i]))),
        encber::ber_length,
    );
    bench_values(
        &mut group,
        "ber_oid_u64/1",
        &values(|i| u64::from(i & BITS_7_TO_1)),
        encber::ber_oid,
    );
    bench_values(
        &mut group,
        "ber_oid_u64/2",
        &values(|i| u64::from(BIT_8 | i)),
        encber::ber_oid,
    );
    // --------------------------------------------------
    // the longest runs, every bit of the type significant
    // --------------------------------------------------
    bench_values(
        &mut group,
        "ber_oid_u64/10",
        &values(|i| u64::MAX - u64::from(i)),
        encber::ber_oid,
    );
    bench_values(
        &mut group,
        "ber_oid_u128/19",
        &values(|i| u128::MAX - u128::from(i)),
        encber::ber_oid,
    );
    group.finish();
}

/// Times a fixed-width value read inside a `size(exact = N)` frame, as the derive cuts it: the
/// shipped `be_*` decoder against a read of the whole bounded slice through `<[u8; N]>::try_from`
///
/// **Outcome:** the from-slice read is no faster. The derive's `len != N` check already bounds
/// the slice, so the optimizer drops the shipped decoder's own length check, and both arms compile
/// to the same check, load and byte swap. Without `size(exact = N)` the decoder's check is the
/// only one, so it cannot go. Lower is faster
///
/// # Arguments
///
/// * `c` - The criterion driver to register the benchmark group under
fn sliced_exact(c: &mut Criterion) {
    let mut group = configured_group(c, "sliced_exact");
    let u32_frames = stream(|i| vec![4, i, i ^ 0x5A, i, 0xA5]);
    let f64_frames = stream(|i| vec![8, 0x40, i, i, i, 0x54, 0x44, i, 0x18]);
    bench_stream(
        &mut group,
        "shipped_u32",
        &u32_frames,
        exact_frame(4, decb::be_u32),
    );
    bench_stream(
        &mut group,
        "from_slice_u32",
        &u32_frames,
        exact_frame(4, |value: &mut &[u8]| {
            let array = <[u8; 4]>::try_from(*value).map_err(|_| ContextError::from_input(value))?;
            Ok(u32::from_be_bytes(array))
        }),
    );
    bench_stream(
        &mut group,
        "shipped_f64",
        &f64_frames,
        exact_frame(8, decb::be_f64),
    );
    bench_stream(
        &mut group,
        "from_slice_f64",
        &f64_frames,
        exact_frame(8, |value: &mut &[u8]| {
            let array = <[u8; 8]>::try_from(*value).map_err(|_| ContextError::from_input(value))?;
            Ok(f64::from_be_bytes(array))
        }),
    );
    group.finish();
}

/// Creates a criterion group with [`SAMPLE_SIZE`] samples and extended warm-up and measurement
/// windows
///
/// # Arguments
///
/// * `c` - The criterion driver to register the group under
/// * `name` - The group's name
///
/// # Returns
///
/// The configured group
fn configured_group<'c>(c: &'c mut Criterion, name: &str) -> BenchmarkGroup<'c, WallTime> {
    let mut group = c.benchmark_group(name);
    group.sample_size(SAMPLE_SIZE);
    group.warm_up_time(Duration::from_secs(3));
    group.measurement_time(Duration::from_secs(10));
    group
}

/// Concatenates [`FIXTURE_LEN`] wires of one shape, each built from its index
///
/// # Arguments
///
/// * `wire` - Builds the wire at an index
///
/// # Returns
///
/// The wires, back to back
fn stream(wire: impl Fn(u8) -> Vec<u8>) -> Vec<u8> {
    (0..FIXTURE_LEN).flat_map(|i| wire(i as u8)).collect()
}

/// Builds [`FIXTURE_LEN`] values of one shape, each from its index
///
/// # Arguments
///
/// * `value` - Builds the value at an index
///
/// # Returns
///
/// The values
fn values<T>(value: impl Fn(u8) -> T) -> Vec<T> {
    (0..FIXTURE_LEN).map(|i| value(i as u8)).collect()
}

/// The bytes in the longest BER-OID run a `bits`-wide type holds: one per started group
/// (X.690 §8.19.2)
///
/// # Arguments
///
/// * `bits` - The type's width
///
/// # Returns
///
/// The run's length, terminator included
const fn run_bytes(bits: u32) -> usize {
    bits.div_ceil(GROUP_BITS) as usize
}

/// The largest first group of the longest BER-OID run a `bits`-wide type holds: the bits its
/// other groups leave
///
/// # Arguments
///
/// * `bits` - The type's width
///
/// # Returns
///
/// The group, its value bits all set
const fn top_group(bits: u32) -> u8 {
    let top_bits = bits - (bits.div_ceil(GROUP_BITS) - 1) * GROUP_BITS;
    (1 << top_bits) - 1
}

/// A BER-OID run of `len` bytes: `first`, continuation bytes built from `i`, then a terminator
///
/// # Arguments
///
/// * `first` - The run's first byte, a continuation byte carrying its first group
/// * `len` - The run's length, terminator included
/// * `i` - The index the middle groups and the terminator are built from
///
/// # Returns
///
/// The run
fn oid_run(first: u8, len: usize, i: u8) -> Vec<u8> {
    iter::once(first)
        .chain(iter::repeat_n(BIT_8 | i, len - RUN_ENDS))
        .chain(iter::once(i & BITS_7_TO_1))
        .collect()
}

/// Wraps a value decoder in the derive's arm for a `size(exact = N)` field: the frame's one-byte
/// length, its bytes cut off the input, the length checked against `N`, then the value read from
/// the cut bytes
///
/// # Arguments
///
/// * `width` - The `N` of `size(exact = N)`
/// * `value` - Reads the value from the cut bytes
///
/// # Returns
///
/// A decoder of one frame
fn exact_frame<T>(
    width: usize,
    value: impl Fn(&mut &[u8]) -> winnow::Result<T>,
) -> impl Fn(&mut &[u8]) -> winnow::Result<T> {
    move |input| {
        let len = usize::from(decb::u8(input)?);
        let mut cut: &[u8] = take(len).parse_next(input)?;
        if len != width {
            return Err(ContextError::from_input(input));
        }
        value(&mut cut)
    }
}

/// Times a decoder over a stream of wires, read back to back until the stream is consumed
///
/// The stream is checked to decode in full before it is timed, so no arm stops early on a bad
/// fixture. It is black-boxed once outside the timed loop, as in [`bench_arm`]
///
/// # Arguments
///
/// * `group` - The criterion group to register under
/// * `name` - The benchmark id
/// * `stream` - The wires, back to back
/// * `decode` - The decoder under test
///
/// # Panics
///
/// If `decode` fails anywhere in `stream`
fn bench_stream<T>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    stream: &[u8],
    decode: impl Fn(&mut &[u8]) -> winnow::Result<T>,
) {
    let mut check = stream;
    while !check.is_empty() {
        assert!(
            decode(&mut check).is_ok(),
            "{name}: the fixture must decode"
        );
    }
    let stream = black_box(stream);
    group.bench_function(name, |b| {
        b.iter(|| {
            let mut input = stream;
            while !input.is_empty() {
                let Ok(value) = decode(&mut input) else {
                    break;
                };
                black_box(value);
            }
        });
    });
}

/// Times an encoder over values appended back to back to one reused buffer
///
/// The values are encoded once before they are timed, so the buffer already holds every byte and
/// no timed pass grows it. They are black-boxed once outside the timed loop, as in
/// [`bench_stream`]
///
/// # Arguments
///
/// * `group` - The criterion group to register under
/// * `name` - The benchmark id
/// * `values` - The values to encode
/// * `encode` - The encoder under test
fn bench_values<T: Copy>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    name: &str,
    values: &[T],
    encode: impl Fn(T, &mut Vec<u8>),
) {
    let mut out = Vec::new();
    values.iter().for_each(|&value| encode(value, &mut out));
    let values = black_box(values);
    group.bench_function(name, |b| {
        b.iter(|| {
            out.clear();
            values.iter().for_each(|&value| encode(value, &mut out));
            black_box(&out);
        });
    });
}

criterion_group!(benches, int_decode, ber_decode, ber_encode, sliced_exact);
criterion_main!(benches);
