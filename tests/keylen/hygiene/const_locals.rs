//! Fused `keylen` derives with a user `const key`, `const len` and `const val` in scope
//!
//! The derive's generated code reads each frame's key, each `Sliced` frame's length, and each
//! field's decode result into locals, so a binding pattern named `key`, `len` or `val` would
//! turn into the user's constant and stop compiling. [`Logged`] is a `debug` struct, so the
//! logged decode result is bound too. Both must decode and encode as if the constants were not
//! there. Each constant differs from every key, length and value on the wire, so a binding turned
//! into a constant pattern never matches by chance
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::support::{header, value};
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Named like the derive's key local, of the key's type
#[expect(non_upper_case_globals, reason = "named like the derive's key local")]
#[expect(dead_code, reason = "the derive must not read it")]
const key: u32 = 0x10;

/// Named like the derive's length local, of its type
#[expect(non_upper_case_globals, reason = "named like the derive's len local")]
#[expect(dead_code, reason = "the derive must not read it")]
const len: usize = 5;

/// Named like the derive's decode-result local
#[expect(non_upper_case_globals, reason = "named like the derive's val local")]
#[expect(dead_code, reason = "the derive must not read it")]
const val: i32 = 9;

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
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32),
    default(typ = String, dec = value::dec_string, enc = &encs::from_string_utf8),
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
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    debug,
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32),
    default(typ = String, dec = value::dec_string, enc = &encs::from_string_utf8),
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
    plain.assert_decodes_from(&WIRE);
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
    logged.assert_decodes_from(&WIRE);
    assert_eq!(logged.encoded(), WIRE);
}
