//! Decode tests for a generic `debug` struct with the fused `#[klv(keylen(..))]` codec
//!
//! Its wire decodes in both drivers, and a malformed `Inline` value is rejected in both, naming
//! the field
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::common::drive::Drive;
use crate::common::varint;
use crate::support::example::Status;
use crate::support::{header, value};
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
    keylen(dec = header::dec_tag),
    allow_unimplemented_encode,
    trait_fallback,
    debug,
    default(typ = String, dec = value::dec_string),
)]
struct Traced<T>
where
    T: for<'a> DecodeValue<&'a [u8]> + Debug,
{
    /// field 1, wire 0 VARINT - `Inline`, `i32` on the wire
    #[klv(key = 0x08, dec = varint::dec_i32, latebind = i64::from)]
    total: i64,

    /// field 2, wire 2 LEN - `Sliced(n)`
    #[klv(key = 0x12)]
    label: String,

    /// field 3, wire 0 VARINT - `Inline`, no `dec`: read by `trait_fallback`
    #[klv(key = 0x18)]
    value: Option<T>,
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
