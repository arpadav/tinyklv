//! The decode cost of two framing choices, each measured between structs that differ only in that
//! choice: a varint value with a length prefix against one without, and opting in to `unknown(..)`
//!
//! Every benchmark is a one-shot `decode_value` over a wire encoded ahead of time, checked to
//! decode to its expected value with every byte read before anything is timed:
//!
//! * `varint_decode_value` - four varints (`u32`, `u32`, `i32`, `u64`) under one-byte varint keys.
//!   [`PrefixedVarints`] frames each as key, length, value; [`ImplicitVarints`], under
//!   `size(implicit)`, as key, value, the bytes protobuf writes for them. The two differ by the
//!   length read and the `take` it bounds
//! * `unknown_decode_value` - eight fixed-width fields in key + len mode, a one-byte key and a
//!   one-byte length. [`FixedWidths`] has no `unknown(..)`, [`FixedWidthsDroppingUnknown`] has it
//!   and no `#[klv(unknown)]` field, [`FixedWidthsKeepingUnknown`] keeps the frames in one. All
//!   three decode the wire with no unknown frame; the two that opt in also decode it with two
//!   unknown frames after the known ones
//!
//! Lower is faster
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use criterion::{
    BenchmarkGroup, BenchmarkId, Criterion, criterion_group, criterion_main, measurement::WallTime,
};
use std::{any::type_name, fmt::Debug, hint::black_box};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Criterion samples per benchmark, as in `benches/suite/mod.rs`: far above the default 100, so
/// the confidence intervals are tight enough to order arms a few nanoseconds apart
const SAMPLE_SIZE: usize = 1000;

/// Bench id parameter of [`FIXED_WIDTHS_WIRE`]
const KNOWN_ONLY: &str = "known_only";

/// Bench id parameter of [`FIXED_WIDTHS_WIRE`] followed by [`UNKNOWN_FRAMES`]
const TWO_UNKNOWN: &str = "two_unknown";

/// The bytes of [`FixedWidths::DECODED`]: each field after its key and its one-byte length
#[rustfmt::skip]
const FIXED_WIDTHS_WIRE: &[u8] = &[
    // channel: key, length 1, 3
    0x01, 0x01, 0x03,
    // sequence: key, length 2, 1_042
    0x02, 0x02, 0x04, 0x12,
    // uptime: key, length 4, 86_400
    0x03, 0x04, 0x00, 0x01, 0x51, 0x80,
    // timestamp: key, length 8, 1_760_000_000_000_000
    0x04, 0x08, 0x00, 0x06, 0x40, 0xB5, 0xEE, 0xCE, 0x00, 0x00,
    // temperature: key, length 2, -1_250
    0x05, 0x02, 0xFB, 0x1E,
    // altitude: key, length 4, 152_400
    0x06, 0x04, 0x00, 0x02, 0x53, 0x50,
    // voltage: key, length 4, 3.3
    0x07, 0x04, 0x40, 0x53, 0x33, 0x33,
    // latitude: key, length 8, 47.4979
    0x08, 0x08, 0x40, 0x47, 0xBF, 0xBB, 0x2F, 0xEC, 0x56, 0xD6,
];

/// Two frames whose keys no field declares, as [`FixedWidthsKeepingUnknown::with_unknown_frames`]
/// keeps them
#[rustfmt::skip]
const UNKNOWN_FRAMES: &[u8] = &[
    // key 0x20, length 3
    0x20, 0x03, 0x01, 0x04, 0x02,
    // key 0x21, length 2
    0x21, 0x02, 0x80, 0x01,
];

/// LEB128 varint decoders: seven value bits per byte, low group first, the high bit set on every
/// byte but the last
mod varint {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use tinyklv::Result;
    use tinyklv::prelude::*;

    // --------------------------------------------------
    // external
    // --------------------------------------------------
    use winnow::error::{ContextError, ParserError};

    // --------------------------------------------------
    // constants
    // --------------------------------------------------
    /// The bit of a varint byte that says another byte follows
    const CONTINUATION: u8 = 0x80;

    /// Value bits each varint byte carries
    const PAYLOAD_BITS: u32 = 7;

    /// The shift of a 10th varint byte, which holds only bit 63 of a `u64`
    const LAST_SHIFT: u32 = 63;

    /// Decodes one varint as a `u64`
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced past the varint on success, to the end
    ///   of the input when it ends first, and left on a 10th byte carrying bits past the 64th
    ///
    /// # Returns
    ///
    /// `Ok(u64)`, or an error on truncation or on a 10th byte carrying bits past the 64th
    pub(super) fn dec_u64(input: &mut &[u8]) -> Result<u64> {
        let mut value = 0;
        let mut shift = 0;
        loop {
            let Some((&byte, rest)) = input.split_first() else {
                return Err(ContextError::from_input(input));
            };
            if shift == LAST_SHIFT && byte > 1 {
                return Err(ContextError::from_input(input));
            }
            *input = rest;
            value |= u64::from(byte & !CONTINUATION) << shift;
            if byte & CONTINUATION == 0 {
                return Ok(value);
            }
            shift += PAYLOAD_BITS;
        }
    }

    /// Decodes one varint as a `u32`, a key or a counter
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced as [`dec_u64`] advances it, and
    ///   rewound to the varint's start when the value does not fit a `u32`
    ///
    /// # Returns
    ///
    /// `Ok(u32)`, or the error of [`dec_u64`], or an error on a value past `u32::MAX`
    pub(super) fn dec_u32(input: &mut &[u8]) -> Result<u32> {
        dec_u64.try_map(u32::try_from).parse_next(input)
    }

    /// Decodes one varint as a `usize`, a value length
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced as [`dec_u64`] advances it, and
    ///   rewound to the varint's start when the value does not fit a `usize`
    ///
    /// # Returns
    ///
    /// `Ok(usize)`, or the error of [`dec_u64`], or an error on a value past `usize::MAX`
    pub(super) fn dec_usize(input: &mut &[u8]) -> Result<usize> {
        dec_u64.try_map(usize::try_from).parse_next(input)
    }

    /// Decodes one varint as an `i32`: the low 32 bits of the `u64`, so a negative value's
    /// ten-byte sign extension reads back
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream to read from; advanced as [`dec_u64`] advances it
    ///
    /// # Returns
    ///
    /// `Ok(i32)`, or the error of [`dec_u64`]
    pub(super) fn dec_i32(input: &mut &[u8]) -> Result<i32> {
        dec_u64.map(|value| value as i32).parse_next(input)
    }
}

/// The codec of a frame no field declares, in a struct whose every frame carries a one-byte
/// length: the value is its bytes
mod raw {
    // --------------------------------------------------
    // local
    // --------------------------------------------------
    use tinyklv::Result;
    use tinyklv::dec::binary as decb;
    use tinyklv::enc::binary as encb;
    use tinyklv::prelude::*;

    // --------------------------------------------------
    // external
    // --------------------------------------------------
    use winnow::error::{ContextError, ParserError};

    /// Decodes the value of a key + len frame: everything after its key
    ///
    /// # Arguments
    ///
    /// * `_key` - the frame's key; every frame carries a length, so the key does not decide how
    ///   the value is framed
    ///
    /// # Returns
    ///
    /// A decoder reading the length, then that many bytes; it fails on a missing length, or with
    /// the input consumed to its end when fewer bytes remain, so a decode loop reads truncation
    pub(super) fn dec(_key: u8) -> impl Fn(&mut &[u8]) -> Result<Vec<u8>> {
        |input| {
            let len = decb::u8_as_usize(input)?;
            let Some((value, rest)) = input.split_at_checked(len) else {
                input.finish();
                return Err(ContextError::from_input(input));
            };
            *input = rest;
            Ok(value.to_vec())
        }
    }

    /// Encodes the value of a key + len frame, the inverse of [`dec`]: its length, then its bytes
    ///
    /// # Arguments
    ///
    /// * `value` - the bytes to encode
    /// * `out` - the buffer the length and bytes are appended to
    pub(super) fn enc(value: &[u8], out: &mut Vec<u8>) {
        encb::u8_from_usize(value.len(), out);
        out.extend_from_slice(value);
    }
}

/// A radio link's counters, each varint after its key and its length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    default(typ = u32, dec = varint::dec_u32),
    default(typ = i32, dec = varint::dec_i32),
    default(typ = u64, dec = varint::dec_u64),
    allow_unimplemented_encode,
)]
struct PrefixedVarints {
    /// Packets sent
    #[klv(key = 0x08)]
    packets: u32,

    /// Packets sent again
    #[klv(key = 0x10)]
    retries: u32,

    /// Received signal strength, dBm
    #[klv(key = 0x18)]
    rssi: i32,

    /// Bytes sent
    #[klv(key = 0x20)]
    transferred: u64,
}

/// [`PrefixedVarints`] implementation
impl PrefixedVarints {
    /// The bytes of [`Self::DECODED`]
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // packets: key, length 2, 1_500
        0x08, 0x02, 0xDC, 0x0B,
        // retries: key, length 1, 3
        0x10, 0x01, 0x03,
        // rssi: key, length 10, -67 sign-extended
        0x18, 0x0A, 0xBD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
        // transferred: key, length 5, 6_000_000_000
        0x20, 0x05, 0x80, 0xF8, 0x82, 0xAD, 0x16,
    ];

    /// The counters [`Self::WIRE`] carries
    const DECODED: Self = Self {
        packets: 1_500,
        retries: 3,
        rssi: -67,
        transferred: 6_000_000_000,
    };
}

/// [`PrefixedVarints`] with every varint after its key alone, delimited by its own last byte
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    default(typ = u32, dec = varint::dec_u32, size(implicit)),
    default(typ = i32, dec = varint::dec_i32, size(implicit)),
    default(typ = u64, dec = varint::dec_u64, size(implicit)),
    allow_unimplemented_encode,
)]
struct ImplicitVarints {
    /// Packets sent
    #[klv(key = 0x08)]
    packets: u32,

    /// Packets sent again
    #[klv(key = 0x10)]
    retries: u32,

    /// Received signal strength, dBm
    #[klv(key = 0x18)]
    rssi: i32,

    /// Bytes sent
    #[klv(key = 0x20)]
    transferred: u64,
}

/// [`ImplicitVarints`] implementation
impl ImplicitVarints {
    /// The bytes of [`Self::DECODED`]
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // packets: key, 1_500
        0x08, 0xDC, 0x0B,
        // retries: key, 3
        0x10, 0x03,
        // rssi: key, -67 sign-extended
        0x18, 0xBD, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01,
        // transferred: key, 6_000_000_000
        0x20, 0x80, 0xF8, 0x82, 0xAD, 0x16,
    ];

    /// The counters [`Self::WIRE`] carries
    const DECODED: Self = Self {
        packets: 1_500,
        retries: 3,
        rssi: -67,
        transferred: 6_000_000_000,
    };
}

/// A sensor sample of eight fixed-width fields, without `unknown(..)`: a frame no field declares
/// is skipped by its length
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    default(typ = u64, dec = decb::be_u64, enc = *encb::be_u64),
    default(typ = i16, dec = decb::be_i16, enc = *encb::be_i16),
    default(typ = i32, dec = decb::be_i32, enc = *encb::be_i32),
    default(typ = f32, dec = decb::be_f32, enc = *encb::be_f32),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
struct FixedWidths {
    /// Radio channel
    #[klv(key = 0x01)]
    channel: u8,

    /// Sample sequence number
    #[klv(key = 0x02)]
    sequence: u16,

    /// Seconds since boot
    #[klv(key = 0x03)]
    uptime: u32,

    /// Microseconds since the Unix epoch
    #[klv(key = 0x04)]
    timestamp: u64,

    /// Hundredths of a degree Celsius
    #[klv(key = 0x05)]
    temperature: i16,

    /// Millimetres above sea level
    #[klv(key = 0x06)]
    altitude: i32,

    /// Supply voltage
    #[klv(key = 0x07)]
    voltage: f32,

    /// Degrees north
    #[klv(key = 0x08)]
    latitude: f64,
}

/// [`FixedWidths`] implementation
impl FixedWidths {
    /// The sample [`FIXED_WIDTHS_WIRE`] carries
    const DECODED: Self = Self {
        channel: 3,
        sequence: 1_042,
        uptime: 86_400,
        timestamp: 1_760_000_000_000_000,
        temperature: -1_250,
        altitude: 152_400,
        voltage: 3.3,
        latitude: 47.4979,
    };
}

/// [`FixedWidths`] with `unknown(..)` and no `#[klv(unknown)]` field: a frame no field declares is
/// decoded by [`raw::dec`] and dropped
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    unknown(dec = raw::dec, enc = raw::enc),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    default(typ = u64, dec = decb::be_u64, enc = *encb::be_u64),
    default(typ = i16, dec = decb::be_i16, enc = *encb::be_i16),
    default(typ = i32, dec = decb::be_i32, enc = *encb::be_i32),
    default(typ = f32, dec = decb::be_f32, enc = *encb::be_f32),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
struct FixedWidthsDroppingUnknown {
    /// Radio channel
    #[klv(key = 0x01)]
    channel: u8,

    /// Sample sequence number
    #[klv(key = 0x02)]
    sequence: u16,

    /// Seconds since boot
    #[klv(key = 0x03)]
    uptime: u32,

    /// Microseconds since the Unix epoch
    #[klv(key = 0x04)]
    timestamp: u64,

    /// Hundredths of a degree Celsius
    #[klv(key = 0x05)]
    temperature: i16,

    /// Millimetres above sea level
    #[klv(key = 0x06)]
    altitude: i32,

    /// Supply voltage
    #[klv(key = 0x07)]
    voltage: f32,

    /// Degrees north
    #[klv(key = 0x08)]
    latitude: f64,
}

/// [`FixedWidthsDroppingUnknown`] implementation
impl FixedWidthsDroppingUnknown {
    /// The sample [`FIXED_WIDTHS_WIRE`] carries, with or without [`UNKNOWN_FRAMES`] after it
    const DECODED: Self = Self {
        channel: 3,
        sequence: 1_042,
        uptime: 86_400,
        timestamp: 1_760_000_000_000_000,
        temperature: -1_250,
        altitude: 152_400,
        voltage: 3.3,
        latitude: 47.4979,
    };
}

/// [`FixedWidths`] with `unknown(..)` and a `#[klv(unknown)]` field: a frame no field declares is
/// decoded by [`raw::dec`] and kept
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
    unknown(dec = raw::dec, enc = raw::enc),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
    default(typ = u16, dec = decb::be_u16, enc = *encb::be_u16),
    default(typ = u32, dec = decb::be_u32, enc = *encb::be_u32),
    default(typ = u64, dec = decb::be_u64, enc = *encb::be_u64),
    default(typ = i16, dec = decb::be_i16, enc = *encb::be_i16),
    default(typ = i32, dec = decb::be_i32, enc = *encb::be_i32),
    default(typ = f32, dec = decb::be_f32, enc = *encb::be_f32),
    default(typ = f64, dec = decb::be_f64, enc = *encb::be_f64),
)]
struct FixedWidthsKeepingUnknown {
    /// Radio channel
    #[klv(key = 0x01)]
    channel: u8,

    /// Sample sequence number
    #[klv(key = 0x02)]
    sequence: u16,

    /// Seconds since boot
    #[klv(key = 0x03)]
    uptime: u32,

    /// Microseconds since the Unix epoch
    #[klv(key = 0x04)]
    timestamp: u64,

    /// Hundredths of a degree Celsius
    #[klv(key = 0x05)]
    temperature: i16,

    /// Millimetres above sea level
    #[klv(key = 0x06)]
    altitude: i32,

    /// Supply voltage
    #[klv(key = 0x07)]
    voltage: f32,

    /// Degrees north
    #[klv(key = 0x08)]
    latitude: f64,

    /// Every other frame, its key and its bytes
    #[klv(unknown)]
    extra: Vec<(u8, Vec<u8>)>,
}

/// [`FixedWidthsKeepingUnknown`] implementation
impl FixedWidthsKeepingUnknown {
    /// The sample [`FIXED_WIDTHS_WIRE`] carries, keeping no frame
    const DECODED: Self = Self {
        channel: 3,
        sequence: 1_042,
        uptime: 86_400,
        timestamp: 1_760_000_000_000_000,
        temperature: -1_250,
        altitude: 152_400,
        voltage: 3.3,
        latitude: 47.4979,
        extra: Vec::new(),
    };

    /// The sample [`FIXED_WIDTHS_WIRE`] carries, keeping [`UNKNOWN_FRAMES`] after it
    ///
    /// # Returns
    ///
    /// [`Self::DECODED`] with the two frames in `extra`, in wire order
    fn with_unknown_frames() -> Self {
        Self {
            extra: vec![(0x20, vec![0x01, 0x04, 0x02]), (0x21, vec![0x80, 0x01])],
            ..Self::DECODED
        }
    }
}

/// Checks that `wire` decodes to `expected` with every byte read, then times the one-shot
/// [`DecodeValue::decode_value`] over it
///
/// The check runs once, outside the timed loop, so a wrong wire stops the run instead of timing a
/// decode that fails or stops short. Each timed decode reads a black-boxed copy of the slice, so
/// the optimizer cannot fold the known bytes into the decoder. The decoded value goes back to
/// criterion, which black-boxes it and drops it inside the timed loop, so a kept frame's
/// allocation and its free are both counted
///
/// # Arguments
///
/// * `group` - the criterion group to register under
/// * `id` - the benchmark id
/// * `wire` - the encoded value
/// * `expected` - the value `wire` decodes to
///
/// # Panics
///
/// When `wire` does not decode to `expected`, or leaves bytes unread
fn bench_decode<T>(
    group: &mut BenchmarkGroup<'_, WallTime>,
    id: BenchmarkId,
    wire: &[u8],
    expected: &T,
) where
    T: for<'a> DecodeValue<&'a [u8]> + PartialEq + Debug,
{
    // --------------------------------------------------
    // the wire decodes to the expected value, every byte read
    // --------------------------------------------------
    let mut cursor = wire;
    assert_eq!(T::decode_value(&mut cursor).as_ref(), Ok(expected));
    assert!(
        cursor.is_empty(),
        "`{}` left {} bytes unread",
        type_name::<T>(),
        cursor.len(),
    );
    // --------------------------------------------------
    // time the decode
    // --------------------------------------------------
    group.bench_function(id, |b| {
        b.iter(|| T::decode_value(&mut black_box(wire)));
    });
}

/// Times [`PrefixedVarints`] and [`ImplicitVarints`] decoding their wires
///
/// No throughput is set: the two wires carry the same value in a different number of bytes, so
/// the time per decode is the figure to compare
///
/// # Arguments
///
/// * `c` - the criterion driver to register the benchmark group under
fn varint_decode_value(c: &mut Criterion) {
    let mut group = c.benchmark_group("varint_decode_value");
    group.sample_size(SAMPLE_SIZE);
    bench_decode(
        &mut group,
        BenchmarkId::from_parameter("prefixed"),
        PrefixedVarints::WIRE,
        &PrefixedVarints::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::from_parameter("implicit"),
        ImplicitVarints::WIRE,
        &ImplicitVarints::DECODED,
    );
    group.finish();
}

/// Times [`FixedWidths`], [`FixedWidthsDroppingUnknown`] and [`FixedWidthsKeepingUnknown`]
/// decoding [`FIXED_WIDTHS_WIRE`], alone and with [`UNKNOWN_FRAMES`] after it
///
/// No throughput is set, as the figure to compare is the time per decode of the same known fields
///
/// # Arguments
///
/// * `c` - the criterion driver to register the benchmark group under
fn unknown_decode_value(c: &mut Criterion) {
    let two_unknown = [FIXED_WIDTHS_WIRE, UNKNOWN_FRAMES].concat();
    let mut group = c.benchmark_group("unknown_decode_value");
    group.sample_size(SAMPLE_SIZE);
    // --------------------------------------------------
    // no unknown frame on the wire
    // --------------------------------------------------
    bench_decode(
        &mut group,
        BenchmarkId::new("no_unknown", KNOWN_ONLY),
        FIXED_WIDTHS_WIRE,
        &FixedWidths::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("dropping", KNOWN_ONLY),
        FIXED_WIDTHS_WIRE,
        &FixedWidthsDroppingUnknown::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("keeping", KNOWN_ONLY),
        FIXED_WIDTHS_WIRE,
        &FixedWidthsKeepingUnknown::DECODED,
    );
    // --------------------------------------------------
    // two unknown frames after the known ones
    // --------------------------------------------------
    bench_decode(
        &mut group,
        BenchmarkId::new("no_unknown", TWO_UNKNOWN),
        &two_unknown,
        &FixedWidths::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("dropping", TWO_UNKNOWN),
        &two_unknown,
        &FixedWidthsDroppingUnknown::DECODED,
    );
    bench_decode(
        &mut group,
        BenchmarkId::new("keeping", TWO_UNKNOWN),
        &two_unknown,
        &FixedWidthsKeepingUnknown::with_unknown_frames(),
    );
    group.finish();
}

criterion_group!(benches, varint_decode_value, unknown_decode_value);
criterion_main!(benches);
