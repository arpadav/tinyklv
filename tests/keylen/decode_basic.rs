//! Whole-message decode tests for the fused `#[klv(keylen(..))]` codec
//!
//! Decodes the mixed-wire [`Example`] end to end and asserts the decoded struct,
//! proving the `Sliced`/`Inline` split routes every wire shape correctly. The
//! generic `debug` struct runs through both drivers
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::support::example::{Example, Inner, Status};
use crate::support::pb;
use tinyklv::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use std::fmt::Debug;

/// A generic `debug` keylen struct: an `Inline` field widened by `latebind`, a `Sliced`
/// field, and an `Inline` `T` read through the `DecodeValue` fallback
#[derive(Klv, Debug, PartialEq)]
#[klv(
    stream = &[u8],
    keylen(dec = pb::dec_tag_keylen),
    allow_unimplemented_encode,
    trait_fallback,
    debug,
    default(typ = String, dec = pb::dec_string),
)]
struct Traced<T>
where
    T: for<'a> DecodeValue<&'a [u8]> + Debug,
{
    /// field 1, wire 0 VARINT - `Inline`, `i32` on the wire
    #[klv(key = 0x08, dec = pb::dec_varint_i32, latebind = i64::from)]
    total: i64,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x12)]
    label: String,

    /// field 3, wire 0 VARINT - `Inline`, no `dec`: read by `trait_fallback`
    #[klv(key = 0x18)]
    value: Option<T>,
}

/// Decodes the full mixed-wire message and asserts every field, covering both
/// the `Inline` framing and every `Sliced` width in one pass
#[test]
fn decode_full_message() {
    let wire = Example::wire();
    let decoded = Example::decode_value(&mut &wire[..]).unwrap();
    assert_eq!(decoded, Example::decoded());
}

/// Decodes the nested message on its own, confirming a `keylen` struct works as a
/// standalone `DecodeValue` (the same path the parent invokes on the sub-slice)
#[test]
fn decode_nested_standalone() {
    // --------------------------------------------------
    // inner body only: code = 7 (field 1, wire 0 VARINT, Inline)
    // --------------------------------------------------
    let wire = [0x08, 0x07];
    let decoded = Inner::decode_value(&mut &wire[..]).unwrap();
    assert_eq!(decoded, Inner { code: 7 });
}

/// Decodes the fields in reverse wire order, proving dispatch is by key (not position)
/// for both inline and sliced frames
#[test]
fn decode_order_independent() {
    let mut frames = Example::frames();
    frames.reverse();
    let wire = frames.concat();
    let decoded = Example::decode_value(&mut &wire[..]).unwrap();
    assert_eq!(decoded, Example::decoded());
}

/// Decodes a full message whose inline `id` is a multi-byte varint: the
/// self-delimiting varint must consume exactly its continuation bytes so every
/// following `Sliced` frame stays aligned
#[test]
fn decode_multibyte_inline_varint() {
    let wire = Example::wire_id_300();
    let decoded = Example::decode_value(&mut &wire[..]).unwrap();
    assert_eq!(
        decoded,
        Example {
            id: 300,
            ..Example::decoded()
        }
    );
}

/// The generic `debug` struct decodes through both drivers
#[test]
fn decode_debug_struct() {
    #[rustfmt::skip]
    let wire = [
        // total = 7 (Inline)
        0x08, 0x07,
        // label = "hi" (Sliced(2))
        0x12, 0x02, 0x68, 0x69,
        // value = Active (Inline)
        0x18, 0x01,
    ];
    let expected = Traced {
        total: 7,
        label: "hi".to_owned(),
        value: Some(Status::Active),
    };
    assert_eq!(Traced::decode_value(&mut &wire[..]).as_ref(), Ok(&expected));
    expected.assert_streams_to(&mut &wire[..]);
}

/// A `debug` struct whose fallback-decoded `Inline` value fails mid-buffer is malformed in
/// both drivers
#[test]
fn decode_debug_struct_rejected_value() {
    #[rustfmt::skip]
    let wire = [
        // value = 9, not a `Status` discriminant
        0x18, 0x09,
        // total = 7
        0x08, 0x07,
    ];
    Traced::<Status>::assert_rejects_naming(&wire, &["Traced::value"]);
}
