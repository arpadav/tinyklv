//! Direct `decode_partial` tests covering each return shape under the
//! new 2-arm `Packet` design.
//!
//! `decode_partial` returns `Result<Packet<T, P>, &'static str>`:
//!
//! * `Ok(Packet::NeedMore(p))` - the loop ran out of bytes mid-packet
//!   OR exited via top-of-loop EOF without a `Done` break condition.
//!   the partial `p` carries every field that landed; the caller
//!   chooses when to finalise (via `Decoder::finish`,
//!   `decode_value`, or `p.try_into()` directly).
//! * `Ok(Packet::Ready(t))` - only when the codegen takes a
//!   conversion-on-give-up path (key bytes consumed but undecodable,
//!   `take(len)` failed, or `BreakType::Done` fired) AND the
//!   partial finalises cleanly.
//! * `Err(label)` - the same conversion-on-give-up path took, but
//!   `Partial::finalize` returned a missing-required label.
//!
//! Tested without the `Decoder<P>` buffering layer so a regression in
//! the codegen mapping is caught on the shortest possible path.

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
/// A complete, well-formed input drives the loop to top-of-loop EOF
/// without a `Done` break condition. Result: `Ok(NeedMore(p))` where
/// `p` already carries every required field. Caller finalises
/// explicitly via `try_into()`.
fn decode_partial_complete_input_emits_needmore_with_full_partial() {
    let v = Pair { a: 3, b: 4 };
    let mut encoded = Vec::new();
    v.encode_value(&mut encoded);
    let mut cursor: &[u8] = encoded.as_slice();

    let p = match Pair::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(p)) => p,
        other => panic!("expected NeedMore, got {}", kind(&other)),
    };
    assert!(
        cursor.is_empty(),
        "cursor must advance past every consumed byte"
    );
    let got: Pair = p.try_into().expect("partial finalises cleanly");
    assert_eq!(got, v);
}

#[test]
/// A declared length that overruns remaining input yields
/// `Ok(NeedMore(_))` AND rewinds the cursor to the pre-attempt offset
/// so a later retry (with more bytes appended) picks up cleanly.
fn decode_partial_need_more_rewinds_cursor() {
    // key=0x01, len=0x05, but only 2 bytes follow
    let stream = [0x01u8, 0x05, 0xAA, 0xBB];
    let mut cursor: &[u8] = &stream;
    let before = cursor.len();

    match Pair::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(_)) => {}
        other => panic!("expected NeedMore, got {}", kind(&other)),
    }
    assert_eq!(
        cursor.len(),
        before,
        "cursor must be rewound to pre-attempt offset on NeedMore",
    );
}

#[test]
/// Key parses (1 byte consumed) but the len decoder runs into EOF.
/// Per the new contract that's recoverable - feed more bytes and the
/// next attempt will re-parse the key and len cleanly. So this is
/// `Ok(NeedMore)`, not an `Err(label)`.
fn decode_partial_short_len_is_recoverable() {
    let stream = [0x01u8];
    let mut cursor: &[u8] = &stream;
    let before = cursor.len();

    match Pair::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(_)) => {}
        other => panic!(
            "expected NeedMore (recoverable truncation), got {}",
            kind(&other)
        ),
    }
    assert_eq!(
        cursor.len(),
        before,
        "cursor must be rewound on key+len truncation",
    );
}

#[test]
/// A required field missing from an otherwise-complete packet does NOT
/// surface from `decode_partial` itself - finalisation is the caller's
/// call. The partial returned by `NeedMore` (or by `Ready` after a
/// `Done` break condition) carries `b = None`. Calling `try_into()`
/// then returns the missing-required label.
fn decode_partial_missing_required_label_via_try_into() {
    let stream = [0x01u8, 0x01, 0xAA];
    let mut cursor: &[u8] = &stream;

    let p = match Pair::decode_partial(&mut cursor) {
        Ok(Packet::NeedMore(p)) => p,
        other => panic!("expected NeedMore, got {}", kind(&other)),
    };
    let label: &'static str = <Pair as core::convert::TryFrom<_>>::try_from(p)
        .expect_err("partial must fail finalisation: required `b` is missing");
    assert!(
        label.contains("Pair::b"),
        "label must name the missing required field; got: {label}"
    );
}

#[test]
/// `decode_value` is the one-shot, fallible entry point: it runs
/// `decode_partial` then finalises automatically. Missing required ->
/// the static label flows into a `ContextError` with input context.
fn decode_value_one_shot_missing_required() {
    let stream = [0x01u8, 0x01, 0xAA];
    let mut cursor: &[u8] = &stream;
    let err = Pair::decode_value(&mut cursor)
        .expect_err("decode_value must fail when required `b` missing");
    let msg = format!("{err:?}");
    assert!(
        msg.contains("Pair::b"),
        "ContextError must surface the static label naming the missing field; got: {msg}"
    );
}

fn kind<T, P>(p: &Result<Packet<T, P>, &'static str>) -> &'static str
where
    P: tinyklv::Partial<Final = T>,
{
    match p {
        Ok(Packet::Ready(_)) => "Ok(Ready)",
        Ok(Packet::NeedMore(_)) => "Ok(NeedMore)",
        Err(_) => "Err(label)",
    }
}
