//! Unknown-key, duplicate-key and `Sliced` value-error tests for the fused
//! `#[klv(keylen(..))]` codec, through both drivers
//!
//! An unknown `Sliced` frame is skipped by its length; an unknown `Inline` frame ends the
//! loop at its header; `deny_unknown_keys` rejects either. A repeated key keeps its last
//! value, and a `Sliced` value whose decoder fails leaves its field unset
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::drive::Drive;
use crate::example::{Reading, Status};
use crate::pb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::cell::Cell;

/// [`Reading`] under `deny_unknown_keys`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen),
    allow_unimplemented_encode,
    deny_unknown_keys,
    default(typ = i32, dec = pb::dec_varint_i32),
    default(typ = String, dec = pb::dec_string),
)]
struct StrictReading {
    #[klv(key = 0x08)]
    /// field 1, wire 0 VARINT - `Inline`
    count: i32,

    #[klv(key = 0x12)]
    /// field 2, wire 2 LEN - `Sliced(n)`
    label: Option<String>,
}

/// Message with no required field, so a decode that reads no frame still finalizes
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = Sparse::header),
    allow_unimplemented_encode,
    default(typ = i32, dec = pb::dec_varint_i32),
)]
struct Sparse {
    #[klv(key = 0x08)]
    /// field 1, wire 0 VARINT - `Inline`
    count: Option<i32>,
}

impl Sparse {
    /// Header reads one test may make before its decode counts as non-terminating
    const HEADER_BUDGET: u32 = 64;

    /// [`pb::dec_tag_keylen`] under [`Sparse::HEADER_BUDGET`], so a decode loop that never
    /// advances panics instead of hanging the suite
    ///
    /// # Arguments
    ///
    /// * `input` - the byte stream positioned at a frame header; advanced past it on success
    ///
    /// # Returns
    ///
    /// The result of [`pb::dec_tag_keylen`]
    ///
    /// # Panics
    ///
    /// On the read past [`Sparse::HEADER_BUDGET`]
    fn header(input: &mut &[u8]) -> tinyklv::Result<(u32, Framing)> {
        thread_local! {
            /// Frame headers read on this test's thread
            static HEADERS_READ: Cell<u32> = const { Cell::new(0) };
        }
        let reads = HEADERS_READ.get() + 1;
        assert!(
            reads <= Self::HEADER_BUDGET,
            "{reads} header reads: the decode loop is not advancing"
        );
        HEADERS_READ.set(reads);
        pb::dec_tag_keylen(input)
    }
}

/// `deny_unknown_keys` rejects an unknown key whatever framing its header resolves to,
/// before a `Sliced` value is taken, with a label naming the struct
#[test]
fn deny_unknown_keys_rejects_sliced_and_inline() {
    #[rustfmt::skip]
    let sliced_unknown = [
        // count = 3
        0x08, 0x03,
        // unknown field 9, wire 2 LEN, Sliced(1)
        0x4A, 0x01, 0x00,
    ];
    #[rustfmt::skip]
    let sliced_unknown_cut = [
        // count = 3
        0x08, 0x03,
        // unknown field 9, wire 2 LEN, Sliced(5) - 1 of its 5 bytes
        0x4A, 0x05, 0x00,
    ];
    #[rustfmt::skip]
    let inline_unknown = [
        // count = 3
        0x08, 0x03,
        // unknown field 9, wire 0 VARINT, Inline
        0x48, 0x01,
    ];
    for wire in [&sliced_unknown[..], &sliced_unknown_cut, &inline_unknown] {
        StrictReading::assert_rejects_naming(wire, &["invalid key", "StrictReading"]);
    }
}

/// Without `deny_unknown_keys` an unknown `Inline` frame has nothing to skip it by:
/// the loop stops at its header and both drivers finalize what landed before it
#[test]
fn inline_unknown_ends_loop() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // unknown field 9, wire 0 VARINT, Inline
        0x48, 0x01,
        // label = "hi" - past the unknown frame, never reached
        0x12, 0x02, 0x68, 0x69,
    ];
    let mut cursor = &wire[..];
    match Reading::decode_partial(&mut cursor) {
        Ok(Packet::Ready(decoded)) => assert_eq!(decoded, Reading::count_only()),
        other => panic!("streaming: expected Ready, got {other:?}"),
    }
    assert_eq!(cursor, &wire[2..], "streaming stops at the unknown frame");
    let mut cursor = &wire[..];
    assert_eq!(
        Reading::decode_value(&mut cursor),
        Ok(Reading::count_only())
    );
    assert_eq!(cursor, &wire[2..], "one-shot stops at the unknown frame");
}

/// An unknown `Inline` frame first on the wire is a `Ready` that consumed nothing, so a
/// `Vec` of such messages must stop on it rather than collect empty elements forever
#[test]
fn inline_unknown_first_frame_is_ready_unmoved() {
    #[rustfmt::skip]
    let wire = [
        // unknown field 9, wire 0 VARINT, Inline
        0x48, 0x01,
        // count = 3 - past the unknown frame, never reached
        0x08, 0x03,
    ];
    let mut cursor = &wire[..];
    match Sparse::decode_partial(&mut cursor) {
        Ok(Packet::Ready(decoded)) => assert_eq!(decoded, Sparse { count: None }),
        other => panic!("single: expected Ready, got {other:?}"),
    }
    assert_eq!(cursor, &wire[..], "single decode must not move the cursor");
    let mut cursor = &wire[..];
    match Vec::<Sparse>::decode_partial(&mut cursor) {
        Ok(Packet::Ready(decoded)) => assert_eq!(decoded, []),
        other => panic!("vec: expected Ready, got {other:?}"),
    }
    assert_eq!(cursor, &wire[..], "vec decode must not move the cursor");
    let mut cursor = &wire[..];
    assert_eq!(Vec::<Sparse>::decode_value(&mut cursor), Ok(vec![]));
    assert_eq!(
        cursor,
        &wire[..],
        "one-shot vec decode must not move the cursor"
    );
}

/// Without `deny_unknown_keys` an unknown `Sliced` frame is skipped by its length and
/// the frames after it decode
#[test]
fn sliced_unknown_is_skipped() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // unknown field 9, wire 2 LEN, Sliced(1)
        0x4A, 0x01, 0x00,
        // label = "hi"
        0x12, 0x02, 0x68, 0x69,
    ];
    let expected = Reading {
        label: Some("hi".to_owned()),
        ..Reading::count_only()
    };
    assert_eq!(
        Reading::decode_value(&mut &wire[..]).as_ref(),
        Ok(&expected)
    );
    expected.assert_streams_to(&mut &wire[..]);
}

/// A key seen twice keeps its last value, for an `Inline` and a `Sliced` field alike
#[test]
fn duplicate_key_last_wins() {
    #[rustfmt::skip]
    let wire = [
        // count = 3
        0x08, 0x03,
        // label = "a"
        0x12, 0x01, 0x61,
        // count = 4
        0x08, 0x04,
        // label = "b"
        0x12, 0x01, 0x62,
    ];
    let expected = Reading {
        count: 4,
        label: Some("b".to_owned()),
        ..Reading::count_only()
    };
    assert_eq!(
        Reading::decode_value(&mut &wire[..]).as_ref(),
        Ok(&expected)
    );
    expected.assert_streams_to(&mut &wire[..]);
}

/// A `Sliced` value whose decoder fails leaves its field unset; the frame is already
/// bounded, so decoding continues with the next frame
#[test]
fn sliced_value_error_is_swallowed() {
    #[rustfmt::skip]
    let wire = [
        // label - 2 bytes of invalid UTF-8
        0x12, 0x02, 0xFF, 0xFE,
        // count = 3
        0x08, 0x03,
        // status = Error
        0x18, 0x02,
    ];
    let expected = Reading {
        status: Some(Status::Error),
        ..Reading::count_only()
    };
    assert_eq!(
        Reading::decode_value(&mut &wire[..]).as_ref(),
        Ok(&expected)
    );
    expected.assert_streams_to(&mut &wire[..]);
}
