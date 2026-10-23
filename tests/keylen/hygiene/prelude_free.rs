//! A fused `keylen` derive with nothing but the derive imported from `tinyklv`, beside a trait
//! whose methods are named like the ones the generated code calls
//!
//! The generated code calls the methods of winnow's `Stream` and `Parser` on the input and on
//! each decoder. No `tinyklv::prelude` trait is in scope here, so a method call that needs one
//! does not compile, and [`Lookalike`], implemented for every type, is, so such a method call
//! reaches it. [`Sensor`] has `Inline` and `Sliced` items, a length-checked and a `lengthed`
//! field, the items no field declares and a `break_on` fn: it must decode and encode as if
//! [`Lookalike`] were not there
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::unknown::{self, Unknown};
use crate::common::varint;
use crate::support::header;
use tinyklv::Klv;
use tinyklv::dec::binary as decb;
use tinyklv::dec::string as decs;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;

/// Implemented for every type, with methods named like the `Stream` and `Parser` methods the
/// derive calls, each of another arity than theirs
#[expect(dead_code, reason = "the derive below must not call it")]
trait Lookalike {
    /// Named like `Stream::checkpoint`
    fn checkpoint(&self, _: u8) {}

    /// Named like `Stream::reset`
    fn reset(&self) {}

    /// Named like `Stream::eof_offset`
    fn eof_offset(&self, _: u8) {}

    /// Named like `Parser::parse_next`
    fn parse_next(&self) {}
}

/// [`Lookalike`] implementation for every type
impl<T: ?Sized> Lookalike for T {}

/// A proto3 sensor reading in keylen mode, keeping its unknown frames, ended by field 15
#[derive(Klv, Debug, PartialEq)]
#[klv(
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    unknown(dec = unknown::unknown_body_dec, enc = unknown::unknown_body_enc),
    break_on = stop,
)]
struct Sensor {
    /// field 1, wire 0 VARINT - `Inline`
    #[klv(key = 0x08, dec = varint::dec_i32, enc = *varint::enc_i32)]
    id: i32,

    /// field 2, wire 5 I32 - `Sliced(4)`, its length checked
    #[klv(key = 0x15, dec = decb::le_f32, enc = *encb::le_f32, size(exact = 4))]
    ratio: f32,

    /// field 3, wire 2 LEN - `Sliced(n)`, its length passed to the decoder
    #[klv(
        key = 0x1A,
        dec = decs::to_string_utf8,
        enc = &encs::from_string_utf8,
        size(lengthed),
    )]
    name: String,

    /// every other frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// [`Sensor`] implementation
impl Sensor {
    /// `id = 5`, `ratio = 1.5`, `name = "hi"`, then unknown field 4 = 7
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // id = 5
        0x08, 0x05,
        // ratio = 1.5
        0x15, 0x00, 0x00, 0xC0, 0x3F,
        // name = "hi"
        0x1A, 0x02, 0x68, 0x69,
        // field 4, wire 0 VARINT = 7
        0x20, 0x07,
    ];
}

/// Ends the loop on field 15, wire 0 VARINT, else decodes the frame
fn stop(tag: &u32, _input: &mut &[u8]) -> tinyklv::BreakType {
    if *tag == 0x78 {
        tinyklv::BreakType::Done
    } else {
        tinyklv::BreakType::Proceed
    }
}

/// [`Sensor`] decodes in both drivers, its unknown frame kept, and encodes back
#[test]
fn a_keylen_struct_needs_no_prelude() {
    let sensor = Sensor {
        id: 5,
        ratio: 1.5,
        name: "hi".to_owned(),
        extra: vec![(0x20, Unknown::Varint(7))],
    };
    sensor.assert_decodes_from(Sensor::WIRE);
    assert_eq!(sensor.encoded(), Sensor::WIRE);
}
