//! Fused `keylen` derives with a user `const key`, `const len` and `const val` in scope
//!
//! The derive's generated code reads each frame's key, each `Sliced` frame's length, and each
//! field's decode result into locals, so a binding pattern named `key`, `len` or `val` would
//! turn into the user's constant and stop compiling. [`Logged`] is a `debug` struct, so the
//! logged decode result is bound too. Both must decode and encode as if the constants were not
//! there
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::support::drive::Drive;
use crate::support::pb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Named like the derive's key local, of the key's type
#[expect(non_upper_case_globals, reason = "named like the derive's key local")]
#[expect(dead_code, reason = "the derive must not read it")]
const key: u32 = 0x08;

/// Named like the derive's length local, of its type
#[expect(non_upper_case_globals, reason = "named like the derive's len local")]
#[expect(dead_code, reason = "the derive must not read it")]
const len: usize = 2;

/// Named like the derive's decode-result local
#[expect(non_upper_case_globals, reason = "named like the derive's val local")]
#[expect(dead_code, reason = "the derive must not read it")]
const val: i32 = 3;

/// The frames both messages decode from and encode to
#[rustfmt::skip]
const WIRE: [u8; 6] = [
    // count = 3
    0x08, 0x03,
    // label = "hi"
    0x12, 0x02, 0x68, 0x69,
];

/// A message decoded and encoded with [`key`], [`len`] and [`val`] in scope
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen, enc = pb::enc_tag_keylen),
    default(typ = i32, dec = pb::dec_varint_i32, enc = *pb::enc_varint_i32),
    default(typ = String, dec = pb::dec_string, enc = &encs::from_string_utf8),
)]
struct Plain {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    count: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x12)]
    label: Option<String>,
}

/// [`Plain`] with `debug`, which logs each decode result
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen, enc = pb::enc_tag_keylen),
    debug,
    default(typ = i32, dec = pb::dec_varint_i32, enc = *pb::enc_varint_i32),
    default(typ = String, dec = pb::dec_string, enc = &encs::from_string_utf8),
)]
struct Logged {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    count: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x12)]
    label: Option<String>,
}

/// Both frames decode in both drivers to the end of input, and the value encodes back to them
#[test]
fn const_locals_in_scope_decode_every_frame() {
    let plain = Plain {
        count: 3,
        label: Some("hi".to_owned()),
    };
    let mut cursor = &WIRE[..];
    assert_eq!(Plain::decode_value(&mut cursor).as_ref(), Ok(&plain));
    assert!(cursor.is_empty(), "one-shot stops at {cursor:02X?}");
    let mut cursor = &WIRE[..];
    plain.assert_streams_to(&mut cursor);
    assert!(cursor.is_empty(), "streaming stops at {cursor:02X?}");
    assert_eq!(plain.encoded(), WIRE);
}

/// [`const_locals_in_scope_decode_every_frame`] with `debug`, which binds each logged decode
/// result too
#[test]
fn const_locals_in_scope_decode_every_frame_with_debug() {
    let logged = Logged {
        count: 3,
        label: Some("hi".to_owned()),
    };
    let mut cursor = &WIRE[..];
    assert_eq!(Logged::decode_value(&mut cursor).as_ref(), Ok(&logged));
    assert!(cursor.is_empty(), "one-shot stops at {cursor:02X?}");
    let mut cursor = &WIRE[..];
    logged.assert_streams_to(&mut cursor);
    assert!(cursor.is_empty(), "streaming stops at {cursor:02X?}");
    assert_eq!(logged.encoded(), WIRE);
}
