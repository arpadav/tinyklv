use tinyklv::codecs::ber::BerOid;
use tinyklv::prelude::*;

#[test]
/// Tests that BER OID encoding of `0` produces an empty byte sequence.
fn ber_oid_zero() {
    // Value 0 encodes as empty (while loop never executes)
    let encoded = tinyklv::enc::ber::ber_oid(&0_u64);
    assert!(
        encoded.is_empty(),
        "BER OID value 0 should produce empty encoding"
    );
}

#[test]
/// Tests BER OID single-byte encoding and roundtrip of `1`.
fn ber_oid_one() {
    let encoded = tinyklv::enc::ber::ber_oid(&1_u64);
    assert_eq!(encoded, vec![0x01]);
    let decoded = tinyklv::dec::ber::ber_oid::<u64>(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 1_u64);
}

#[test]
/// Tests BER OID roundtrip of `127` (upper single-byte boundary) with MSB clear.
fn ber_oid_127() {
    let encoded = tinyklv::enc::ber::ber_oid(&127_u64);
    assert_eq!(encoded, vec![0x7F]);
    assert!(encoded[0] & 0x80 == 0, "single-byte OID MSB must be 0");
    let decoded = tinyklv::dec::ber::ber_oid::<u64>(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 127_u64);
}

#[test]
/// Tests BER OID encoding of `128` produces a 2-byte continuation sequence with MSB set on the first byte and clear on the last.
fn ber_oid_128() {
    // 128 = 0b10000000; in BER OID: [0x81, 0x00]
    let encoded = tinyklv::enc::ber::ber_oid(&128_u64);
    assert_eq!(encoded.len(), 2);
    // All bytes except last have MSB set
    for &b in &encoded[..encoded.len() - 1] {
        assert!(b & 0x80 != 0, "continuation byte MSB must be 1");
    }
    assert!(
        encoded.last().unwrap() & 0x80 == 0,
        "final byte MSB must be 0"
    );
    let decoded = tinyklv::dec::ber::ber_oid::<u64>(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 128_u64);
}

#[test]
/// Tests `BerOid` roundtrip of `23298` with known-good encoding `[129, 182, 2]` (from doctest).
fn ber_oid_23298_known() {
    // From doctest: 23298 encodes as [129, 182, 2]
    let encoded = BerOid::encode_value(&23298_u64);
    assert_eq!(encoded, vec![129, 182, 2]);
    let decoded = BerOid::<u64>::decode_value(&mut vec![129, 182, 2].as_slice()).unwrap();
    assert_eq!(decoded.value, 23298_u64);
}

#[test]
/// Tests BER OID roundtrip of `255`.
fn ber_oid_255() {
    let encoded = tinyklv::enc::ber::ber_oid(&255_u64);
    let decoded = tinyklv::dec::ber::ber_oid::<u64>(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 255_u64);
}

#[test]
/// Tests BER OID roundtrip of `16383` (`0x3FFF`, maximum 2-byte OID value).
fn ber_oid_16383() {
    // 16383 = 0x3FFF: max value fitting in 2 OID bytes
    let encoded = tinyklv::enc::ber::ber_oid(&16383_u64);
    assert_eq!(encoded.len(), 2);
    let decoded = tinyklv::dec::ber::ber_oid::<u64>(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 16383_u64);
}

#[test]
/// Tests BER OID roundtrip of `0x0010_0000` (a large multi-byte OID value).
fn ber_oid_large_value() {
    let val: u64 = 0x0010_0000;
    let encoded = tinyklv::enc::ber::ber_oid(&val);
    let decoded = tinyklv::dec::ber::ber_oid::<u64>(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, val);
}

#[test]
/// Tests BER OID roundtrip across a range of values spanning all continuation-byte boundaries.
fn ber_oid_roundtrip_range() {
    for val in [
        1_u32,
        2,
        63,
        64,
        127,
        128,
        255,
        256,
        16383,
        16384,
        0x007F_FFFF,
    ] {
        let encoded = tinyklv::enc::ber::ber_oid(&val);
        let decoded = tinyklv::dec::ber::ber_oid::<u32>(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded, val, "roundtrip failed for OID value {val}");
    }
}

#[test]
/// Tests `BerOid` wrapper struct roundtrip for `23298`.
fn ber_oid_struct_roundtrip() {
    let val = 23298_u64;
    let oid = BerOid::new(&val);
    let encoded = oid.encode_value();
    let decoded = BerOid::<u64>::decode_value(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded.value, val);
}

#[test]
/// Tests that `ber_oid` errors on empty input.
fn ber_oid_empty_input_fails() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::ber::ber_oid::<u64>(&mut input).is_err());
}
