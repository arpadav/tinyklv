// --------------------------------------------------
// 1-byte inputs for multi-byte decoders
// --------------------------------------------------
#[test]
/// Tests that `be_u16` errors with only 1 byte (needs 2).
fn be_u16_one_byte_fails() {
    let mut input: &[u8] = &[0x01];
    assert!(tinyklv::dec::binary::be_u16(&mut input).is_err());
}

#[test]
/// Tests that `be_u32` errors with only 1 byte (needs 4).
fn be_u32_one_byte_fails() {
    let mut input: &[u8] = &[0x01];
    assert!(tinyklv::dec::binary::be_u32(&mut input).is_err());
}

#[test]
/// Tests that `be_u32` errors with 3 bytes (needs 4).
fn be_u32_three_bytes_fails() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03];
    assert!(tinyklv::dec::binary::be_u32(&mut input).is_err());
}

#[test]
/// Tests that `be_u64` errors with only 1 byte (needs 8).
fn be_u64_one_byte_fails() {
    let mut input: &[u8] = &[0x01];
    assert!(tinyklv::dec::binary::be_u64(&mut input).is_err());
}

#[test]
/// Tests that `be_u64` errors with 7 bytes (needs 8).
fn be_u64_seven_bytes_fails() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
    assert!(tinyklv::dec::binary::be_u64(&mut input).is_err());
}

#[test]
/// Tests that `be_u128` errors with 15 bytes (needs 16).
fn be_u128_fifteen_bytes_fails() {
    let mut input: &[u8] = &[0u8; 15];
    assert!(tinyklv::dec::binary::be_u128(&mut input).is_err());
}

#[test]
/// Tests that `be_f32` errors with 3 bytes (needs 4).
fn be_f32_three_bytes_fails() {
    let mut input: &[u8] = &[0x3F, 0x80, 0x00];
    assert!(tinyklv::dec::binary::be_f32(&mut input).is_err());
}

#[test]
/// Tests that `be_f64` errors with 7 bytes (needs 8).
fn be_f64_seven_bytes_fails() {
    let mut input: &[u8] = &[0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(tinyklv::dec::binary::be_f64(&mut input).is_err());
}

#[test]
/// Tests that `le_u32` errors with 2 bytes (needs 4).
fn le_u32_two_bytes_fails() {
    let mut input: &[u8] = &[0x01, 0x02];
    assert!(tinyklv::dec::binary::le_u32(&mut input).is_err());
}

#[test]
/// Tests that `le_u64` errors with only 1 byte (needs 8).
fn le_u64_one_byte_fails() {
    let mut input: &[u8] = &[0x01];
    assert!(tinyklv::dec::binary::le_u64(&mut input).is_err());
}
// --------------------------------------------------
// lengthed decoder: asks for more bytes than available
// --------------------------------------------------
#[test]
/// Tests that `be_u16_lengthed(4)` errors when only 2 bytes are in the input.
fn be_u16_lengthed_asks_more_than_available() {
    // Decoder wants 4 bytes but only 2 in input
    let mut input: &[u8] = &[0x01, 0x02];
    assert!(tinyklv::dec::binary::be_u16_lengthed(4)(&mut input).is_err());
}

#[test]
/// Tests that `be_u32_lengthed(6)` errors when only 3 bytes are in the input.
fn be_u32_lengthed_asks_more_than_available() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03];
    assert!(tinyklv::dec::binary::be_u32_lengthed(6)(&mut input).is_err());
}
// --------------------------------------------------
// BER long form: first byte claims N extra bytes but
// fewer are available
// --------------------------------------------------
#[test]
/// Tests that BER prefix `0x82` (claims 2 extra bytes) errors when only 1 byte follows.
fn ber_length_truncated_claims_2_has_1() {
    // 0x82 = long form, 2 extra bytes claimed; only 1 byte follows
    let mut input: &[u8] = &[0x82, 0x01];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}

#[test]
/// Tests that BER prefix `0x83` (claims 3 extra bytes) errors when only 2 bytes follow.
fn ber_length_truncated_claims_3_has_2() {
    // 0x83 = long form, 3 extra bytes; only 2 follow
    let mut input: &[u8] = &[0x83, 0x01, 0x02];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}

#[test]
/// Tests that BER prefix `0x84` (claims 4 extra bytes) errors when no bytes follow.
fn ber_length_truncated_claims_4_has_0() {
    // 0x84 = long form, 4 extra bytes; none follow
    let mut input: &[u8] = &[0x84];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}
// --------------------------------------------------
// to_string_utf8 with truncated input
// --------------------------------------------------
#[test]
/// Tests that `le_u16` errors with only 1 byte (needs 2).
fn le_u16_one_byte_fails() {
    let mut input: &[u8] = &[0x01];
    assert!(tinyklv::dec::binary::le_u16(&mut input).is_err());
}

#[test]
/// Tests that `le_u64` errors with 3 bytes (needs 8).
fn le_u64_three_bytes_fails() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03];
    assert!(tinyklv::dec::binary::le_u64(&mut input).is_err());
}

#[test]
/// Tests that `be_u128` errors with 7 bytes (needs 16).
fn be_u128_seven_bytes_fails() {
    let mut input: &[u8] = &[0u8; 7];
    assert!(tinyklv::dec::binary::be_u128(&mut input).is_err());
}

#[test]
/// Tests that `be_i128` errors with 15 bytes (needs 16).
fn be_i128_fifteen_bytes_fails() {
    let mut input: &[u8] = &[0u8; 15];
    assert!(tinyklv::dec::binary::be_i128(&mut input).is_err());
}

#[test]
/// Tests that `be_u16_as_usize` errors with only 1 byte (needs 2).
fn be_u16_as_usize_one_byte_fails() {
    let mut input: &[u8] = &[0x01];
    assert!(tinyklv::dec::binary::be_u16_as_usize(&mut input).is_err());
}

#[test]
/// Tests that `be_u32_as_usize` errors with 3 bytes (needs 4).
fn be_u32_as_usize_three_bytes_fails() {
    let mut input: &[u8] = &[0x01, 0x02, 0x03];
    assert!(tinyklv::dec::binary::be_u32_as_usize(&mut input).is_err());
}

#[test]
/// Tests that `le_u16_as_usize` errors with only 1 byte (needs 2).
fn le_u16_as_usize_one_byte_fails() {
    let mut input: &[u8] = &[0xAB];
    assert!(tinyklv::dec::binary::le_u16_as_usize(&mut input).is_err());
}

#[test]
/// Tests that `to_string_utf8(4)` errors when only 2 bytes are available.
fn to_string_utf8_truncated() {
    let mut input: &[u8] = &[0x41, 0x42]; // Only 2 bytes but ask for 4
    assert!(tinyklv::dec::binary::to_string_utf8(4)(&mut input).is_err());
}

#[test]
/// Tests that `to_string_utf16_le(4)` errors when only 2 bytes are available.
fn to_string_utf16_le_truncated() {
    let mut input: &[u8] = &[0x41, 0x00]; // Only 2 bytes, ask for 4
    assert!(tinyklv::dec::binary::to_string_utf16_le(4)(&mut input).is_err());
}

#[test]
/// Tests that `be_f32` errors with only 1 byte (needs 4).
fn be_f32_one_byte_fails() {
    let mut input: &[u8] = &[0x3F];
    assert!(tinyklv::dec::binary::be_f32(&mut input).is_err());
}

#[test]
/// Tests that `be_f64` errors with 3 bytes (needs 8).
fn be_f64_three_bytes_fails() {
    let mut input: &[u8] = &[0x3F, 0xF0, 0x00];
    assert!(tinyklv::dec::binary::be_f64(&mut input).is_err());
}
