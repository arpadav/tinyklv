#[test]
fn be_u16_lengthed_exact() {
    let mut input: &[u8] = &[0x01, 0xE0];
    assert_eq!(
        tinyklv::dec::binary::be_u16_lengthed(2)(&mut input),
        Ok(0x01E0_u16)
    );
}

#[test]
fn be_u16_lengthed_truncate() {
    // len=4 > native 2 bytes: takes last 2 of the 4 consumed bytes
    let mut input1: &[u8] = &[0x01, 0xE0, 0xFF, 0xFF];
    assert_eq!(
        tinyklv::dec::binary::be_u16_lengthed(4)(&mut input1),
        Ok(0xFFFF_u16)
    );
}

#[test]
fn be_u16_lengthed_pad() {
    // len=1 < native 2 bytes: zero-pads the high byte
    let mut input: &[u8] = &[0xE0];
    assert_eq!(
        tinyklv::dec::binary::be_u16_lengthed(1)(&mut input),
        Ok(0x00E0_u16)
    );
}

#[test]
fn be_u16_lengthed_doctest() {
    let mut input1: &[u8] = &[0x01, 0xE0, 0xFF, 0xFF];
    let mut input2: &[u8] = &[0x01, 0xE0];
    let mut input3: &[u8] = &[0xE0];
    assert_eq!(
        tinyklv::dec::binary::be_u16_lengthed(4)(&mut input1),
        Ok(0xFFFF)
    );
    assert_eq!(
        tinyklv::dec::binary::be_u16_lengthed(2)(&mut input2),
        Ok(0x01E0)
    );
    assert_eq!(
        tinyklv::dec::binary::be_u16_lengthed(1)(&mut input3),
        Ok(0x00E0)
    );
}

#[test]
fn be_u32_lengthed_exact() {
    let mut input: &[u8] = &[0x00, 0x01, 0xE0, 0xFF];
    assert_eq!(
        tinyklv::dec::binary::be_u32_lengthed(4)(&mut input),
        Ok(0x0001E0FF_u32)
    );
}

#[test]
fn be_u32_lengthed_truncate() {
    // len=5: takes last 4 of the 5 consumed bytes
    let mut input: &[u8] = &[0x00, 0x01, 0xE0, 0xFF, 0xFF];
    assert_eq!(
        tinyklv::dec::binary::be_u32_lengthed(5)(&mut input),
        Ok(0x01E0FFFF_u32)
    );
}

#[test]
fn be_u32_lengthed_pad() {
    // len=3: zero-pads high byte
    let mut input: &[u8] = &[0x01, 0xE0, 0xFF];
    assert_eq!(
        tinyklv::dec::binary::be_u32_lengthed(3)(&mut input),
        Ok(0x0001E0FF_u32)
    );
}

#[test]
fn be_u64_lengthed_exact() {
    let val: u64 = 0x0001020304050607;
    let bytes = val.to_be_bytes();
    let mut input: &[u8] = &bytes;
    assert_eq!(
        tinyklv::dec::binary::be_u64_lengthed(8)(&mut input),
        Ok(val)
    );
}

#[test]
fn be_u64_lengthed_truncate() {
    // len=9: last 8 bytes
    let mut input: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00, 0x00];
    assert_eq!(
        tinyklv::dec::binary::be_u64_lengthed(9)(&mut input),
        Ok(0x00_01_E0_FF_FF_00_00_00_u64)
    );
}

#[test]
fn be_u64_lengthed_pad() {
    // len=1: zero-pads 7 high bytes
    let mut input: &[u8] = &[0xAB];
    assert_eq!(
        tinyklv::dec::binary::be_u64_lengthed(1)(&mut input),
        Ok(0x00_00_00_00_00_00_00_AB_u64)
    );
}

#[test]
fn le_u16_lengthed_exact() {
    let mut input: &[u8] = &[0xE0, 0x01];
    assert_eq!(
        tinyklv::dec::binary::le_u16_lengthed(2)(&mut input),
        Ok(0x01E0_u16)
    );
}

#[test]
fn le_u16_lengthed_truncate() {
    // len=5: takes first 2 bytes in LE
    let mut input: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF];
    assert_eq!(
        tinyklv::dec::binary::le_u16_lengthed(5)(&mut input),
        Ok(480_u16) // 0x01E0 = 480
    );
}

#[test]
fn le_u16_lengthed_pad() {
    // len=1: zero-pads high byte
    let mut input: &[u8] = &[0x01];
    assert_eq!(
        tinyklv::dec::binary::le_u16_lengthed(1)(&mut input),
        Ok(1_u16)
    );
}

#[test]
fn le_u32_lengthed_exact() {
    let mut full_input: &[u8] = &[0x01, 0x02, 0x03];
    assert_eq!(
        tinyklv::dec::binary::le_u32_lengthed(3)(&mut full_input),
        Ok(197_121_u32)
    );
}

#[test]
fn le_u32_lengthed_truncate() {
    // len=5: takes first 4 bytes in LE
    let mut input: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF];
    assert_eq!(
        tinyklv::dec::binary::le_u32_lengthed(5)(&mut input),
        Ok(4_294_902_240_u32)
    );
}

#[test]
fn le_u32_lengthed_single_byte() {
    let mut input: &[u8] = &[0x01];
    assert_eq!(
        tinyklv::dec::binary::le_u32_lengthed(1)(&mut input),
        Ok(1_u32)
    );
}

#[test]
fn enc_be_u32_lengthed_exact() {
    assert_eq!(
        tinyklv::enc::binary::be_u32_lengthed(4)(0x0001E0FF_u32),
        vec![0x00, 0x01, 0xE0, 0xFF]
    );
}

#[test]
fn enc_be_u32_lengthed_truncate() {
    // len=3: strips high byte 0x00
    assert_eq!(
        tinyklv::enc::binary::be_u32_lengthed(3)(0x0001E0FF_u32),
        vec![0x01, 0xE0, 0xFF]
    );
}

#[test]
fn enc_be_u32_lengthed_pad() {
    // len=5: zero-pads left
    assert_eq!(
        tinyklv::enc::binary::be_u32_lengthed(5)(0x01E0FFFF_u32),
        vec![0x00, 0x01, 0xE0, 0xFF, 0xFF]
    );
}

#[test]
fn enc_le_u16_lengthed_truncate() {
    // len=5 but u16 is 2 bytes: result is 2 LE bytes + 3 zero-pads
    let result = tinyklv::enc::binary::le_u16_lengthed(5)(480_u16);
    assert_eq!(&result[..2], &[0xE0, 0x01]);
}

#[test]
fn enc_le_u16_lengthed_single_byte() {
    let result = tinyklv::enc::binary::le_u16_lengthed(1)(1_u16);
    assert_eq!(&result[..1], &[0x01]);
}

#[test]
fn be_u16_lengthed_zero_len() {
    // consuming 0 bytes should decode to 0
    let mut input: &[u8] = &[0xAB, 0xCD];
    assert_eq!(
        tinyklv::dec::binary::be_u16_lengthed(0)(&mut input),
        Ok(0_u16)
    );
}

#[test]
fn enc_be_u16_lengthed_zero_len() {
    let result = tinyklv::enc::binary::be_u16_lengthed(0)(0x1234_u16);
    assert!(result.is_empty());
}
