//! Encode tests for the fused `#[klv(keylen(enc = ..))]` codec
//!
//! Every field is staged, framed by the fused encoder - the tag, plus a length only for
//! wire 2 LEN - and appended; a skipped field writes no frame. These tests pin the exact bytes
//! of that framing for every wire shape
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::support::example::{Example, Reading, Status};
use crate::support::{header, value};
#[cfg(feature = "ber")]
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
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32),
    default(typ = String, dec = value::dec_string, enc = &encs::from_string_utf8),
    default(typ = u64, dec = decb::le_u64, enc = *encb::le_u64),
)]
struct Measurement {
    /// field 1, wire 0 VARINT - tag, varint, no length
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire 5 I32 - tag, 4 LE bytes, no length
    #[klv(key = 0x15)]
    ratio: f32,

    /// field 3, wire 2 LEN - tag, varint length, bytes
    #[klv(key = 0x1A)]
    name: String,

    /// field 4, wire 1 I64 - tag, 8 LE bytes, no length
    #[klv(key = 0x21)]
    created_at: Option<u64>,
}

/// [`Measurement`] implementation
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
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    trait_fallback,
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32),
)]
struct Tagged<T>
where
    T: for<'a> DecodeValue<&'a [u8]> + EncodeValue,
{
    /// field 1, wire 0 VARINT
    #[klv(key = 0x08)]
    count: i32,

    /// field 2, wire 0 VARINT - `Inline`, read and written through the `T` trait impls
    #[klv(key = 0x10)]
    value: T,
}

/// The first two fields of [`Reading`], encode only: neither `keylen` nor any field has a `dec`
#[derive(Klv, Debug)]
#[klv(keylen(enc = header::enc_tag), allow_unimplemented_decode)]
struct Outgoing {
    /// field 1, wire 0 VARINT
    #[klv(key = 0x08, enc = *varint::enc_i32)]
    count: i32,

    /// field 2, wire 2 LEN
    #[klv(key = 0x12, enc = &encs::from_string_utf8)]
    label: Option<String>,
}

/// [`Measurement`] as proto3 writes it: each scalar left out at its default, by its type's
/// `default(..)`, and `name` left out when empty, by its own `skip_if`
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = header::dec_tag, enc = header::enc_tag),
    default(typ = i32, dec = varint::dec_i32, enc = *varint::enc_i32, skip_if_default),
    default(typ = f32, dec = decb::le_f32, enc = *encb::le_f32, skip_if_default),
    default(typ = String, dec = value::dec_string, enc = &encs::from_string_utf8),
)]
struct Proto3Measurement {
    /// field 1, wire 0 VARINT - left out at 0
    #[klv(key = 0x08)]
    id: i32,

    /// field 2, wire 5 I32 - left out at 0.0
    #[klv(key = 0x15)]
    ratio: f32,

    /// field 3, wire 2 LEN - left out when empty
    #[klv(key = 0x1A, skip_if = String::is_empty)]
    name: String,
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

/// The canonical message encodes to its hand-built wire byte for byte: the nested message,
/// the packed repeated, the custom enum, and the two-byte tag and length of `note`
#[test]
fn encode_example_matches_wire() {
    assert_eq!(Example::decoded().encoded(), Example::wire());
}

/// Every optional field set, with `level` and `size` each taking two bytes, survives encode then decode
#[cfg(feature = "ber")]
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

/// An encode-only message frames its fields exactly as [`Reading`] reads them back
#[test]
#[rustfmt::skip]
fn encode_only_keylen_decodes_as_reading() {
    let outgoing = Outgoing { count: 3, label: Some("ab".to_owned()) };
    let wire = vec![
        // count = 3
        0x08, 0x03,
        // label = "ab"
        0x12, 0x02, 0x61, 0x62,
    ];
    assert_eq!(outgoing.encoded(), wire);
    assert_eq!(
        Reading::decode_value(&mut &wire[..]),
        Ok(Reading { label: Some("ab".to_owned()), ..Reading::count_only() })
    );
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

/// A field at its default under its type's `skip_if_default`, and one its own `skip_if` holds
/// for, write no frame, and their missing frames decode back to their defaults in both drivers
/// and at every chunk split
#[test]
fn skipped_fields_write_no_frame() {
    let unnamed = Proto3Measurement {
        id: 0,
        ratio: 1.5,
        name: String::new(),
    };
    #[rustfmt::skip]
    let wire = [
        // ratio = 1.5
        0x15, 0x00, 0x00, 0xC0, 0x3F,
    ];
    assert_eq!(unnamed.encoded(), wire);
    unnamed.assert_decodes_from(&wire);
}
