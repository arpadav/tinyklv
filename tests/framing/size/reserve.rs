//! `size(reserve = N)` on a field of a key + len struct: `N` more bytes in the encoder's one
//! up-front allocation
//!
//! The reserve sizes the buffer `encode_value` appends to before the first frame is written, and
//! changes no byte on the wire
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::varint;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

/// A station whose name reserves room for a long one, though most are short
#[derive(Klv, Debug)]
#[klv(
    key(enc = varint::enc_u32),
    len(enc = varint::enc_usize),
    allow_unimplemented_decode,
)]
struct Station {
    /// field 3, wire type 2
    #[klv(key = 0x1A, enc = &encs::from_string_utf8, size(reserve = 64))]
    name: String,
}

/// Encode reserves the field's bytes before it writes: an empty buffer holds at least the
/// `reserve = 64` after a frame far shorter than that, and the frame is the one written without it
#[test]
fn encode_reserves_the_fields_bytes_up_front() {
    let station = Station {
        name: "hi".to_owned(),
    };
    let mut out = Vec::new();
    station.encode_value(&mut out);
    // name = "hi": tag, length 2, "hi"
    assert_eq!(out, [0x1A, 0x02, b'h', b'i']);
    assert!(out.capacity() >= 64, "capacity {}", out.capacity());
}
