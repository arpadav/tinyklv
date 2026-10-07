//! A user `macro_rules! debug_assert_eq` in scope above a known-width struct
//!
//! The derive checks, in debug builds, the bytes its length and value encoders wrote. A user
//! macro of the same name in scope must not stand in for that check: the one here panics, so a
//! derive that expanded it fails the encode, and leaves its `unused_macros` expectation unmet
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

/// Named like `core`'s, and panics whenever it expands
#[expect(unused_macros, reason = "the derive below must not expand it")]
macro_rules! debug_assert_eq {
    ($($args:tt)*) => {
        panic!("the derive expanded the user's `debug_assert_eq!`")
    };
}

/// A known-width `u16` field and a back-filled string field under a one-byte length slot
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Telemetry {
    /// two bytes, a width known when the derive runs
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16)]
    altitude: u16,

    /// a width known only once encoded, back-filled into the length slot
    #[klv(
        key = 0x02,
        size(lengthed),
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8
    )]
    callsign: String,
}

/// Both length checks run the derive's own assertion: the encode is the wire, which decodes
/// back to the value with the cursor at the end
#[test]
fn user_debug_assert_eq_does_not_hijack_length_checks() {
    #[rustfmt::skip]
    let wire = [
        // altitude = 300
        0x01, 0x02, 0x01, 0x2C,
        // callsign = "KLV"
        0x02, 0x03, 0x4B, 0x4C, 0x56,
    ];
    let telemetry = Telemetry {
        altitude: 300,
        callsign: "KLV".to_owned(),
    };
    let mut encoded = Vec::new();
    telemetry.encode_value(&mut encoded);
    assert_eq!(encoded, wire);
    let mut cursor = &wire[..];
    assert_eq!(Telemetry::decode_value(&mut cursor), Ok(telemetry));
    assert!(cursor.is_empty(), "decode stops at {cursor:02X?}");
}
