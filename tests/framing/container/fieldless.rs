//! A key + len struct with no field
//!
//! It decodes by skipping every frame by its length, unless it denies unknown keys, which then
//! rejects every frame, and encodes to nothing
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use tinyklv::dec::binary as decb;
use tinyklv::prelude::*;

/// A heartbeat, whose frames carry nothing it keeps
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8, enc = tinyklv::enc::binary::u8),
    len(dec = decb::u8_as_usize, enc = tinyklv::enc::binary::u8_from_usize),
)]
struct Heartbeat {}

/// Both drivers skip every frame to the end of the input, and the encode writes nothing
#[test]
fn a_fieldless_struct_skips_every_frame_and_encodes_nothing() {
    #[rustfmt::skip]
    let wire = [
        // key 0x08, length 1
        0x08, 0x01,
        // its value
        0x05,
        // key 0x12, length 1
        0x12, 0x01,
        // its value
        0x61,
    ];
    Heartbeat {}.assert_decodes_from(&wire);
    assert_eq!(Heartbeat {}.encoded(), []);
}

/// A heartbeat that denies every key it does not declare, which is every key
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = decb::u8),
    len(dec = decb::u8_as_usize),
    deny_unknown_keys,
    allow_unimplemented_encode,
)]
struct Silence {}

/// Both drivers reject the first frame of a fieldless struct that denies unknown keys
#[test]
fn a_fieldless_struct_that_denies_unknown_keys_rejects_every_frame() {
    // key 0x08, length 1, its value
    let wire = [0x08, 0x01, 0x05];
    Silence::assert_rejects_naming(
        &wire,
        &["key (expected one of the keys defined on `Silence`"],
    );
}
