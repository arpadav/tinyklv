#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::unwrap_used)]
//! Example 13 - `varlen = true` for length-parameterised value decoders.
//!
//! A fixed-width value decoder has the signature
//! `fn(&mut Stream) -> Result<T>` - it reads exactly as many bytes as the
//! type requires. A variable-length field instead reads a number of bytes
//! dictated by the preceding `len` field, which the macro plumbs into the
//! decoder when the field is annotated with `varlen = true`:
//!
//! ```text
//! fn(usize) -> impl FnMut(&mut Stream) -> Result<T>
//! ```
//!
//! This example uses a sensor-log entry with a fixed-width `u32` timestamp
//! and a variable-length UTF-8 annotation, then round-trips two payloads -
//! a short one (single-byte length) and a long one (multi-byte length via
//! BER) - so the length codec gets exercised in both regimes.
//!
//! Showcases:
//! * `varlen = true` on a `String` field
//! * Mixing fixed-width and variable-length fields in one struct
//! * BER length codec scaling to small and large payloads on the same struct
use tinyklv::prelude::*;            // Klv proc-macro + traits
use tinyklv::dec::binary as decb;   // binary decoders
use tinyklv::enc::binary as encb;   // binary encoders
use tinyklv::enc::string as encs;   // string encoders
use tinyklv::dec::ber as decber;    // BER decoders
use tinyklv::enc::ber as encber;    // BER encoders

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],                                              // default, shown for clarity
    sentinel = b"SENSORLOG",
    key(dec = decber::ber_oid::<u64>, enc = encber::ber_oid),
    len(dec = decber::ber_length,     enc = encber::ber_length::<usize>),        // BER length scales with payload size
)]
/// Sensor log entry: fixed-width timestamp + variable-length UTF-8 annotation
struct SensorLog {
    /// Unix timestamp in seconds (fixed-width big-endian u32)
    #[klv(
        key = 0x01_u64,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    timestamp_s: u32,

    /// Human-readable annotation; `varlen = true` passes the decoded length
    /// into the string decoder so it reads exactly that many bytes
    #[klv(
        key = 0x02_u64,
        varlen = true,
        dec = decb::to_string_utf8,
        enc = &encs::from_string_utf8,
    )]
    annotation: String,
}

fn main() {
    // build - a short annotation whose BER length fits in one byte
    let small = SensorLog {
        timestamp_s: 1_700_000_000,
        annotation:  String::from("OK"),
    };

    // encode + decode - round-trip through the single-byte BER length path
    let small_frame = small.encode_frame();
    let dec_small = SensorLog::decode_frame(
        &mut small_frame.as_slice(),
    ).unwrap();
    assert_eq!(dec_small, small);

    // build - a long annotation forcing the outer BER length into multiple bytes
    let large = SensorLog {
        timestamp_s: 1_700_000_001,
        annotation:  "X".repeat(200), // 200 > 127 -> multi-byte BER length
    };

    // encode + decode - round-trip through the multi-byte BER length path
    let large_frame = large.encode_frame();
    let dec_large = SensorLog::decode_frame(
        &mut large_frame.as_slice(),
    ).unwrap();

    // assert - both payloads survive; the long one proves `varlen = true`
    // correctly threaded the length into the string decoder
    assert_eq!(dec_large, large);
    assert!(
        large_frame.len() > small_frame.len(),
        "long annotation must produce a longer frame",
    );
}
