//! Direct `decode_partial` tests exercising each `Progress` variant.
//!
//! Covers the three-way return explicitly, without the `Decoder<T>`
//! buffering layer, so a regression in the codegen's `Progress` mapping
//! is caught on the shortest possible path.

use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Pair {
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    a: u8,
    #[klv(key = 0x02, dec = decb::u8, enc = *encb::u8)]
    b: u8,
}

#[test]
/// A complete, well-formed input yields `Progress::Ready(_)` and advances
/// the cursor past every consumed byte.
fn decode_partial_ready_on_complete() {
    let v = Pair { a: 3, b: 4 };
    let encoded = v.encode_value();
    let mut cursor: &[u8] = encoded.as_slice();

    match Pair::decode_partial(&mut cursor) {
        Progress::Ready(got) => assert_eq!(got, v),
        other => panic!("expected Ready, got {}", kind(&other)),
    }
    assert!(cursor.is_empty(), "cursor must advance past consumed bytes");
}

#[test]
/// A declared length that overruns remaining input yields
/// `Progress::NeedMore(_)` AND rewinds the cursor to the pre-attempt
/// offset so a later retry (with more bytes appended) picks up cleanly.
fn decode_partial_need_more_rewinds_cursor() {
    // key=0x01, len=0x05, but only 2 bytes follow
    let stream = [0x01u8, 0x05, 0xAA, 0xBB];
    let mut cursor: &[u8] = &stream;
    let before = cursor.len();

    match Pair::decode_partial(&mut cursor) {
        Progress::NeedMore(_) => {}
        other => panic!("expected NeedMore, got {}", kind(&other)),
    }
    assert_eq!(
        cursor.len(),
        before,
        "cursor must be rewound to pre-attempt offset on NeedMore",
    );
}

#[test]
/// Bytes present but unparseable at the key/len layer yield
/// `Progress::Malformed(_)`. Here we feed a single byte which is enough
/// for the `u8` key decoder but then the `u8_as_usize` len decoder fails
/// on empty input - a malformed shape, not truncation of a declared
/// length.
fn decode_partial_malformed_on_bad_keylen() {
    // one byte is consumed by the key decoder, then len decoder fails on
    // empty - that's malformed per our contract (we don't know how many
    // bytes would have been "enough" for an unrelated continuation).
    let stream = [0x01u8];
    let mut cursor: &[u8] = &stream;

    match Pair::decode_partial(&mut cursor) {
        Progress::Malformed(_) => {}
        other => panic!("expected Malformed, got {}", kind(&other)),
    }
}

#[test]
/// A required field missing from an otherwise-complete packet surfaces
/// as `Progress::Malformed(_)` from the `items_set_progress` guard. Here
/// only key `0x01` is present; `b` is required.
fn decode_partial_missing_required_is_malformed() {
    let stream = [0x01u8, 0x01, 0xAA];
    let mut cursor: &[u8] = &stream;

    match Pair::decode_partial(&mut cursor) {
        Progress::Malformed(_) => {}
        other => panic!("expected Malformed, got {}", kind(&other)),
    }
}

fn kind<T>(p: &Progress<T>) -> &'static str {
    match p {
        Progress::Ready(_) => "Ready",
        Progress::NeedMore(_) => "NeedMore",
        Progress::Malformed(_) => "Malformed",
    }
}
