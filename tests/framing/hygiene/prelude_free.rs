//! Key + len derives with nothing but the derive imported from `tinyklv`, beside a trait whose
//! methods are named like the ones the generated code calls
//!
//! The generated code calls the methods of winnow's `Stream` and `Parser` on the input and on
//! each decoder. No `tinyklv::prelude` trait is in scope here, so a method call that needs one
//! does not compile, and [`Lookalike`], implemented for every type, is, so such a method call
//! reaches it. [`Gauge`] has a sentinel, a length-checked field and a `break_on` fn, [`Log`] the
//! frames no field declares, a `size(implicit)` field and `strict`: both must decode and encode as
//! if [`Lookalike`] were not there
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::unknown::{self, Unknown};
use crate::common::varint;
use tinyklv::Klv;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;

/// Implemented for every type, with methods named like the `Stream` and `Parser` methods the
/// derive calls, each of another arity than theirs
#[expect(dead_code, reason = "the derives below must not call it")]
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

/// A gauge reading behind a sentinel, ended by a `0xFF` key
#[derive(Klv, Debug, PartialEq)]
#[klv(
    sentinel = b"\xAA",
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
    break_on = stop,
)]
struct Gauge {
    /// key `0x01`, two bytes, its length checked
    #[klv(key = 0x01, dec = decb::be_u16, enc = *encb::be_u16, size(exact = 2))]
    level: u16,

    /// key `0x02`, one byte
    #[klv(key = 0x02, dec = decb::u8, enc = *encb::u8)]
    mode: Option<u8>,
}

/// [`Gauge`] implementation
impl Gauge {
    /// `level = 42`, `mode = 5`
    #[rustfmt::skip]
    const BODY: &[u8] = &[
        // level = 42
        0x01, 0x02, 0x00, 0x2A,
        // mode = 5
        0x02, 0x01, 0x05,
    ];
}

/// Ends the loop on key `0xFF`, else decodes the frame
fn stop(key: &u8, _input: &mut &[u8]) -> tinyklv::BreakType {
    if *key == 0xFF {
        tinyklv::BreakType::Done
    } else {
        tinyklv::BreakType::Proceed
    }
}

/// A log entry in protobuf's key + len form, keeping the frames no field declares
#[derive(Klv, Debug, PartialEq)]
#[klv(
    key(dec = varint::dec_u32, enc = varint::enc_u32),
    len(dec = varint::dec_usize, enc = varint::enc_usize),
    unknown(dec = unknown::unknown_dec, enc = unknown::unknown_enc),
    strict,
)]
struct Log {
    /// field 1, wire type 0, a varint with no length
    #[klv(key = 0x08, dec = varint::dec_i32, enc = *varint::enc_i32, size(implicit))]
    id: i32,

    /// field 2, wire type 2, two length-prefixed bytes
    #[klv(key = 0x12, dec = decb::be_u16, enc = *encb::be_u16)]
    count: u16,

    /// every other frame, keyed by its tag
    #[klv(unknown)]
    extra: Vec<(u32, Unknown)>,
}

/// [`Log`] implementation
impl Log {
    /// `id = 5`, `count = 258`, then unknown field 3 = 7
    #[rustfmt::skip]
    const WIRE: &[u8] = &[
        // id = 5
        0x08, 0x05,
        // count = 258
        0x12, 0x02, 0x01, 0x02,
        // field 3, wire type 0 = 7
        0x18, 0x07,
    ];
}

/// [`Gauge`] decodes in both drivers and from its sentinel frame, and encodes back to both
#[test]
fn a_sentinel_struct_needs_no_prelude() {
    let gauge = Gauge {
        level: 42,
        mode: Some(5),
    };
    let frame = [&[0xAA, 0x07], Gauge::BODY].concat();
    gauge.assert_decodes_from(Gauge::BODY);
    assert_eq!(gauge.encoded(), Gauge::BODY);
    assert_eq!(
        <Gauge as tinyklv::traits::DecodeFrame<&[u8]>>::decode_frame(&mut &frame[..]),
        Ok(gauge),
    );
}

/// [`Log`] decodes in both drivers, its unknown frame kept, and encodes back
#[test]
fn an_unknown_keeping_struct_needs_no_prelude() {
    let log = Log {
        id: 5,
        count: 258,
        extra: vec![(0x18, Unknown::Varint(7))],
    };
    log.assert_decodes_from(Log::WIRE);
    assert_eq!(log.encoded(), Log::WIRE);
}
