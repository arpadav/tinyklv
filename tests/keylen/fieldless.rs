//! A fused `#[klv(keylen(..))]` struct with no field
//!
//! It decodes by skipping every `Sliced` frame by its length, and encodes to nothing
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::support::header;
use tinyklv::prelude::*;

/// A heartbeat keyed by protobuf tag, whose frames carry nothing it keeps
#[derive(Klv, Debug, PartialEq)]
#[klv(keylen(dec = header::dec_tag, enc = header::enc_tag))]
struct Heartbeat {}

/// Both drivers skip every frame to the end of the input, and the encode writes nothing
#[test]
fn a_fieldless_struct_skips_every_frame_and_encodes_nothing() {
    #[rustfmt::skip]
    let wire = [
        // tag 0x12 (field 2, wire 2 LEN, Sliced(1))
        0x12, 0x01,
        // its value
        0x61,
        // tag 0x1A (field 3, wire 2 LEN, Sliced(2))
        0x1A, 0x02,
        // its value
        0x62, 0x63,
    ];
    Heartbeat {}.assert_decodes_from(&wire);
    assert_eq!(Heartbeat {}.encoded(), []);
}
