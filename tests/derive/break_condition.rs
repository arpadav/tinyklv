// --------------------------------------------------
// local
// --------------------------------------------------
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
struct BreakPacket {
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

// The default BreakCondition returns Proceed - all keys are processed.

#[test]
/// Tests that the default `BreakCondition::Proceed` processes every key in the stream.
fn default_break_condition_proceeds_through_all_keys() {
    let data: &[u8] = &[0x01, 0x02, 0x00, 0x2A, 0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let result = BreakPacket::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.a, 42);
    assert_eq!(result.b, Some(0xDEAD_BEEF));
}

#[test]
/// Verifies that an unrecognized key is skipped rather than aborting the decode under the default break condition.
fn default_break_condition_unknown_key_skipped_not_aborted() {
    let data: &[u8] = &[
        0xAA, 0x01, 0x00, // unknown key, skipped
        0x01, 0x02, 0x00, 0x07, // known key a=7
    ];
    let result = BreakPacket::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.a, 7);
}

#[test]
/// Ensures that decoding an empty stream under the default break condition fails because the required field is missing.
fn default_break_condition_empty_stream_fails_required() {
    let result = BreakPacket::decode_value(&mut [].as_slice());
    assert!(result.is_err());
}

#[test]
/// Tests that decoding errors when only the optional field is present and the required field is absent.
fn default_break_condition_partial_stream_fails_required() {
    let data: &[u8] = &[0x02, 0x04, 0xDE, 0xAD, 0xBE, 0xEF];
    let result = BreakPacket::decode_value(&mut &data[..]);
    assert!(result.is_err());
}
