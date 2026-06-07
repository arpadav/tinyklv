//! `*_as_usize` and `*_from_usize` binary codec tests
//!
//! Tests the usize-bridging codecs that read or write platform-sized integers:
//! `u8_as_usize`, `u16_as_usize`, `u32_as_usize`, `u64_as_usize`,
//! `be_u16_as_usize`, `be_u32_as_usize`, `be_u64_as_usize`,
//! `le_u16_as_usize`, `le_u32_as_usize`, and their `_from_usize` encoder
//! counterparts. Covers known-value decoding for BE and LE variants,
//! encode/decode roundtrips, and the `be_u32_from_usize`/`be_u32_as_usize`
//! roundtrip across boundary values
//!
//! Author: aav
#[test]
/// Tests `u8_as_usize` decodes native-endian `1_u8` bytes to `1_usize`
fn u8_as_usize_one() {
    let mut input: &[u8] = &(1_u8).to_ne_bytes();
    assert_eq!(tinyklv::dec::binary::u8_as_usize(&mut input), Ok(1_usize));
}

#[test]
/// Tests `u8_as_usize` decodes `0x00` to `0_usize`
fn u8_as_usize_zero() {
    let mut input: &[u8] = &[0x00];
    assert_eq!(tinyklv::dec::binary::u8_as_usize(&mut input), Ok(0_usize));
}

#[test]
/// Tests `u8_as_usize` decodes `0xFF` to `255_usize`
fn u8_as_usize_max() {
    let mut input: &[u8] = &[0xFF];
    assert_eq!(tinyklv::dec::binary::u8_as_usize(&mut input), Ok(255_usize));
}

#[test]
/// Tests `u16_as_usize` decodes native-endian `1_u16` bytes to `1_usize`
fn u16_as_usize_one() {
    let mut input: &[u8] = &(1_u16).to_ne_bytes();
    assert_eq!(tinyklv::dec::binary::u16_as_usize(&mut input), Ok(1_usize));
}

#[test]
/// Tests `u32_as_usize` decodes native-endian `1_u32` bytes to `1_usize`
fn u32_as_usize_one() {
    let mut input: &[u8] = &(1_u32).to_ne_bytes();
    assert_eq!(tinyklv::dec::binary::u32_as_usize(&mut input), Ok(1_usize));
}

#[test]
/// Tests `u64_as_usize` decodes native-endian `1_u64` bytes to `1_usize`
fn u64_as_usize_one() {
    let mut input: &[u8] = &(1_u64).to_ne_bytes();
    assert_eq!(tinyklv::dec::binary::u64_as_usize(&mut input), Ok(1_usize));
}

#[test]
/// Tests `u8_as_usize` decodes `1_u8` (BE byte order) to `1_usize`
fn u8_as_usize_one_be_bytes() {
    let mut input: &[u8] = &(1_u8).to_be_bytes();
    assert_eq!(tinyklv::dec::binary::u8_as_usize(&mut input), Ok(1_usize));
}

#[test]
/// Tests `be_u16_as_usize` decodes `1_u16` big-endian bytes to `1_usize`
fn be_u16_as_usize_one() {
    let mut input: &[u8] = &(1_u16).to_be_bytes();
    assert_eq!(
        tinyklv::dec::binary::be_u16_as_usize(&mut input),
        Ok(1_usize)
    );
}

#[test]
/// Tests `be_u16_as_usize` decodes `[0x01, 0x00]` (`0x0100` BE) to `256_usize`
fn be_u16_as_usize_known() {
    let mut input: &[u8] = &[0x01, 0x00]; // 0x0100 = 256 BE
    assert_eq!(
        tinyklv::dec::binary::be_u16_as_usize(&mut input),
        Ok(256_usize)
    );
}

#[test]
/// Tests `be_u32_as_usize` decodes `[0x00, 0x01, 0x00, 0x00]` to `65536_usize`
fn be_u32_as_usize_known() {
    let mut input: &[u8] = &[0x00, 0x01, 0x00, 0x00]; // 0x00010000 = 65536
    assert_eq!(
        tinyklv::dec::binary::be_u32_as_usize(&mut input),
        Ok(65536_usize)
    );
}

#[test]
/// Tests `be_u64_as_usize` decodes `1_u64` big-endian bytes to `1_usize`
fn be_u64_as_usize_one() {
    let mut input: &[u8] = &(1_u64).to_be_bytes();
    assert_eq!(
        tinyklv::dec::binary::be_u64_as_usize(&mut input),
        Ok(1_usize)
    );
}

#[test]
/// Tests `u8_as_usize` decodes `1_u8` (LE byte order) to `1_usize`
fn u8_as_usize_one_le_bytes() {
    let mut input: &[u8] = &(1_u8).to_le_bytes();
    assert_eq!(tinyklv::dec::binary::u8_as_usize(&mut input), Ok(1_usize));
}

#[test]
/// Tests `le_u16_as_usize` decodes `[0x00, 0x01]` (`0x0100` LE) to `256_usize`
fn le_u16_as_usize_known() {
    let mut input: &[u8] = &[0x00, 0x01]; // 0x0100 = 256 LE
    assert_eq!(
        tinyklv::dec::binary::le_u16_as_usize(&mut input),
        Ok(256_usize)
    );
}

#[test]
/// Tests `le_u32_as_usize` decodes `1_u32` little-endian bytes to `1_usize`
fn le_u32_as_usize_one() {
    let mut input: &[u8] = &(1_u32).to_le_bytes();
    assert_eq!(
        tinyklv::dec::binary::le_u32_as_usize(&mut input),
        Ok(1_usize)
    );
}

#[test]
/// Tests `u8_from_usize(1)` encodes to `[0x01]`
fn u8_from_usize_one() {
    let mut __v = Vec::new();
    tinyklv::enc::binary::u8_from_usize(1, &mut __v);
    assert_eq!(__v, vec![0x01]);
}

#[test]
/// Tests `u8_from_usize(0)` encodes to `[0x00]`
fn u8_from_usize_zero() {
    let mut __v = Vec::new();
    tinyklv::enc::binary::u8_from_usize(0, &mut __v);
    assert_eq!(__v, vec![0x00]);
}

#[test]
/// Tests `u16_from_usize(1)` encodes and native-endian decodes back to `1_u16`
fn u16_from_usize_one() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::u16_from_usize(1, &mut encoded);
    assert_eq!(encoded.len(), 2);
    // Decode back as native-endian u16
    let decoded = tinyklv::dec::binary::u16(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 1_u16);
}

#[test]
/// Tests `be_u16_from_usize(1)` encodes to big-endian `[0x00, 0x01]`
fn be_u16_from_usize_one() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u16_from_usize(1, &mut encoded);
    assert_eq!(encoded, vec![0x00, 0x01]);
}

#[test]
/// Tests `be_u16_from_usize(256)` encodes to big-endian `[0x01, 0x00]`
fn be_u16_from_usize_256() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::be_u16_from_usize(256, &mut encoded);
    assert_eq!(encoded, vec![0x01, 0x00]);
}

#[test]
/// Tests `le_u16_from_usize(256)` encodes to little-endian `[0x00, 0x01]`
fn le_u16_from_usize_256() {
    let mut encoded = Vec::new();
    tinyklv::enc::binary::le_u16_from_usize(256, &mut encoded);
    assert_eq!(encoded, vec![0x00, 0x01]);
}

#[test]
/// Tests `be_u32_from_usize`/`be_u32_as_usize` roundtrip across boundary values
fn be_u32_from_usize_roundtrip() {
    for val in [0_usize, 1, 255, 65536] {
        let mut encoded = Vec::new();
        tinyklv::enc::binary::be_u32_from_usize(val, &mut encoded);
        let decoded = tinyklv::dec::binary::be_u32_as_usize(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}
