//! `key = oneof(K => Enum::Variant(Type), ..)` on an `Option<Enum>` field of a fused
//! `#[klv(keylen(..))]` struct
//!
//! The header frames each arm's value by the wire type in its tag, and the arm type's
//! `default(typ = ..)` codes it, so a oneof of an `int32` and a `string` reads and writes the
//! bytes protobuf does: the `int32` arm with no length after its tag, the `string` arm with a
//! varint length, and an unset oneof with no frame at all
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

/// A reading's payload: a count or a note
#[derive(Debug, PartialEq)]
enum Payload {
    /// field 7, wire 0 VARINT - `Inline`
    Count(i32),

    /// field 8, wire 2 LEN - `Sliced(n)`
    Note(String),
}

/// A proto3 message in keylen mode with a oneof: `int32 id = 1` and `oneof payload { int32
/// count = 7; string note = 8; }`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32),
    default(typ = String, dec = value::dec_string, enc = &encs::from_string_utf8),
)]
struct Reading {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08)]
    id: i32,

    /// fields 7 and 8: a count, wire 0 VARINT, or a note, wire 2 LEN
    #[klv(key = oneof(0x38 => Payload::Count(i32), 0x42 => Payload::Note(String)))]
    payload: Option<Payload>,
}

/// The oneof decodes from, and encodes byte for byte to, the bytes protobuf writes for each of its
/// states, in both drivers and at every chunk split: the count arm, the note arm, and unset
#[test]
#[rustfmt::skip]
fn a_oneof_matches_protobufs_bytes() {
    let counted = [
        // id = 5 (field 1, wire 0 VARINT, Inline)
        0x08, 0x05,
        // count = 300 (field 7, wire 0 VARINT, Inline)
        0x38, 0xAC, 0x02,
    ];
    let noted = [
        // id = 5 (field 1, wire 0 VARINT, Inline)
        0x08, 0x05,
        // note = "hi" (field 8, wire 2 LEN, Sliced(2))
        0x42, 0x02, b'h', b'i',
    ];
    let unset = [
        // id = 5 (field 1, wire 0 VARINT, Inline)
        0x08, 0x05,
    ];
    let cases: [(&[u8], _); 3] = [
        (&counted, Some(Payload::Count(300))),
        (&noted, Some(Payload::Note("hi".to_owned()))),
        (&unset, None),
    ];
    for (wire, payload) in cases {
        let reading = Reading { id: 5, payload };
        assert_eq!(reading.encoded(), wire);
        reading.assert_decodes_from(wire);
    }
}
