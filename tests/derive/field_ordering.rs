// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

fn enc_u8(v: &u8) -> Vec<u8> {
    tinyklv::enc::binary::u8(*v)
}
fn enc_u16(v: &u16) -> Vec<u8> {
    tinyklv::enc::binary::be_u16(*v)
}
fn enc_u32(v: &u32) -> Vec<u8> {
    tinyklv::enc::binary::be_u32(*v)
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct OrderIndependent {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u8, enc = enc_u8)]
    a: u8,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    b: u16,
    #[klv(key = 0x03, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
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
fn normal_key_order() {
    let data = build_packet(true);
    let result = OrderIndependent::decode(&mut data.as_slice()).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}

#[test]
fn reversed_key_order_same_result() {
    let data = build_packet(false);
    let result = OrderIndependent::decode(&mut data.as_slice()).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}

#[test]
fn both_orderings_produce_identical_structs() {
    let fwd = build_packet(true);
    let rev = build_packet(false);
    let r_fwd = OrderIndependent::decode(&mut fwd.as_slice()).unwrap();
    let r_rev = OrderIndependent::decode(&mut rev.as_slice()).unwrap();
    assert_eq!(r_fwd, r_rev);
}

#[test]
fn interleaved_order() {
    // b first, a second, c third
    let data: &[u8] = &[
        0x02, 0x02, 0x01, 0x02, 0x01, 0x01, 0x42, 0x03, 0x04, 0xAB, 0xCD, 0xEF, 0x01,
    ];
    let result = OrderIndependent::decode(&mut &data[..]).unwrap();
    assert_eq!(result.a, 0x42);
    assert_eq!(result.b, 0x0102);
    assert_eq!(result.c, 0xABCDEF01);
}
