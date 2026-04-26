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
struct SingleField {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    value: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct TwoFields {
    #[klv(
        key = 0x01,
        dec = decb::be_u16,
        enc = *encb::be_u16,
    )]
    a: u16,
    #[klv(
        key = 0x02,
        dec = decb::be_u32,
        enc = *encb::be_u32,
    )]
    b: Option<u32>,
}

#[test]
/// Tests that when a key appears twice, the second (last) decoded value wins on a required `u16` field.
fn duplicate_key_last_wins() {
    // Key 0x01 appears twice: first value 0x0001, second 0x0002.
    // Last successful decode wins: result = 2.
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x01, // first occurrence: 1
        0x01, 0x02, 0x00, 0x02, // second occurrence: 2 (last wins)
    ];
    let result = SingleField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.value, 2, "last occurrence should win");
}

#[test]
/// Tests that last-wins semantics hold across three consecutive occurrences of the same key.
fn duplicate_key_three_times_last_wins() {
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x0A, // 10
        0x01, 0x02, 0x00, 0x14, // 20
        0x01, 0x02, 0x00, 0x1E, // 30 (last wins)
    ];
    let result = SingleField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.value, 30, "last of three occurrences should win");
}

#[test]
/// Sanity check that a single occurrence of a key decodes normally when no duplicates are present.
fn duplicate_key_single_occurrence_works_normally() {
    let data: &[u8] = &[0x01, 0x02, 0xAB, 0xCD];
    let result = SingleField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.value, 0xABCD);
}

#[test]
/// Tests idempotency: repeating the same value yields that value regardless of last-wins ordering.
fn duplicate_key_same_value_both_times() {
    // Idempotent: same value repeated, result unchanged regardless of last-wins.
    let data: &[u8] = &[0x01, 0x02, 0xFF, 0xFF, 0x01, 0x02, 0xFF, 0xFF];
    let result = SingleField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.value, u16::MAX);
}

#[test]
/// Tests that duplicating the first key does not disturb decoding of a distinct second key.
fn duplicate_of_first_field_other_field_unaffected() {
    // key 0x01 appears twice (last wins = 2), key 0x02 appears once.
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x01, // a = 1
        0x01, 0x02, 0x00, 0x02, // a = 2 (last wins)
        0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF, // b = 0xDEADBEEF
    ];
    let result = TwoFields::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.a, 2);
    assert_eq!(result.b, Some(0xDEAD_BEEF));
}

#[test]
/// Tests that duplicating the second key does not disturb the already-decoded first key.
fn duplicate_of_second_field_other_field_unaffected() {
    // key 0x02 appears twice, last wins = 2.
    let data: &[u8] = &[
        0x01, 0x02, 0xBE, 0xEF, // a = 0xBEEF
        0x02, 0x04, 0x00, 0x00, 0x00, 0x01, // b = 1
        0x02, 0x04, 0x00, 0x00, 0x00, 0x02, // b = 2 (last wins)
    ];
    let result = TwoFields::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.a, 0xBEEF);
    assert_eq!(result.b, Some(2));
}

#[test]
/// Tests that interleaved duplicates of two distinct keys each resolve to their own last-occurrence value.
fn interleaved_duplicates_last_wins_for_each() {
    // Both fields duplicated and interleaved; last value for each key wins.
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x0A, // a = 10
        0x02, 0x04, 0x00, 0x00, 0x00, 0x64, // b = 100
        0x01, 0x02, 0x00, 0x14, // a = 20 (last wins)
        0x02, 0x04, 0x00, 0x00, 0x00, 0xC8, // b = 200 (last wins)
    ];
    let result = TwoFields::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.a, 20);
    assert_eq!(result.b, Some(200));
}

#[test]
/// Tests last-wins when the final occurrence is the boundary maximum (`u16::MAX`).
fn duplicate_key_max_value_last() {
    // Last occurrence is the max value.
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x00, // 0
        0x01, 0x02, 0xFF, 0xFF, // u16::MAX (last wins)
    ];
    let result = SingleField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.value, u16::MAX);
}

#[test]
/// Tests last-wins when the final occurrence is zero, overriding a prior non-zero value.
fn duplicate_key_zero_last() {
    // Last occurrence is zero.
    let data: &[u8] = &[
        0x01, 0x02, 0xFF, 0xFF, // u16::MAX
        0x01, 0x02, 0x00, 0x00, // 0 (last wins)
    ];
    let result = SingleField::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.value, 0_u16);
}
