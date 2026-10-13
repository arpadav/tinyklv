//! A user `macro_rules! matches` in scope above a `deny_unknown_keys` struct
//!
//! The derive tests each frame's key against the declared keys with `matches!`. A user macro of
//! the same name in scope must not stand in for that test: the one here matches everything, so a
//! derive that expanded it lets an unknown key through, and leaves its `unused_macros` expectation
//! unmet
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;

/// Named like `core`'s, and matches everything
#[expect(unused_macros, reason = "the derive below must not expand it")]
macro_rules! matches {
    ($($args:tt)*) => {
        true
    };
}

/// A one-byte gauge level that rejects every key but its own
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    deny_unknown_keys,
    allow_unimplemented_encode,
)]
struct Gauge {
    /// key `0x08`, one byte
    #[klv(key = 0x08, dec = decb::u8)]
    level: u8,
}

/// The derive's own key test runs: an unknown key before the known one is rejected by both
/// drivers
#[test]
fn user_matches_does_not_hijack_the_unknown_key_gate() {
    #[rustfmt::skip]
    let wire = [
        // unknown key 0x30, length 1, `00`
        0x30, 0x01, 0x00,
        // level = 5
        0x08, 0x01, 0x05,
    ];
    assert!(Gauge::decode_value(&mut &wire[..]).is_err());
    assert!(Gauge::decode_partial(&mut &wire[..]).is_err());
}
