//! book reference - codecs: lengthed integers and BER
use tinyklv::dec::ber as decber;
use tinyklv::dec::binary as decb;
use tinyklv::enc::ber as encber;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(
        dec = decb::u8,
        enc = encb::u8,
    ),
    len(
        dec = decb::u8_as_usize,
        enc = encb::u8_from_usize,
    ),
)]
/// A `u32` written in 3 bytes
struct Counter {
    #[klv(
        key = 0x04,
        dec = decb::be_u32_lengthed,
        enc = *(encb::be_u32_lengthed(3)),
        size(exact = 3, lengthed),
    )]
    count: u32,
}

/// Reads all of `value` with `be_u16_lengthed`
fn be_u16_lengthed(value: &[u8]) -> u16 {
    decb::be_u16_lengthed(value.len())(&mut &value[..]).unwrap()
}

/// Encodes `value` with an encoder
fn encode_with<T>(encoder: impl Fn(T, &mut Vec<u8>), value: T) -> Vec<u8> {
    let mut out = Vec::new();
    encoder(value, &mut out);
    out
}

fn main() {
    // lengthed: a shorter value is zero-extended, a longer
    // one keeps its low-order bytes
    assert_eq!(be_u16_lengthed(&[0x2A]), 42);
    assert_eq!(be_u16_lengthed(&[0x09, 0x2E]), 2350);
    assert_eq!(be_u16_lengthed(&[0x00, 0x09, 0x2E]), 2350);

    // a u32 in 3 bytes
    let counter = Counter { count: 70_000 };
    let mut encoded = Vec::new();
    counter.encode_value(&mut encoded);
    println!("{counter:?} -> {encoded:02X?}");
    let wire = [0x04, 0x03, 0x01, 0x11, 0x70];
    assert_eq!(encoded, wire);
    let decoded = Counter::decode_value(&mut wire.as_slice());
    assert_eq!(decoded.unwrap(), counter);

    // BER length: 1 byte below 128, then a count of the
    // bytes that follow
    for (len, wire) in [
        (47, &[0x2F][..]),
        (128, &[0x81, 0x80]),
        (201, &[0x81, 0xC9]),
    ] {
        assert_eq!(encode_with(encber::ber_length, len), wire);
        assert_eq!(decber::ber_length(&mut &wire[..]), Ok(len));
    }

    // BER-OID: 7 bits per byte, the top bit set while more
    // follow
    for (oid, wire) in [
        (47_u64, &[0x2F][..]),
        (128, &[0x81, 0x00]),
        (201, &[0x81, 0x49]),
    ] {
        assert_eq!(encode_with(encber::ber_oid, oid), wire);
        assert_eq!(decber::ber_oid(&mut &wire[..]), Ok(oid));
    }
}
