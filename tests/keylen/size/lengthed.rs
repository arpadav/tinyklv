//! `size(lengthed)` on a field of a fused `#[klv(keylen(..))]` struct
//!
//! A `Sliced` frame hands the field's decoder the frame's length, as key + len mode does, so the
//! crate's length-taking codecs read proto `string` fields. An `Inline` frame carries no length to
//! hand it: its value is malformed wherever the cursor is, rejected in both drivers with an error
//! naming the field, the decoder never run and the cursor left at the value start
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::common::wire;
use crate::support::example::Status;
use crate::support::header;
use tinyklv::dec::string as decs;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

/// proto3 contact keyed by field number, whose `string` fields read through the crate's
/// length-taking UTF-8 decoder
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_field, enc = Contact::enc_header),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32),
    default(
        typ = String,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    ),
)]
struct Contact {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 1)]
    id: i32,

    /// field 2, wire 2 LEN - `Sliced(n)`, `size(lengthed)` from the `String` default
    #[klv(key = 2)]
    name: String,

    /// field 3, wire 2 LEN - optional, `size(lengthed)` from the `String` default
    #[klv(key = 3)]
    email: Option<String>,

    /// field 4, wire 0 VARINT - `Inline` custom enum
    #[klv(key = 4, dec = Status::decode_value, enc = Status::encode_value)]
    status: Option<Status>,
}

/// [`Contact`] implementation
impl Contact {
    /// id = 7 (field 1, wire 0 VARINT, Inline)
    const ID: &[u8] = &[0x08, 0x07];

    /// name = "Ada" (field 2, wire 2 LEN, Sliced(3))
    const NAME: &[u8] = &[0x12, 0x03, b'A', b'd', b'a'];

    /// email = "ada@x.io" (field 3, wire 2 LEN, Sliced(8))
    #[rustfmt::skip]
    const EMAIL: &[u8] = &[
        0x1A, 0x08,
        b'a', b'd', b'a', b'@', b'x', b'.', b'i', b'o',
    ];

    /// status = Active (field 4, wire 0 VARINT, Inline)
    const STATUS: &[u8] = &[0x20, 0x01];

    /// The full wire encoding of [`Contact::decoded`]
    fn wire() -> Vec<u8> {
        [Self::ID, Self::NAME, Self::EMAIL, Self::STATUS].concat()
    }

    /// The canonical decoded [`Contact`] matching [`Contact::wire`]
    fn decoded() -> Self {
        Self {
            id: 7,
            name: "Ada".to_owned(),
            email: Some("ada@x.io".to_owned()),
            status: Some(Status::Active),
        }
    }

    /// Encodes a frame header keyed by field number, each field with the wire type it is sent as
    ///
    /// # Arguments
    ///
    /// * `(field, value_len)` - the field number and the byte length of the value that follows
    /// * `out` - the buffer the header bytes are appended to
    fn enc_header((field, value_len): (u32, usize), out: &mut Vec<u8>) {
        let wire_type = match field {
            // name and email
            2 | 3 => wire::LEN,
            // id and status
            _ => wire::VARINT,
        };
        header::enc_field(wire_type, (field, value_len), out);
    }
}

/// Encode writes each `string` as a LEN frame, and its `Sliced` frames hand the crate's
/// `to_string_utf8` their length, in both drivers and at every chunk split
#[test]
fn encode_round_trips() {
    let encoded = Contact::decoded().encoded();
    assert_eq!(encoded, Contact::wire());
    Contact::decoded().assert_decodes_from(&encoded);
}

/// An `Inline` frame for a `lengthed` field at the end of the input is still malformed, never
/// truncation: streaming errors rather than waiting for a value
#[test]
fn an_inline_frame_at_the_end_of_the_input_is_malformed() {
    #[rustfmt::skip]
    let wire = [
        Contact::ID,
        // name (field 2, wire 0 VARINT, Inline), and nothing after it
        &[0x10],
    ]
    .concat();
    Contact::assert_rejects_naming(&wire, &["`Contact::name`", "size(lengthed)"]);
}
