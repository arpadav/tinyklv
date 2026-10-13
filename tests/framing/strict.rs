//! `strict` on a key + len struct: a value with a length prefix that its decoder rejects is
//! malformed
//!
//! Both drivers reject such a value at every chunk split, with an error naming the field. The same
//! message without `strict` drops the value, so the field keeps what it had
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use tinyklv::dec::string as decs;
use tinyklv::prelude::*;

/// A proto3 message in key + len mode under `strict`: `int32 id = 1` (a varint) and `string name
/// = 3` (length-delimited)
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
    strict,
    default(typ = i32, dec = varint::dec_i32, size(implicit)),
)]
struct StrictTicket {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// field 3, wire type 2, the empty string when absent
    #[klv(
        key = 0x1A,
        dec = decs::to_string_utf8_strict,
        size(lengthed),
        skip_if_default
    )]
    name: String,
}

/// [`StrictTicket`] without `strict`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32),
    len(dec = varint::dec_usize),
    allow_unimplemented_encode,
    default(typ = i32, dec = varint::dec_i32, size(implicit)),
)]
struct Ticket {
    /// field 1, wire type 0
    #[klv(key = 0x08)]
    id: i32,

    /// field 3, wire type 2, the empty string when absent
    #[klv(
        key = 0x1A,
        dec = decs::to_string_utf8_strict,
        size(lengthed),
        skip_if_default
    )]
    name: String,
}

/// A value that is not UTF-8 is malformed under `strict`; without it the value is dropped and the
/// field finalizes to the empty string
#[test]
fn a_value_its_decoder_rejects_is_malformed() {
    let message = "`StrictTicket::name` value (its decoder rejected the frame's value)";
    #[rustfmt::skip]
    let wire = [
        // id = 5
        0x08, 0x05,
        // name, length 2: bytes that are not UTF-8
        0x1A, 0x02, 0xFF, 0xFE,
    ];
    StrictTicket::assert_rejects_naming(&wire, &[message]);
    let dropped = Ticket {
        id: 5,
        name: String::new(),
    };
    dropped.assert_decodes_from(&wire);
}
