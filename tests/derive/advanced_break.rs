//! Custom `BreakCondition` trait tests
//!
//! Tests `Done`, `Abort`, and `Skip` break conditions via manual `Decode`
//! implementations. The blanket `impl<T: Decode<S>> BreakCondition<S> for T`
//! provides the default `Proceed` return, but the default method signature
//! `fn break_condition<K, L>(key: K, len: L)` carries no trait bounds on K
//! or L, making it impossible to inspect the key inside an override without
//! a concrete type. The idiomatic pattern is therefore to embed the break
//! logic directly in the manual `Decode` loop using the concrete `u8` key
//! value, which is exactly what the derive macro expansion does.
//!
//! Author: aav

// --------------------------------------------------
// local
// --------------------------------------------------
use super::types::*;
use tinyklv::prelude::*;

// --------------------------------------------------
// BreakOnDone - stops loop when key 0xFF is seen
// --------------------------------------------------

#[derive(Debug, PartialEq, Default)]
struct BreakOnDone {
    color: Option<Color>,
    priority: Option<Priority>,
}

impl tinyklv::Decode<&[u8]> for BreakOnDone {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut color: Option<Color> = None;
        let mut priority: Option<Priority> = None;

        loop {
            let key = match tinyklv::dec::binary::be_u8(input) {
                Ok(k) => k,
                Err(_) => break,
            };
            let len = match tinyklv::dec::binary::be_u8_as_usize(input) {
                Ok(l) => l,
                Err(_) => break,
            };

            // Done: stop the loop, attempt to return what we have
            if key == 0xFF {
                break;
            }

            match key {
                0x01 => {
                    color = decode_color(input).ok().or(color);
                }
                0x02 => {
                    priority = decode_priority(input).ok().or(priority);
                }
                _ => {
                    let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                }
            }
        }

        Ok(BreakOnDone { color, priority })
    }
}

// --------------------------------------------------
// BreakOnAbort - returns Err when key 0xFE is seen
// --------------------------------------------------

#[derive(Debug, PartialEq, Default)]
struct BreakOnAbort {
    color: Option<Color>,
    priority: Option<Priority>,
}

impl tinyklv::Decode<&[u8]> for BreakOnAbort {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut color: Option<Color> = None;
        let mut priority: Option<Priority> = None;

        loop {
            let key = match tinyklv::dec::binary::be_u8(input) {
                Ok(k) => k,
                Err(_) => break,
            };
            let len = match tinyklv::dec::binary::be_u8_as_usize(input) {
                Ok(l) => l,
                Err(_) => break,
            };

            // Abort: return an error immediately, no partial result
            if key == 0xFE {
                return Err(winnow::error::ParserError::from_input(input));
            }

            match key {
                0x01 => {
                    color = decode_color(input).ok().or(color);
                }
                0x02 => {
                    priority = decode_priority(input).ok().or(priority);
                }
                _ => {
                    let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                }
            }
        }

        Ok(BreakOnAbort { color, priority })
    }
}

// --------------------------------------------------
// BreakOnSkip - skips key 0xAA payload without decoding
// --------------------------------------------------

#[derive(Debug, PartialEq, Default)]
struct BreakOnSkip {
    color: Option<Color>,
    priority: Option<Priority>,
}

impl tinyklv::Decode<&[u8]> for BreakOnSkip {
    fn decode(input: &mut &[u8]) -> tinyklv::Result<Self> {
        let mut color: Option<Color> = None;
        let mut priority: Option<Priority> = None;

        loop {
            let key = match tinyklv::dec::binary::be_u8(input) {
                Ok(k) => k,
                Err(_) => break,
            };
            let len = match tinyklv::dec::binary::be_u8_as_usize(input) {
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
                    color = decode_color(input).ok().or(color);
                }
                0x02 => {
                    priority = decode_priority(input).ok().or(priority);
                }
                _ => {
                    let _: tinyklv::Result<_> = winnow::token::take(len).parse_next(input);
                }
            }
        }

        Ok(BreakOnSkip { color, priority })
    }
}

// --------------------------------------------------
// tests
// --------------------------------------------------

#[test]
fn break_done_stops_decode() {
    // Color(0x01), Done terminator(0xFF, len=0), Priority(0x02) - last must not be decoded
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x01, // Color::Red
        0xFF, 0x00, // Done terminator
        0x02, 0x01, 0x02, // Priority::High - unreachable
    ];
    let result = BreakOnDone::decode(&mut &data[..]).unwrap();
    assert_eq!(result.color, Some(Color::Red));
    assert_eq!(
        result.priority, None,
        "priority after Done terminator must not be decoded"
    );
}

#[test]
fn break_done_empty_after_terminator() {
    // Terminator at the very start - nothing decoded
    let data: &[u8] = &[0xFF, 0x00];
    let result = BreakOnDone::decode(&mut &data[..]).unwrap();
    assert_eq!(result.color, None);
    assert_eq!(result.priority, None);
}

#[test]
fn break_abort_returns_error() {
    // Color(0x01), abort trigger(0xFE, len=0), Priority(0x02) - must Err
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x02, // Color::Green
        0xFE, 0x00, // Abort trigger
        0x02, 0x01, 0x01, // Priority::Medium - unreachable
    ];
    let result = BreakOnAbort::decode(&mut &data[..]);
    assert!(result.is_err(), "Abort condition must return Err");
}

#[test]
fn break_abort_at_start_returns_error() {
    // Abort trigger before any useful data
    let data: &[u8] = &[0xFE, 0x00];
    assert!(BreakOnAbort::decode(&mut &data[..]).is_err());
}

#[test]
fn break_skip_deprecated() {
    // Color(0x01), deprecated(0xAA, len=3, 3 junk bytes), Priority(0x02)
    let data: &[u8] = &[
        0x01, 0x02, 0x00, 0x03, // Color::Blue
        0xAA, 0x03, 0xDE, 0xAD, 0xBE, // deprecated key - payload consumed, not decoded
        0x02, 0x01, 0x03, // Priority::Critical
    ];
    let result = BreakOnSkip::decode(&mut &data[..]).unwrap();
    assert_eq!(result.color, Some(Color::Blue));
    assert_eq!(result.priority, Some(Priority::Critical));
}

#[test]
fn break_skip_multiple_deprecated() {
    // Two deprecated keys surrounding real fields
    let data: &[u8] = &[
        0xAA, 0x02, 0xFF, 0xFF, // first deprecated - skipped
        0x01, 0x02, 0x00, 0x04, // Color::Alpha
        0xAA, 0x01, 0x00, // second deprecated - skipped
        0x02, 0x01, 0x00, // Priority::Low
    ];
    let result = BreakOnSkip::decode(&mut &data[..]).unwrap();
    assert_eq!(result.color, Some(Color::Alpha));
    assert_eq!(result.priority, Some(Priority::Low));
}
