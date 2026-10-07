//! BER length encoding and decoding tests
//!
//! Covers `enc::ber::ber_length` / `dec::ber::ber_length` at the function
//! level and the [`BerLength`] typed-struct wrapper: short-form boundary
//! values (`0`, `1`, `64`, `127`), the long-form boundary (`128`), long-form
//! values through `u32::MAX`, explicit byte-layout assertions for known
//! encodings, error cases on empty or truncated long-form input, and the value
//! boundary at every target width
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use super::helpers::DecoderExt;
use super::width::BerWidth;
use crate::common::error::ContextErrorExt;
use tinyklv::codecs::ber::BerLength;
use tinyklv::dec::ber as decber;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::iter;

#[test]
/// Tests BER length encoding of `0` as short form (`[0x00]`) and its roundtrip
fn short_form_zero() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(0_u8, &mut encoded);
    assert_eq!(encoded, vec![0x00]);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 0_usize);
}

#[test]
/// Tests BER length encoding of `1` as short form (`[0x01]`) and its roundtrip
fn short_form_one() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(1_u8, &mut encoded);
    assert_eq!(encoded, vec![0x01]);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 1_usize);
}

#[test]
/// Tests BER length encoding of `64` as short form and verifies the MSB stays clear
fn short_form_64() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(64_u8, &mut encoded);
    assert_eq!(encoded, vec![0x40]);
    assert!(encoded[0] & 0x80 == 0, "short form MSB must be 0");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 64_usize);
}

#[test]
/// Tests BER length encoding of `127` (upper short-form boundary) with MSB clear
fn short_form_127() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(127_u8, &mut encoded);
    assert_eq!(encoded, vec![0x7F]);
    assert!(encoded[0] & 0x80 == 0, "short form MSB must be 0");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 127_usize);
}

#[test]
/// Tests BER length encoding of `128` as long form with one extra byte and MSB set
fn long_form_128() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(128_u8, &mut encoded);
    assert_eq!(encoded.len(), 2);
    assert!(encoded[0] & 0x80 != 0, "long form first byte MSB must be 1");
    assert_eq!(encoded[0] & 0x7F, 1, "should claim 1 extra byte");
    assert_eq!(encoded[1], 128);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 128_usize);
}

#[test]
/// Tests BER length roundtrip of `129` (long form)
fn long_form_129() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(129_u8, &mut encoded);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 129_usize);
}

#[test]
/// Tests BER length roundtrip of `255` with the long-form MSB set
fn long_form_255() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(255_u8, &mut encoded);
    assert!(encoded[0] & 0x80 != 0, "long form first byte MSB must be 1");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 255_usize);
}

#[test]
/// Tests BER length roundtrip of `256` (first 2-byte long-form value)
fn long_form_256() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(256_u16, &mut encoded);
    assert!(encoded[0] & 0x80 != 0);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 256_usize);
}

#[test]
/// Tests BER length roundtrip of `65535` (upper 2-byte long-form boundary)
fn long_form_65535() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(65535_u32, &mut encoded);
    assert!(encoded[0] & 0x80 != 0);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 65535_usize);
}

#[test]
/// Tests BER length roundtrip at `u32::MAX` (requires 4 long-form bytes)
fn long_form_u32_max() {
    let mut encoded = Vec::new();
    tinyklv::enc::ber::ber_length(u32::MAX, &mut encoded);
    assert!(encoded[0] & 0x80 != 0);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, u32::MAX as usize);
}

#[test]
/// Tests `BerLength` wrapper roundtrip for a short-form value `47`
fn berlength_struct_47() {
    let val = 47_u64;
    let ber = BerLength::new(val);
    let mut encoded = Vec::new();
    ber.encode_value(&mut encoded);
    assert_eq!(encoded, vec![47]);
    let decoded = BerLength::<u64>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, BerLength::new(val));
}

#[test]
/// Tests `BerLength` wrapper roundtrip for `201` with explicit long-form byte layout `[0x81, 201]`
fn berlength_struct_201() {
    let val = 201_u64;
    let ber = BerLength::new(val);
    let mut encoded = Vec::new();
    ber.encode_value(&mut encoded);
    assert_eq!(encoded, vec![128 + 1, 201]);
    let decoded = BerLength::<u64>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, BerLength::new(val));
}

#[test]
/// Tests `BerLength` wrapper roundtrip for a 6-byte long-form u64 value with explicit expected bytes
fn berlength_struct_large() {
    let val = 123891829038102_u64;
    let ber = BerLength::new(val);
    let mut encoded = Vec::new();
    ber.encode_value(&mut encoded);
    assert_eq!(encoded, vec![128 + 6, 112, 173, 208, 117, 220, 22]);
    let decoded = BerLength::<u64>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, BerLength::new(val));
}

#[test]
/// Tests `BerLength<u32>` roundtrip for `8_500_738` with expected long-form bytes `[0x83, 0x81, 0xB6, 0x02]`
fn berlength_8500738_u32() {
    let mut encoded = Vec::new();
    BerLength::new(8_500_738_u32).encode_value(&mut encoded);
    assert_eq!(encoded, vec![128 + 3, 129, 182, 2]);
    let decoded = BerLength::<u32>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, BerLength::new(8_500_738_u32));
}

#[test]
/// Tests that `ber_length` errors on empty input
fn ber_length_empty_input_fails() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}

#[test]
/// Tests that a long-form prefix `0x82` claiming 2 extra bytes errors when only 1 byte follows,
/// with the cursor at the end of input and an error expecting the bytes its long form claims, so a
/// framed caller classes the split as truncation
fn ber_length_truncated_long_form_fails() {
    // Byte 0x82 claims 2 more bytes, but only 1 follows
    let wire = [0x82, 0x01];
    let mut input = &wire[..];
    match tinyklv::dec::ber::ber_length(&mut input) {
        Err(error) if error.expects_length_bytes() => {}
        other => panic!("expected a truncated-length Err, got {other:?}"),
    }
    assert_eq!(input, &wire[2..]);
}

#[test]
/// Tests that a long-form prefix `0x81` claiming 1 extra byte errors when no byte follows
fn ber_length_truncated_long_form_zero_extra() {
    // Byte 0x81 claims 1 more byte, but none follow
    let mut input: &[u8] = &[0x81];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}

#[test]
/// Tests that a `BerLength` long form split inside its length bytes errors with the cursor at
/// the end of input, so a framed caller classes the split as truncation
fn berlength_split_long_form_consumes_to_end() {
    // 0x83 claims 3 more bytes, but only 2 follow
    let wire = [0x83, 0x01, 0x00];
    let mut input = &wire[..];
    assert!(BerLength::<u32>::decode_value(&mut input).is_err());
    assert_eq!(input, &wire[3..]);
}

#[test]
/// Tests that `BerLength` rejects a value past its type's range with the cursor unchanged and
/// an error expecting a value that fits
fn berlength_overflow_rewinds() {
    // 0x82 0x01 0x00 = 256, past u8::MAX
    let wire = [0x82, 0x01, 0x00];
    let mut input = &wire[..];
    match BerLength::<u8>::decode_value(&mut input) {
        Err(error) if error.expects_fits_target() => {}
        other => panic!("expected an overflow Err, got {other:?}"),
    }
    assert_eq!(input, &wire[..]);
}

#[test]
/// Tests that `ber_length` reads `usize::MAX` from as many length bytes as `usize` is wide
fn ber_length_usize_max() {
    decber::ber_length.assert_reads(&usize::length(None, 0xFF), usize::MAX);
}

#[test]
/// Tests that `ber_length` rejects a long form one length byte wider than `usize` (9 bytes on a
/// 64-bit target), every byte non-zero, with the cursor rewound: the whole value is checked,
/// never cut to its low bytes
fn ber_length_nonzero_past_usize_rewinds() {
    let count = size_of::<usize>() as u8 + 1;
    let wire: Vec<u8> = iter::once(0x80 | count).chain(1..=count).collect();
    decber::ber_length.assert_rewinds(&wire);
}

#[test]
/// Tests the `BerLength` value boundary at `u8` (see `BerWidth::assert_length_boundary`)
fn berlength_boundary_u8() {
    u8::assert_length_boundary();
}

#[test]
/// Tests the `BerLength` value boundary at `u16` (see `BerWidth::assert_length_boundary`)
fn berlength_boundary_u16() {
    u16::assert_length_boundary();
}

#[test]
/// Tests the `BerLength` value boundary at `u32` (see `BerWidth::assert_length_boundary`)
fn berlength_boundary_u32() {
    u32::assert_length_boundary();
}

#[test]
/// Tests the `BerLength` value boundary at `u64` (see `BerWidth::assert_length_boundary`)
fn berlength_boundary_u64() {
    u64::assert_length_boundary();
}

#[test]
/// Tests the `BerLength` value boundary at `u128` (see `BerWidth::assert_length_boundary`)
fn berlength_boundary_u128() {
    u128::assert_length_boundary();
}

#[test]
/// Tests the `BerLength` value boundary at `usize` (see `BerWidth::assert_length_boundary`)
fn berlength_boundary_usize() {
    usize::assert_length_boundary();
}
