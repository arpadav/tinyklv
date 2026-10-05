//! Encode tests for the fused `#[klv(keylen(enc = ..))]` codec
//!
//! Every field is staged, framed by the fused encoder - the tag, plus a length only for
//! wire 2 LEN - and appended. These tests pin the exact bytes of that framing for every
//! wire shape and round-trip each message back through its decoder
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::example::{Example, Inner, Reading, Status};
use crate::pb;
use tinyklv::codecs::ber::BerLength;
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::enc::string as encs;
use tinyklv::prelude::*;

/// The proto3 message `Measurement { int32 id = 1; float ratio = 2; string name = 3; fixed64 created_at = 4; }`
///
/// `created_at` is optional so an unset timestamp is omitted from the wire
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen, enc = pb::enc_tag_keylen),
    default(typ = i32, dec = pb::dec_varint_i32, enc = *pb::enc_varint_i32),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32),
    default(typ = String, dec = pb::dec_string, enc = &encs::from_string_utf8),
    default(typ = u64, dec = decb::le_u64, enc = *encb::le_u64),
)]
struct Measurement {
    #[klv(key = 0x08)]
    /// field 1, wire 0 VARINT - tag, varint, no length
    id: i32,

    #[klv(key = 0x15)]
    /// field 2, wire 5 I32 - tag, 4 LE bytes, no length
    ratio: f32,

    #[klv(key = 0x1A)]
    /// field 3, wire 2 LEN - tag, varint length, bytes
    name: String,

    #[klv(key = 0x21)]
    /// field 4, wire 1 I64 - tag, 8 LE bytes, no length
    created_at: Option<u64>,
}

impl Measurement {
    /// `id = 5, ratio = 1.5, name = "hi"`, with `created_at` unset
    fn unstamped() -> Self {
        Self {
            id: 5,
            ratio: 1.5,
            name: "hi".to_owned(),
            created_at: None,
        }
    }
}

/// A generic message whose `value` has no `enc`/`dec`: it is coded by `trait_fallback`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen, enc = pb::enc_tag_keylen),
    trait_fallback,
    default(typ = i32, dec = pb::dec_varint_i32, enc = *pb::enc_varint_i32),
)]
struct Tagged<T>
where
    T: for<'a> DecodeValue<&'a [u8]> + EncodeValue,
{
    #[klv(key = 0x08)]
    /// field 1, wire 0 VARINT
    count: i32,

    #[klv(key = 0x10)]
    /// field 2, wire 0 VARINT - `Inline`, read and written through the `T` trait impls
    value: T,
}

/// Encodes a value into a fresh buffer
trait Encoded {
    /// # Returns
    ///
    /// The bytes `encode_value` appends to an empty buffer
    fn encoded(&self) -> Vec<u8>;
}

/// [`EncodeValue`] implementation of [`Encoded`]
impl<T: EncodeValue> Encoded for T {
    fn encoded(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.encode_value(&mut out);
        out
    }
}

/// The worked example encodes to its exact bytes: no length for the VARINT or the I32,
/// a varint length for the LEN, and nothing for the unset field
#[test]
#[rustfmt::skip]
fn encode_measurement_exact_bytes() {
    let wire = vec![
        // id = 5
        0x08, 0x05,
        // ratio = 1.5
        0x15, 0x00, 0x00, 0xC0, 0x3F,
        // name = "hi"
        0x1A, 0x02, 0x68, 0x69,
    ];
    assert_eq!(Measurement::unstamped().encoded(), wire);
    assert_eq!(Measurement::decode_value(&mut &wire[..]).unwrap(), Measurement::unstamped());
}

/// An empty LEN value is framed by its tag and a zero length, with no value bytes
#[test]
#[rustfmt::skip]
fn encode_measurement_empty_len_value() {
    let unnamed = Measurement { name: String::new(), ..Measurement::unstamped() };
    let wire = vec![
        // id = 5
        0x08, 0x05,
        // ratio = 1.5
        0x15, 0x00, 0x00, 0xC0, 0x3F,
        // name = ""
        0x1A, 0x00,
    ];
    assert_eq!(unnamed.encoded(), wire);
    assert_eq!(Measurement::decode_value(&mut &wire[..]).unwrap(), unnamed);
}

/// A set wire-1 I64 field is framed by its tag alone, followed by 8 LE bytes
#[test]
#[rustfmt::skip]
fn encode_measurement_fixed64_has_no_length() {
    let stamped = Measurement { created_at: Some(9), ..Measurement::unstamped() };
    let mut wire = Measurement::unstamped().encoded();
    wire.extend([
        // created_at = 9
        0x21, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    assert_eq!(stamped.encoded(), wire);
    assert_eq!(Measurement::decode_value(&mut &wire[..]).unwrap(), stamped);
}

/// The canonical message encodes to its hand-built wire byte for byte: the nested message,
/// the packed repeated, the custom enum, and the two-byte tag and length of `note`
#[test]
fn encode_example_matches_wire() {
    assert_eq!(Example::decoded().encoded(), Example::wire());
}

/// The canonical message survives encode then decode unchanged
#[test]
fn encode_example_roundtrips() {
    let wire = Example::decoded().encoded();
    assert_eq!(
        Example::decode_value(&mut &wire[..]).unwrap(),
        Example::decoded()
    );
}

/// A two-byte inline varint encodes to the same bytes the decode tests hand-build
#[test]
fn encode_example_two_byte_varint() {
    let example = Example {
        id: 300,
        ..Example::decoded()
    };
    let wire = example.encoded();
    assert_eq!(wire, Example::wire_id_300());
    assert_eq!(Example::decode_value(&mut &wire[..]).unwrap(), example);
}

/// A negative `int32` is sign-extended to a ten-byte varint and decodes back to itself
#[test]
fn encode_example_negative_varint_roundtrips() {
    let example = Example {
        id: -1,
        ..Example::decoded()
    };
    let wire = example.encoded();
    let mut expected = Example::wire();
    // --------------------------------------------------
    // the id varint 0x05 becomes nine 0xFF continuation bytes and a final 0x01
    // --------------------------------------------------
    expected.splice(
        1..2,
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01],
    );
    assert_eq!(wire, expected);
    assert_eq!(Example::decode_value(&mut &wire[..]).unwrap(), example);
}

/// The nested message encodes standalone to the bytes its parent frames as a LEN value
#[test]
fn encode_inner_standalone() {
    let wire = Inner { code: 7 }.encoded();
    assert_eq!(wire, [0x08, 0x07]);
    assert_eq!(
        Inner::decode_value(&mut &wire[..]).unwrap(),
        Inner { code: 7 }
    );
}

/// Unset optional fields are omitted entirely, leaving only the required field
#[test]
fn encode_reading_count_only() {
    let wire = Reading::count_only().encoded();
    assert_eq!(wire, [0x08, 0x03]);
    assert_eq!(
        Reading::decode_value(&mut &wire[..]).unwrap(),
        Reading::count_only()
    );
}

/// Every optional field set, with `level` and `size` each taking two bytes, survives encode then decode
#[test]
#[rustfmt::skip]
fn encode_reading_roundtrips() {
    let reading = Reading {
        count: 3,
        label: Some("ab".to_owned()),
        status: Some(Status::Error),
        level: Some(200),
        size: Some(BerLength::new(200)),
    };
    let wire = vec![
        // count = 3
        0x08, 0x03,
        // label = "ab"
        0x12, 0x02, 0x61, 0x62,
        // status = Error
        0x18, 0x02,
        // level = 200, a two-byte BER-OID
        0x20, 0x81, 0x48,
        // size = 200, a long-form BER length
        0x28, 0x81, 0xC8,
    ];
    assert_eq!(reading.encoded(), wire);
    assert_eq!(Reading::decode_value(&mut &wire[..]).unwrap(), reading);
}

/// A generic message codes its `trait_fallback` field through `T`, and encoding appends after
/// whatever the buffer already holds
#[test]
#[rustfmt::skip]
fn encode_generic_trait_fallback_appends() {
    let tagged = Tagged { count: 3, value: Status::Error };
    let prefix = [0xDE, 0xAD];
    let mut out = prefix.to_vec();
    tagged.encode_value(&mut out);
    let wire = [
        // count = 3
        0x08, 0x03,
        // value = Error, through `Status`'s `EncodeValue`
        0x10, 0x02,
    ];
    assert_eq!(out, [&prefix[..], &wire[..]].concat());
    assert_eq!(Tagged::<Status>::decode_value(&mut &wire[..]).unwrap(), tagged);
}

/// Every [`Status`] variant survives its `u64` conversion and its VARINT codec
#[test]
fn status_roundtrips_every_variant() {
    for status in Status::ALL {
        assert_eq!(Status::try_from(u64::from(status)), Ok(status));
        let wire = status.encoded();
        assert_eq!(Status::decode_value(&mut &wire[..]), Ok(status));
    }
}
