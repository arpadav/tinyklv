//! Regression test for generic parameter support on `#[derive(Klv)]`.
//!
//! Exercises:
//! * lifetime-generic stream structs
//! * type-generic structs carrying `PhantomData<T>` (zero-size, skipped by derive)
//! * user-authored `where` clauses preserved verbatim via `split_for_impl`
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use std::marker::PhantomData;
use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

// --------------------------------------------------
// Type-generic container with PhantomData<T>
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA",
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Tagged<T> {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    id: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    counter: u32,
    _phantom: PhantomData<T>,
}

// phantom marker types
#[derive(Debug, PartialEq)]
struct MarkerA;
#[derive(Debug, PartialEq)]
struct MarkerB;

// --------------------------------------------------
// Type-generic container with user `where` clause
// (preserved verbatim via split_for_impl)
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Bounded<T>
where
    T: Send + Sync + 'static,
{
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    value: u16,
    _phantom: PhantomData<T>,
}

// --------------------------------------------------
// Multi-parameter generic container
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct TwoParams<T, U> {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    first: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    second: u32,
    _t: PhantomData<T>,
    _u: PhantomData<U>,
}

// --------------------------------------------------
// tests
// --------------------------------------------------

#[test]
fn tagged_roundtrip_marker_a() {
    let original: Tagged<MarkerA> = Tagged {
        id: 0xBEEF,
        counter: 0xDEADBEEF,
        _phantom: PhantomData,
    };
    let encoded = original.encode_frame();
    let decoded = Tagged::<MarkerA>::decode_frame(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn tagged_roundtrip_marker_b() {
    let original: Tagged<MarkerB> = Tagged {
        id: 0x0101,
        counter: 0x02030405,
        _phantom: PhantomData,
    };
    let encoded = original.encode_frame();
    let decoded = Tagged::<MarkerB>::decode_frame(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn tagged_distinct_marker_types_have_separate_impls() {
    // Proves separate monomorphized impls - same bytes decode into either T.
    let a: Tagged<MarkerA> = Tagged {
        id: 0x1234,
        counter: 0x56789ABC,
        _phantom: PhantomData,
    };
    let bytes = a.encode_frame();
    let decoded_a = Tagged::<MarkerA>::decode_frame(&mut bytes.as_slice()).unwrap();
    let decoded_b = Tagged::<MarkerB>::decode_frame(&mut bytes.as_slice()).unwrap();
    assert_eq!(decoded_a.id, decoded_b.id);
    assert_eq!(decoded_a.counter, decoded_b.counter);
}

#[test]
fn bounded_where_clause_roundtrip() {
    let original: Bounded<u64> = Bounded {
        value: 0xCAFE,
        _phantom: PhantomData,
    };
    let encoded = original.encode_value();
    let decoded = Bounded::<u64>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}

#[test]
fn two_params_roundtrip() {
    let original: TwoParams<MarkerA, MarkerB> = TwoParams {
        first: 0xAAAA,
        second: 0xBBBBCCCC,
        _t: PhantomData,
        _u: PhantomData,
    };
    let encoded = original.encode_value();
    let decoded = TwoParams::<MarkerA, MarkerB>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, original);
}
