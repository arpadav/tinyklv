//! A sentinel struct with an inherent `encode_value` of its own
//!
//! The derive's `encode_frame` writes the frame's body with the struct's derived `EncodeValue`.
//! An inherent method of the same name must not stand in for it: the one here writes `INHERENT`
//! and the mode, so a derive that called it frames those bytes, and leaves its `dead_code`
//! expectation unmet.
//! Each struct reaches one of the two frame writers: a known-width length slot back-filled after
//! the body, or a length written once the body is staged
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// A one-byte mode under a one-byte frame length slot
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Slotted {
    /// key `0x01`, one byte
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    mode: u8,
}

/// [`Slotted`] implementation
impl Slotted {
    /// Named like [`EncodeValue::encode_value`], and writes `INHERENT` and the mode
    #[expect(dead_code, reason = "the derive above must not call it")]
    fn encode_value(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"INHERENT");
        out.push(self.mode);
    }
}

/// A one-byte mode under a frame length of no declared width
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    sentinel = b"\xAA",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct Staged {
    /// key `0x01`, one byte
    #[klv(key = 0x01, dec = decb::u8, enc = *encb::u8)]
    mode: u8,
}

/// [`Staged`] implementation
impl Staged {
    /// Named like [`EncodeValue::encode_value`], and writes `INHERENT` and the mode
    #[expect(dead_code, reason = "the derive above must not call it")]
    fn encode_value(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"INHERENT");
        out.push(self.mode);
    }
}

/// Both frame writers frame the derived body
#[test]
fn inherent_encode_value_does_not_hijack_the_frame_body() {
    #[rustfmt::skip]
    let wire = [
        // sentinel, frame length 3
        0xAA, 0x03,
        // mode = 7
        0x01, 0x01, 0x07,
    ];
    let mut slotted = Vec::new();
    Slotted { mode: 7 }.encode_frame(&mut slotted);
    assert_eq!(slotted, wire);
    let mut staged = Vec::new();
    Staged { mode: 7 }.encode_frame(&mut staged);
    assert_eq!(staged, wire);
}
