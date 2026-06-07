//! `#[klv(break_on = ..)]` container-attribute tests
//!
//! Exercises both forms of the attribute - a key literal (stop the loop when a
//! decoded key matches) and a function `fn(key, len) -> BreakType` - across the
//! one-shot `decode_value` path AND the streaming `decoder()`/feed path,
//! confirming the loop control fires identically. Field types are the shared
//! custom enums (`Color`, `Priority`), not bare primitives.
// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// literal form: stop the loop on key 0xFF
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    break_on = 0xFF,
)]
struct LitTerminated {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
    #[klv(key = 0x02, dec = Priority::decode_value, enc = Priority::encode_value)]
    priority: Option<Priority>,
}

#[test]
/// A literal `break_on = 0xFF` stops `decode_value` when key `0xFF` is read, so a decoy `0x01`
/// (color) triple planted *after* the terminator must not overwrite the already-decoded color.
fn literal_terminator_stops_oneshot_before_decoy() {
    let mut body = Vec::new();
    LitTerminated {
        color: Color::Blue,
        priority: Some(Priority::High),
    }
    .encode_value(&mut body);
    // terminator triple (key 0xFF, zero len), then a decoy color triple that would set
    // `color = Color::Unknown(0xDEAD)` if the loop did not stop
    body.extend_from_slice(&[0xFF, 0x00]);
    body.extend_from_slice(&[0x01, 0x02, 0xDE, 0xAD]);

    let decoded = LitTerminated::decode_value(&mut body.as_slice()).unwrap();
    assert_eq!(
        decoded,
        LitTerminated {
            color: Color::Blue,
            priority: Some(Priority::High),
        },
        "decode continued past the 0xFF terminator and consumed the decoy triple",
    );
}

// --------------------------------------------------
// streaming parity: same literal terminator, framed + fed through `decoder()`
// --------------------------------------------------

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"BRK",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    break_on = 0xFF,
)]
struct StreamTerminated {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
    #[klv(key = 0x02, dec = Priority::decode_value, enc = Priority::encode_value)]
    priority: Option<Priority>,
}

#[test]
/// The streaming `decoder()` path honours `break_on` identically to the one-shot path: a framed
/// body whose declared length spans a terminator plus trailing decoy bytes still decodes to the
/// pre-terminator fields. Junk before the sentinel exercises the seek.
fn literal_terminator_stops_streaming_before_decoy() {
    let mut inner = Vec::new();
    StreamTerminated {
        color: Color::Blue,
        priority: Some(Priority::High),
    }
    .encode_value(&mut inner);
    // terminator + decoy color triple, both inside the framed body length
    inner.extend_from_slice(&[0xFF, 0x00]);
    inner.extend_from_slice(&[0x01, 0x02, 0xDE, 0xAD]);

    let mut frame = Vec::new();
    // pre-sentinel junk the seek must skip
    frame.extend_from_slice(&[0x00, 0x00, 0x99]);
    frame.extend_from_slice(b"BRK");
    frame.push(u8::try_from(inner.len()).unwrap());
    frame.extend_from_slice(&inner);

    let mut dec = StreamTerminated::decoder();
    dec.feed(&frame);
    let got: StreamTerminated = dec.next().expect("a complete frame should decode");
    assert_eq!(
        got,
        StreamTerminated {
            color: Color::Blue,
            priority: Some(Priority::High),
        },
    );
}

// --------------------------------------------------
// function form: Skip + Done outcomes
// --------------------------------------------------

/// Reserved key `0xFE` is consumed-and-skipped; terminator `0xFF` stops the loop.
fn skip_then_done(key: u8, _len: usize) -> BreakType {
    match key {
        0xFE => BreakType::Skip,
        0xFF => BreakType::Done,
        _ => BreakType::Proceed,
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    break_on = skip_then_done,
)]
struct FnControlled {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
    #[klv(key = 0x02, dec = Priority::decode_value, enc = Priority::encode_value)]
    priority: Option<Priority>,
}

#[test]
/// A `break_on` function: a reserved `0xFE` triple is skipped (its bytes consumed so the following
/// fields stay aligned), the real fields decode, and a `0xFF` terminator stops the loop before the
/// trailing decoy.
fn function_skip_and_done() {
    // reserved triple first (key 0xFE, len 3, garbage) - must be skipped, not misalign the rest
    let mut body = vec![0xFE, 0x03, 0x11, 0x22, 0x33];
    FnControlled {
        color: Color::Green,
        priority: Some(Priority::Critical),
    }
    .encode_value(&mut body);
    body.extend_from_slice(&[0xFF, 0x00]);
    body.extend_from_slice(&[0x01, 0x02, 0xDE, 0xAD]);

    let decoded = FnControlled::decode_value(&mut body.as_slice()).unwrap();
    assert_eq!(
        decoded,
        FnControlled {
            color: Color::Green,
            priority: Some(Priority::Critical),
        },
    );
}

// --------------------------------------------------
// function form: Abort outcome
// --------------------------------------------------

/// Forbidden key `0xEE` aborts the decode with a static message.
fn abort_on_forbidden(key: u8, _len: usize) -> BreakType {
    if key == 0xEE {
        BreakType::Abort("break_on_attr: forbidden key 0xEE")
    } else {
        BreakType::Proceed
    }
}

#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    break_on = abort_on_forbidden,
)]
struct Abortable {
    #[klv(key = 0x01, dec = Color::decode_value, enc = Color::encode_value)]
    color: Color,
}

#[test]
/// `BreakType::Abort` returns `Err` from `decode_value` even though the required field already
/// decoded - abort surfaces no partial value - and the static message rides out in the error.
fn function_abort_errors_with_message() {
    let mut body = Vec::new();
    Abortable { color: Color::Red }.encode_value(&mut body);
    // a forbidden 0xEE triple after the (complete) required field
    body.extend_from_slice(&[0xEE, 0x00]);

    let result = Abortable::decode_value(&mut body.as_slice());
    assert!(
        result.is_err(),
        "Abort must force an Err, not surface the decoded value"
    );
    let rendered = format!("{}", result.unwrap_err());
    assert!(
        rendered.contains("forbidden key 0xEE"),
        "abort message should surface in the error, got: {rendered}",
    );
}
