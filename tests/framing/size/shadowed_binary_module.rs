//! A user module named `binary`, like the crate's binary codecs, under a fixed-width length slot
//!
//! A value whose encoder is one of the crate's fixed-width ones has its length written before it.
//! A user module's encoder of the same name must not stand in for the crate's: its `u32` here
//! writes a varint, so its value's length is back-filled from the bytes it wrote, and the frame
//! decodes back
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

/// Named like the crate's binary codecs, its `u32` a varint
mod binary {
    pub(super) use crate::common::varint::{dec_u32, enc_u32 as u32};
}

/// A count under a one-byte length slot, written by the user's `binary::u32`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize, size(exact = 1)),
)]
struct Tally {
    /// key 0x01, a varint
    #[klv(key = 0x01, dec = binary::dec_u32, enc = *binary::u32)]
    count: u32,
}

/// The count's varint is two bytes, not the four a native-endian `u32` takes: its length is the
/// two bytes written, and the frame decodes back to the count
#[test]
fn a_user_binary_module_takes_the_measured_length() {
    #[rustfmt::skip]
    let wire = [
        // count = 300, a two-byte varint
        0x01, 0x02, 0xAC, 0x02,
    ];
    let tally = Tally { count: 300 };
    assert_eq!(tally.encoded(), wire);
    tally.assert_decodes_from(&wire);
}
