//! Manual loop-control tests (the hand-written escape hatch)
//!
//! Tests `Done`, `Abort`, and `Skip` loop control via manual `DecodeValue`
//! implementations that embed the break logic directly in the loop. This is
//! the low-level escape hatch; the derive-level mechanism is the
//! `#[klv(break_on = ..)]` container attribute (covered in `break_on_attr.rs`),
//! which threads a [`tinyklv::BreakType`] decision into the generated loop.
//! These manual impls remain fully supported for cases the attribute does not
//! cover, and pin the same outcomes the attribute produces.
// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;

#[derive(Debug, PartialEq, Default)]
struct BreakOnDone {
    color: Option<Color>,
    priority: Option<Priority>,
}
impl tinyklv::DecodeValue<&[u8]> for BreakOnDone {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut color: Option<Color> = None;
        let mut priority: Option<Priority> = None;
        loop {
            let key = match decb::u8(input) {
                Ok(k) => k,
                Err(_) => break,
            };
            let len = match decb::u8_as_usize(input) {
                Ok(l) => l,
                Err(_) => break,
            };
            // Done: stop the loop, attempt to return what we have
            if key == 0xFF {
                break;
            }
            match key {
                0x01 => {
                    color = Color::decode_value(input).ok().or(color);
                }
                0x02 => {
                    priority = Priority::decode_value(input).ok().or(priority);
                }
                _ => {
                    let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                }
            }
        }
        Ok(BreakOnDone { color, priority })
    }
}

#[derive(Debug, PartialEq, Default)]
struct BreakOnAbort {
    color: Option<Color>,
    priority: Option<Priority>,
}
impl tinyklv::DecodeValue<&[u8]> for BreakOnAbort {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut color: Option<Color> = None;
        let mut priority: Option<Priority> = None;
        loop {
            let key = match decb::u8(input) {
                Ok(k) => k,
                Err(_) => break,
            };
            let len = match decb::u8_as_usize(input) {
                Ok(l) => l,
                Err(_) => break,
            };

            // Abort: return an error immediately, no partial result
            if key == 0xFE {
                return Err(winnow::error::ParserError::from_input(input));
            }

            match key {
                0x01 => {
                    color = Color::decode_value(input).ok().or(color);
                }
                0x02 => {
                    priority = Priority::decode_value(input).ok().or(priority);
                }
                _ => {
                    let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                }
            }
        }

        Ok(BreakOnAbort { color, priority })
    }
}

#[derive(Debug, PartialEq, Default)]
struct BreakOnSkip {
    color: Option<Color>,
    priority: Option<Priority>,
}
impl tinyklv::DecodeValue<&[u8]> for BreakOnSkip {
    fn decode_value(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut color: Option<Color> = None;
        let mut priority: Option<Priority> = None;

        loop {
            let key = match decb::u8(input) {
                Ok(k) => k,
                Err(_) => break,
            };
            let len = match decb::u8_as_usize(input) {
                Ok(l) => l,
                Err(_) => break,
            };

            // Skip: consume the payload bytes and continue the loop
            if key == 0xAA {
                let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                continue;
            }

            match key {
                0x01 => {
                    color = Color::decode_value(input).ok().or(color);
                }
                0x02 => {
                    priority = Priority::decode_value(input).ok().or(priority);
                }
                _ => {
                    let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                }
            }
        }

        Ok(BreakOnSkip { color, priority })
    }
}

#[test]
/// Tests that a `Done` break condition (key `0xFF`) halts the decode loop and leaves subsequent keys unread.
fn break_done_stops_decode() {
    // Color(0x01), Done terminator(0xFF, len=0), Priority(0x02) - last must not be decoded
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x01, // Color::Red
        0xFF, 0x00, // Done terminator
        0x02, 0x01, 0x02, // Priority::High - unreachable
    ];
    let result = BreakOnDone::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.color, Some(Color::Red));
    assert_eq!(
        result.priority, None,
        "priority after Done terminator must not be decoded"
    );
}

#[test]
/// Tests that a `Done` terminator at the very start of input yields a fully-`None` struct.
fn break_done_empty_after_terminator() {
    let data: &[u8] = &[0xFF, 0x00];
    let result = BreakOnDone::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
}

#[test]
/// Tests that an `Abort` break condition (key `0xFE`) returns `Err` immediately, discarding any partial state.
fn break_abort_returns_error() {
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x02, // Color::Green
        0xFE, 0x00, // Abort trigger
        0x02, 0x01, 0x01, // Priority::Medium - unreachable
    ];
    let result = BreakOnAbort::decode_value(&mut &data[..]);
    assert!(result.is_err(), "Abort condition must return Err");
}

#[test]
/// Tests that an `Abort` trigger encountered before any useful data produces an error.
fn break_abort_at_start_returns_error() {
    let data: &[u8] = &[0xFE, 0x00];
    assert!(BreakOnAbort::decode_value(&mut &data[..]).is_err());
}

#[test]
/// Tests that a `Skip` break condition consumes a deprecated key's payload and continues decoding subsequent fields.
fn break_skip_deprecated() {
    // Color(0x01), deprecated(0xAA, len=3, 3 junk bytes), Priority(0x02)
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x03, // Color::Blue
        0xAA, 0x03, 0xDE, 0xAD, 0xBE, // deprecated key - payload consumed, not decoded
        0x02, 0x01, 0x03, // Priority::Critical
    ];
    let result = BreakOnSkip::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.color, Some(Color::Blue));
    assert_eq!(result.priority, Some(Priority::Critical));
}

#[test]
/// Tests skipping multiple deprecated keys, where a deprecated key is defined by the break-condition closure.
fn break_skip_multiple_deprecated() {
    // Two deprecated keys surrounding real fields
    let data: &[u8] = &[
        0xAA, 0x02, 0xFF, 0xFF, // first deprecated - skipped
        0x01, 0x02, 0x00, 0x04, // Color::Alpha
        0xAA, 0x01, 0x00, // second deprecated - skipped
        0x02, 0x01, 0x00, // Priority::Low
    ];
    let result = BreakOnSkip::decode_value(&mut &data[..]).unwrap();
    assert_eq!(result.color, Some(Color::Alpha));
    assert_eq!(result.priority, Some(Priority::Low));
}
