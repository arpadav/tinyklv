// --------------------------------------------------
// native-endian _as_usize decoders
// --------------------------------------------------

#[test]
fn u8_as_usize_one() {
    let mut input: &[u8] = &(1_u8).to_ne_bytes();
    assert_eq!(tinyklv::dec::binary::u8_as_usize(&mut input), Ok(1_usize));
}

#[test]
fn u8_as_usize_zero() {
    let mut input: &[u8] = &[0x00];
    assert_eq!(tinyklv::dec::binary::u8_as_usize(&mut input), Ok(0_usize));
}

#[test]
fn u8_as_usize_max() {
    let mut input: &[u8] = &[0xFF];
    assert_eq!(tinyklv::dec::binary::u8_as_usize(&mut input), Ok(255_usize));
}

#[test]
fn u16_as_usize_one() {
    let mut input: &[u8] = &(1_u16).to_ne_bytes();
    assert_eq!(tinyklv::dec::binary::u16_as_usize(&mut input), Ok(1_usize));
}

#[test]
fn u32_as_usize_one() {
    let mut input: &[u8] = &(1_u32).to_ne_bytes();
    assert_eq!(tinyklv::dec::binary::u32_as_usize(&mut input), Ok(1_usize));
}

#[test]
fn u64_as_usize_one() {
    let mut input: &[u8] = &(1_u64).to_ne_bytes();
    assert_eq!(tinyklv::dec::binary::u64_as_usize(&mut input), Ok(1_usize));
}

// --------------------------------------------------
// be_ prefixed _as_usize decoders
// --------------------------------------------------

#[test]
fn be_u8_as_usize_one() {
    let mut input: &[u8] = &(1_u8).to_be_bytes();
    assert_eq!(
        tinyklv::dec::binary::be_u8_as_usize(&mut input),
        Ok(1_usize)
    );
}

#[test]
fn be_u16_as_usize_one() {
    let mut input: &[u8] = &(1_u16).to_be_bytes();
    assert_eq!(
        tinyklv::dec::binary::be_u16_as_usize(&mut input),
        Ok(1_usize)
    );
}

#[test]
fn be_u16_as_usize_known() {
    let mut input: &[u8] = &[0x01, 0x00]; // 0x0100 = 256 BE
    assert_eq!(
        tinyklv::dec::binary::be_u16_as_usize(&mut input),
        Ok(256_usize)
    );
}

#[test]
fn be_u32_as_usize_known() {
    let mut input: &[u8] = &[0x00, 0x01, 0x00, 0x00]; // 0x00010000 = 65536
    assert_eq!(
        tinyklv::dec::binary::be_u32_as_usize(&mut input),
        Ok(65536_usize)
    );
}

#[test]
fn be_u64_as_usize_one() {
    let mut input: &[u8] = &(1_u64).to_be_bytes();
    assert_eq!(
        tinyklv::dec::binary::be_u64_as_usize(&mut input),
        Ok(1_usize)
    );
}

// --------------------------------------------------
// le_ prefixed _as_usize decoders
// --------------------------------------------------

#[test]
fn le_u8_as_usize_one() {
    let mut input: &[u8] = &(1_u8).to_le_bytes();
    assert_eq!(
        tinyklv::dec::binary::le_u8_as_usize(&mut input),
        Ok(1_usize)
    );
}

#[test]
fn le_u16_as_usize_known() {
    let mut input: &[u8] = &[0x00, 0x01]; // 0x0100 = 256 LE
    assert_eq!(
        tinyklv::dec::binary::le_u16_as_usize(&mut input),
        Ok(256_usize)
    );
}

#[test]
fn le_u32_as_usize_one() {
    let mut input: &[u8] = &(1_u32).to_le_bytes();
    assert_eq!(
        tinyklv::dec::binary::le_u32_as_usize(&mut input),
        Ok(1_usize)
    );
}

// --------------------------------------------------
// _from_usize encoders
// --------------------------------------------------

#[test]
fn u8_from_usize_one() {
    assert_eq!(tinyklv::enc::binary::u8_from_usize(1), vec![0x01]);
}

#[test]
fn u8_from_usize_zero() {
    assert_eq!(tinyklv::enc::binary::u8_from_usize(0), vec![0x00]);
}

#[test]
fn u16_from_usize_one() {
    let encoded = tinyklv::enc::binary::u16_from_usize(1);
    assert_eq!(encoded.len(), 2);
    // Decode back as native-endian u16
    let decoded = tinyklv::dec::binary::u16(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, 1_u16);
}

#[test]
fn be_u16_from_usize_one() {
    let encoded = tinyklv::enc::binary::be_u16_from_usize(1);
    assert_eq!(encoded, vec![0x00, 0x01]);
}

#[test]
fn be_u16_from_usize_256() {
    let encoded = tinyklv::enc::binary::be_u16_from_usize(256);
    assert_eq!(encoded, vec![0x01, 0x00]);
}

#[test]
fn le_u16_from_usize_256() {
    let encoded = tinyklv::enc::binary::le_u16_from_usize(256);
    assert_eq!(encoded, vec![0x00, 0x01]);
}

#[test]
fn be_u32_from_usize_roundtrip() {
    for val in [0_usize, 1, 255, 65536] {
        let encoded = tinyklv::enc::binary::be_u32_from_usize(val);
        let decoded = tinyklv::dec::binary::be_u32_as_usize(&mut encoded.as_slice()).unwrap();
        assert_eq!(val, decoded);
    }
}

// --------------------------------------------------
// truncation: usize values larger than target type
// wrap/truncate
// --------------------------------------------------

#[test]
fn u8_from_usize_truncates() {
    // 256 as u8 = 0
    let encoded = tinyklv::enc::binary::u8_from_usize(256);
    assert_eq!(encoded, vec![0x00]);
}
