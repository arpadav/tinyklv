// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;
use tinyklv::Klv;

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
/// Tests decoding when keys appear in their declared struct order (`0x01`, `0x02`, `0x03`).
fn normal_key_order() {
    let data = build_packet(true);
    let result = OrderIndependent::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}

#[test]
/// Verifies that decoding yields the same struct when keys appear in fully reversed data order.
fn reversed_key_order_same_result() {
    let data = build_packet(false);
    let result = OrderIndependent::decode_value(&mut data.as_slice()).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}

#[test]
/// Tests that forward- and reverse-ordered key streams decode to identical struct values, confirming order-independence.
fn both_orderings_produce_identical_structs() {
    let fwd = build_packet(true);
    let rev = build_packet(false);
    let r_fwd = OrderIndependent::decode_value(&mut fwd.as_slice()).unwrap();
    let r_rev = OrderIndependent::decode_value(&mut rev.as_slice()).unwrap();
    assert_eq!(r_fwd, r_rev);
}

#[test]
/// Tests decoding when keys arrive in an arbitrary interleaved order (`b`, `a`, `c`).
fn interleaved_order() {
    let data: &[u8] = &[
        0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x42, 0x03, 0x04, 0xAB, 0xCD, 0xEF, 0x01,
    ];
    let result = OrderIndependent::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}
