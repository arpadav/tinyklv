//! A fused `#[klv(keylen(..))]` struct behind a `sentinel(..)`
//!
//! The `keylen` codec writes no length of its own for a packet, so the `len(..)` in
//! `sentinel(..)` writes and reads each packet's length
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::varint;
use crate::support::example::Status;
use crate::support::header;
use tinyklv::prelude::*;

/// The proto3 message `Heartbeat { int32 seq = 1; Status status = 2; }` behind `HB`, each packet's
/// length a varint
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel(value = b"HB", len(dec = varint::dec_usize, enc = varint::enc_usize)),
    keylen(dec = header::dec_tag, enc = header::enc_tag),
)]
struct Heartbeat {
    /// field 1, wire 0 VARINT
    #[klv(key = 0x08, dec = varint::dec_i32, enc = *varint::enc_i32)]
    seq: i32,

    /// field 2, wire 0 VARINT, a custom enum
    #[klv(key = 0x10, dec = Status::decode_value, enc = Status::encode_value)]
    status: Status,
}

/// [`Heartbeat`] implementation
impl Heartbeat {
    /// The heartbeat [`Self::PACKET`] encodes
    const SAMPLE: Self = Self {
        seq: 7,
        status: Status::Active,
    };

    /// The packet of [`Self::SAMPLE`]
    #[rustfmt::skip]
    const PACKET: [u8; 7] = [
        // sentinel
        b'H', b'B',
        // packet length, a varint
        0x04,
        // seq = 7: tag 0x08, the varint
        0x08, 0x07,
        // status = Active: tag 0x10, the varint
        0x10, 0x01,
    ];
}

/// The sentinel's `len(..)` writes and reads the packet length, which the `keylen` codec has none
/// of
#[test]
fn a_keylen_struct_round_trips_behind_its_sentinel_len() {
    let mut wire = Vec::new();
    Heartbeat::SAMPLE.encode_frame(&mut wire);
    assert_eq!(wire, Heartbeat::PACKET);
    let mut input = wire.as_slice();
    let heartbeat = Heartbeat::decode_frame(&mut input).expect("the packet decodes");
    assert_eq!(heartbeat, Heartbeat::SAMPLE);
    assert!(input.is_empty(), "{input:02X?}");
}
