//! Field-ordering independence tests for `#[derive(Klv)]`
//!
//! Verifies that the derived `decode_value` implementation is order-independent:
//! a stream where keys arrive in declaration order, reversed order, or an
//! arbitrary interleaved order must all produce the same decoded struct. The
//! `OrderIndependent` struct uses three keys (`0x01`, `0x02`, `0x03`) and
//! tests all three orderings
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::Klv;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct OrderIndependent {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    a: u8,
    #[klv(
        key = 0x02,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    b: u16,
    #[klv(
        key = 0x03,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    c: u32,
}

/// Build a raw `OrderIndependent` byte stream in either forward or reversed key order
///
/// When `a_first` is `true` keys arrive as `0x01, 0x02, 0x03` (declaration order);
/// when `false` they arrive as `0x03, 0x02, 0x01` (reversed order). In both cases
/// the encoded field values are `a=0x42`, `b=0x0102`, `c=0xABCDEF01`
fn build_packet(a_first: bool) -> Vec<u8> {
    if a_first {
        // a=0x01 key first
        vec![
            0x01, 0x01, 0x42, 0x02, 0x02, 0x01, 0x02, 0x03, 0x04, 0xAB, 0xCD, 0xEF, 0x01,
        ]
    } else {
        // c key first, then b, then a - reversed order
        vec![
            0x03, 0x04, 0xAB, 0xCD, 0xEF, 0x01, 0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x42,
        ]
    }
}

#[test]
/// Tests decoding when keys appear in their declared struct order (`0x01`, `0x02`, `0x03`)
fn normal_key_order() {
    let data = build_packet(true);
    let result = OrderIndependent::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}

#[test]
/// Verifies that decoding yields the same struct when keys appear in fully reversed data order
fn reversed_key_order_same_result() {
    let data = build_packet(false);
    let result = OrderIndependent::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}

#[test]
/// Tests that forward- and reverse-ordered key streams decode to identical struct values, confirming order-independence
fn both_orderings_produce_identical_structs() {
    let fwd = build_packet(true);
    let rev = build_packet(false);
    let r_fwd = OrderIndependent::decode_value(&mut fwd.as_slice()).unwrap();
    let r_rev = OrderIndependent::decode_value(&mut rev.as_slice()).unwrap();
    assert_eq!(r_fwd, r_rev);
}

#[test]
/// Tests decoding when keys arrive in an arbitrary interleaved order (`b`, `a`, `c`)
fn interleaved_order() {
    let data: &[u8] = &[
        0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x42, 0x03, 0x04, 0xAB, 0xCD, 0xEF, 0x01,
    ];
    let result = OrderIndependent::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}
