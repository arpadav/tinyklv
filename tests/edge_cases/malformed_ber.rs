// --------------------------------------------------
// malformed BER length encodings
// --------------------------------------------------
#[test]
fn ber_length_0xff_claims_127_extra_bytes() {
    // 0xFF = long form, claims 127 additional bytes; only 1 follows
    let mut input: &[u8] = &[0xFF, 0x01];
    assert!(
        tinyklv::dec::ber::ber_length(&mut input).is_err(),
        "0xFF with 1 following byte should fail (claims 127)"
    );
}

#[test]
fn ber_length_0x80_claims_zero_extra_bytes() {
    // 0x80 = long form with 0 extra bytes (indefinite form, not supported).
    // The decoder reads 0 additional bytes from the stream.
    // With 0 bytes to form a u128 from parse_length_u128, it folds to 0.
    // This actually succeeds and returns 0 - document the actual behavior.
    let mut input: &[u8] = &[0x80];
    let result = tinyklv::dec::ber::ber_length(&mut input);
    // Behavior: 0x80 has num_bytes=0; parse_length_u128 with 0 bytes
    // produces 0 via fold. This is technically "indefinite form" but
    // the library decodes it as length 0.
    match result {
        Ok(len) => assert_eq!(len, 0, "0x80 decodes as length 0"),
        Err(_) => { /* also acceptable if implementation rejects it */ }
    }
}

#[test]
fn ber_length_entirely_empty_fails() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}

#[test]
fn ber_length_large_count_with_insufficient_data() {
    // 0x8A = long form, claims 10 extra bytes; only 3 follow
    let mut input: &[u8] = &[0x8A, 0x01, 0x02, 0x03];
    assert!(tinyklv::dec::ber::ber_length(&mut input).is_err());
}
// --------------------------------------------------
// malformed BER OID encodings
// --------------------------------------------------
#[test]
fn ber_oid_empty_fails() {
    let mut input: &[u8] = &[];
    assert!(tinyklv::dec::ber::ber_oid::<u64>(&mut input).is_err());
}

#[test]
fn ber_oid_all_continuation_bytes_no_terminator() {
    // All bytes have MSB set (0x80, 0x80): take_while_msb_set consumes them all,
    // then take_one on empty input fails.
    let mut input: &[u8] = &[0x80, 0x80];
    assert!(tinyklv::dec::ber::ber_oid::<u64>(&mut input).is_err());
}

#[test]
fn ber_oid_single_continuation_byte_no_terminator() {
    // Single continuation byte with no terminator
    let mut input: &[u8] = &[0x81];
    assert!(tinyklv::dec::ber::ber_oid::<u64>(&mut input).is_err());
}

#[test]
fn ber_oid_three_continuation_bytes_no_terminator() {
    // 0x80 has MSB set - take_while_msb_set consumes all three, then take_one fails
    let mut input: &[u8] = &[0x80, 0x80, 0x80];
    assert!(tinyklv::dec::ber::ber_oid::<u64>(&mut input).is_err());
}
// --------------------------------------------------
// BER length: 0x81 alone (claims 1 byte, none available)
// --------------------------------------------------
#[test]
fn ber_length_0x81_no_following_byte() {
    // 0x81 = long form, 1 extra byte claimed; nothing follows
    let mut input: &[u8] = &[0x81];
    assert!(
        tinyklv::dec::ber::ber_length(&mut input).is_err(),
        "0x81 with no following byte should fail"
    );
}

#[test]
fn ber_length_0x82_only_one_byte_follows() {
    // 0x82 = long form, 2 bytes claimed; only 1 follows
    let mut input: &[u8] = &[0x82, 0x01];
    assert!(
        tinyklv::dec::ber::ber_length(&mut input).is_err(),
        "0x82 with only 1 byte should fail"
    );
}
// --------------------------------------------------
// large BER length roundtrip
// --------------------------------------------------
#[test]
fn ber_length_u16_max_roundtrip() {
    let val = u16::MAX as u64;
    let encoded = tinyklv::enc::ber::ber_length(&val);
    assert!(encoded.len() > 1, "u16::MAX must encode as long form");
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, val as usize);
}

#[test]
fn ber_length_u32_max_roundtrip() {
    let val = u32::MAX as u64;
    let encoded = tinyklv::enc::ber::ber_length(&val);
    assert!(encoded.len() > 1);
    let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, val as usize);
}

#[test]
fn ber_oid_large_value_roundtrip() {
    // Encode a 3-byte OID value (> 16383, needs 3 VLQ bytes)
    let val = 0x00_20_00_00_u64; // 2_097_152 - needs 4 VLQ bytes
    let encoded = tinyklv::enc::ber::ber_oid(&val);
    assert!(encoded.len() >= 3, "large OID needs multiple bytes");
    let decoded = tinyklv::dec::ber::ber_oid::<u64>(&mut encoded.as_slice()).unwrap();
    assert_eq!(decoded, val);
}

#[test]
fn ber_length_short_form_boundary_roundtrips() {
    // Every value 0..=127 must round-trip as 1 byte
    for v in 0u64..=127 {
        let encoded = tinyklv::enc::ber::ber_length(&v);
        assert_eq!(encoded.len(), 1, "value {v} should be short form (1 byte)");
        let decoded = tinyklv::dec::ber::ber_length(&mut encoded.as_slice()).unwrap();
        assert_eq!(decoded, v as usize, "short-form roundtrip failed for {v}");
    }
}
