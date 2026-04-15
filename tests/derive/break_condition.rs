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
struct BreakPacket {
    #[klv(key = 0x01, dec = tinyklv::dec::binary::be_u16, enc = enc_u16)]
    a: u16,
    #[klv(key = 0x02, dec = tinyklv::dec::binary::be_u32, enc = enc_u32)]
    b: Option<u32>,
}

// The default BreakCondition returns Proceed — all keys are processed.

#[test]
fn default_break_condition_proceeds_through_all_keys() {
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x2A, 0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let result = BreakPacket::decode(&mut &data[..]).unwrap();
    assert_eq!(result.a, 42);
    assert_eq!(result.b, Some(0xDEAD_BEEF));
}

#[test]
fn default_break_condition_unknown_key_skipped_not_aborted() {
    // Unknown key 0xAA (not in struct); default Proceed means loop continues.
    let data: &[u8] = &[
        0xAA, 0x01, 0x00, // unknown key, skipped
        0x01, 0x02, 0x00, 0x07, // known key a=7
    ];
    let result = BreakPacket::decode(&mut &data[..]).unwrap();
    assert_eq!(result.a, 7);
}

#[test]
fn default_break_condition_empty_stream_fails_required() {
    let result = BreakPacket::decode(&mut [].as_slice());
    assert!(result.is_err());
}

#[test]
fn default_break_condition_partial_stream_fails_required() {
    // Only optional field present — required `a` absent → Err
    let data: &[u8] = &[0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let result = BreakPacket::decode(&mut &data[..]);
    assert!(result.is_err());
}
