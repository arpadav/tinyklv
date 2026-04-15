// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::prelude::*;
use tinyklv::Klv;

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
struct SingleField {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    value: u16,
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = tinyklv::dec::binary::be_u8, enc = tinyklv::enc::binary::u8),
    len(dec = tinyklv::dec::binary::be_u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct TwoFields {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    a: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    b: Option<u32>,
}

#[test]
fn duplicate_key_last_wins() {
    // Key 0x01 appears twice: first value 0x0001, second 0x0002.
    // Last successful decode wins: result = 2.
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x01, // first occurrence: 1
        0x01, 0x02, 0x00, 0x02, // second occurrence: 2 (last wins)
    ];
    let result = SingleField::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, 2, "last occurrence should win");
}

#[test]
fn duplicate_key_three_times_last_wins() {
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x0A, // 10
        0x01, 0x02, 0x00, 0x14, // 20
        0x01, 0x02, 0x00, 0x1E, // 30 (last wins)
    ];
    let result = SingleField::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, 30, "last of three occurrences should win");
}

#[test]
fn duplicate_key_single_occurrence_works_normally() {
    let data: &[u8] = &[0x01, 0x02, 0xAB, 0xCD];
    let result = SingleField::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, 0xABCD);
}

#[test]
fn duplicate_key_same_value_both_times() {
    // Idempotent: same value repeated, result unchanged regardless of last-wins.
    let data: &[u8] = &[0x01, 0x02, 0xFF, 0xFF, 0x01, 0x02, 0xFF, 0xFF];
    let result = SingleField::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, u16::MAX);
}

#[test]
fn duplicate_of_first_field_other_field_unaffected() {
    // key 0x01 appears twice (last wins = 2), key 0x02 appears once.
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x01, // a = 1
        0x01, 0x02, 0x00, 0x02, // a = 2 (last wins)
        0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF, // b = 0xDEADBEEF
    ];
    let result = TwoFields::decode(&mut &data[..]).unwrap();
    assert_eq!(result.a, 2);
    assert_eq!(result.b, Some(0xDEAD_BEEF));
}

#[test]
fn duplicate_of_second_field_other_field_unaffected() {
    // key 0x02 appears twice, last wins = 2.
    let data: &[u8] = &[
        0x01, 0x02, 0xBE, 0xEF, // a = 0xBEEF
        0x02, 0x04, 0x00, 0x00, 0x00, 0x01, // b = 1
        0x02, 0x04, 0x00, 0x00, 0x00, 0x02, // b = 2 (last wins)
    ];
    let result = TwoFields::decode(&mut &data[..]).unwrap();
    assert_eq!(result.a, 0xBEEF);
    assert_eq!(result.b, Some(2));
}

#[test]
fn interleaved_duplicates_last_wins_for_each() {
    // Both fields duplicated and interleaved; last value for each key wins.
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x0A, // a = 10
        0x02, 0x04, 0x00, 0x00, 0x00, 0x64, // b = 100
        0x01, 0x02, 0x00, 0x14, // a = 20 (last wins)
        0x02, 0x04, 0x00, 0x00, 0x00, 0xC8, // b = 200 (last wins)
    ];
    let result = TwoFields::decode(&mut &data[..]).unwrap();
    assert_eq!(result.a, 20);
    assert_eq!(result.b, Some(200));
}

#[test]
fn duplicate_key_max_value_last() {
    // Last occurrence is the max value.
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x00, // 0
        0x01, 0x02, 0xFF, 0xFF, // u16::MAX (last wins)
    ];
    let result = SingleField::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, u16::MAX);
}

#[test]
fn duplicate_key_zero_last() {
    // Last occurrence is zero.
    let data: &[u8] = &[
        0x01, 0x02, 0xFF, 0xFF, // u16::MAX
        0x01, 0x02, 0x00, 0x00, // 0 (last wins)
    ];
    let result = SingleField::decode(&mut &data[..]).unwrap();
    assert_eq!(result.value, 0_u16);
}
