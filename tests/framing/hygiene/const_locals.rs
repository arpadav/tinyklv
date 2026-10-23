//! A key + len derive with a user `const key`, `const len`, `const val`, `const checkpoint` and
//! `const position` in scope
//!
//! The derive's generated code reads each frame's key and length, each field's decode result, the
//! input position at the fn's entry and the sentinel's position into locals, so a binding pattern
//! of one of these names would turn into the user's constant: a `let .. else` then skips every
//! frame whose key or length differs from it, and any other binding stops compiling. [`Pair`]
//! must decode, from its body and from its sentinel frame, and encode as if the constants were not
//! there. The body sends `b` first, its key not `key`, so a key binding turned into the constant
//! stops at the first frame and decodes nothing
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// Named like the derive's key local, of the key's type
#[expect(non_upper_case_globals, reason = "named like the derive's key local")]
#[expect(dead_code, reason = "the derive must not read it")]
const key: u8 = 0x01;

/// Named like the derive's length local, of its type
#[expect(non_upper_case_globals, reason = "named like the derive's len local")]
#[expect(dead_code, reason = "the derive must not read it")]
const len: usize = 3;

/// Named like the derive's decode-result local
#[expect(non_upper_case_globals, reason = "named like the derive's val local")]
#[expect(dead_code, reason = "the derive must not read it")]
const val: u8 = 9;

/// Named like the derive's fn-entry checkpoint local
#[expect(
    non_upper_case_globals,
    reason = "named like the derive's checkpoint local"
)]
#[expect(dead_code, reason = "the derive must not read it")]
const checkpoint: u8 = 0;

/// Named like the derive's sentinel position local, of its type
#[expect(
    non_upper_case_globals,
    reason = "named like the derive's position local"
)]
#[expect(dead_code, reason = "the derive must not read it")]
const position: usize = 3;

/// A pair of one-byte readings behind a sentinel, decoded with every constant above in scope
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel = b"\xAA",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    default(typ = u8, dec = decb::u8, enc = *encb::u8),
)]
struct Pair {
    /// key `0x01`
    #[klv(key = 0x01)]
    a: Option<u8>,

    /// key `0x02`
    #[klv(key = 0x02)]
    b: Option<u8>,
}

/// [`Pair`] implementation
impl Pair {
    /// `b = 6`, then `a = 5`
    #[rustfmt::skip]
    const BODY: &[u8] = &[
        // b = 6
        0x02, 0x01, 0x06,
        // a = 5
        0x01, 0x01, 0x05,
    ];
}

/// Both frames decode in both drivers and from the sentinel frame, and the value encodes back
/// in field order
#[test]
fn const_locals_in_scope_decode_every_frame() {
    let pair = Pair {
        a: Some(5),
        b: Some(6),
    };
    let frame = [&[0xAA, 0x06], Pair::BODY].concat();
    pair.assert_decodes_from(Pair::BODY);
    assert_eq!(pair.encoded(), [0x01, 0x01, 0x05, 0x02, 0x01, 0x06]);
    assert_eq!(Pair::decode_frame(&mut &frame[..]), Ok(pair));
}
