//! Matrix tests for the `enc` dispatch sigils: none / `&`
//!
//! Covers both call-shapes in the Encode codegen template:
//! * `enc = func`  -> `func(&self.field)`                          (`Fn(&T) -> O`)
//! * `enc = *func` -> `func(EncodeAs::encode_as(&self.field))`     dispatches via
//!   the [`EncodeAs`] trait: primitives by value (Copy), `String -> &str`,
//!   `Vec<T> -> &[T]`, `Box<T>/Rc<T>/Arc<T> -> &T`. No clone, no heap alloc.
//!
//! Each sigil is exercised on both a required field and an `Option<T>` field
//! so the optional branch of the codegen is also verified. Roundtrip via
//! `decode_frame` / `encode_frame` confirms the generated closure shapes
//! type-check and produce identical bytes
use super::types::{Color, Priority};
use tinyklv::dec::binary as decb;
use tinyklv::enc::binary as encb;
use tinyklv::prelude::*;

use std::rc::Rc;
use std::sync::Arc;

/// `None` sigil target: takes `&T`
fn enc_u8_ref(v: &u8) -> Vec<u8> {
    encb::u8(*v)
}

/// `&` sigil target for a primitive: takes owned `u16` (Copy, zero-cost)
fn enc_u16_owned(v: u16) -> Vec<u8> {
    encb::be_u16(v)
}

/// `&` sigil target for `String` via `EncodeAs` -> `&str`
fn enc_str_ref(s: &str) -> Vec<u8> {
    let mut out = encb::u8_from_usize(s.len());
    out.extend_from_slice(s.as_bytes());
    out
}

/// `&` sigil target for `Vec<u8>` via `EncodeAs` -> `&[u8]`
fn enc_bytes_ref(b: &[u8]) -> Vec<u8> {
    let mut out = encb::u8_from_usize(b.len());
    out.extend_from_slice(b);
    out
}

/// `None` sigil decode counterpart for length-prefixed strings
fn dec_u8_len_string(input: &mut &[u8]) -> tinyklv::Result<String> {
    let len = decb::u8(input)? as usize;
    let (head, rest) = input.split_at(len);
    *input = rest;
    Ok(String::from_utf8_lossy(head).into_owned())
}

/// `None` sigil decode counterpart for length-prefixed bytes
fn dec_u8_len_bytes(input: &mut &[u8]) -> tinyklv::Result<Vec<u8>> {
    let len = decb::u8(input)? as usize;
    let (head, rest) = input.split_at(len);
    *input = rest;
    Ok(head.to_vec())
}

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    sentinel = b"\x00\x00\x00\x01",
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct AllSigils {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = enc_u8_ref,
    )]
    /// None sigil: encoder takes &u8
    count: u8,

    #[klv(
        key = 0x02,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    /// None sigil: encoder takes &Color (method form)
    color: Color,

    #[klv(
        key = 0x03,
        dec = decb::be_u16,
        enc = *enc_u16_owned,
    )]
    /// & sigil: primitive by value through EncodeAs
    id: u16,

    #[klv(
        key = 0x04,
        dec = dec_u8_len_string,
        enc = &enc_str_ref,
    )]
    /// & sigil: String -> &str through EncodeAs
    label: String,

    #[klv(
        key = 0x05,
        dec = dec_u8_len_bytes,
        enc = &enc_bytes_ref,
    )]
    /// & sigil: Vec<u8> -> &[u8] through EncodeAs
    blob: Vec<u8>,
}

#[test]
/// Verifies frame roundtrip across a struct that mixes no-sigil and `&` sigil encoders covering `u8`, custom enums, `u16`, `String`, and `Vec<u8>` fields.
fn all_sigils_roundtrip() {
    let original = AllSigils {
        count: 42,
        color: Color::Blue,
        id: 0x1234,
        label: String::from("sigil"),
        blob: vec![0xDE, 0xAD, 0xBE, 0xEF],
    };
    let bytes = original.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = AllSigils::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(original, decoded);
}

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    sentinel = b"\x00\x00\x00\x02",
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct AllSigilsOptional {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = enc_u8_ref,
    )]
    count: Option<u8>,

    #[klv(
        key = 0x02,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    color: Option<Color>,

    #[klv(
        key = 0x03,
        dec = decb::be_u16,
        enc = *enc_u16_owned,
    )]
    id: Option<u16>,

    #[klv(
        key = 0x04,
        dec = dec_u8_len_string,
        enc = &enc_str_ref,
    )]
    label: Option<String>,

    #[klv(
        key = 0x05,
        dec = dec_u8_len_bytes,
        enc = &enc_bytes_ref,
    )]
    blob: Option<Vec<u8>>,
}

#[test]
/// Verifies that the `&` sigil dispatches through `EncodeAs` correctly for `Option<T>` fields when every optional is `Some(_)`.
fn all_sigils_optional_all_present() {
    let original = AllSigilsOptional {
        count: Some(7),
        color: Some(Color::Red),
        id: Some(0xCAFE),
        label: Some(String::from("hi")),
        blob: Some(vec![1, 2, 3]),
    };
    let bytes = original.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = AllSigilsOptional::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(original, decoded);
}

#[test]
/// Tests that when every `Option<T>` field is `None`, both sigil code paths emit nothing and the frame decodes back to all-`None`.
fn all_sigils_optional_all_absent() {
    let original = AllSigilsOptional {
        count: None,
        color: None,
        id: None,
        label: None,
        blob: None,
    };
    let bytes = original.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = AllSigilsOptional::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(original, decoded);
}

#[test]
/// Tests a mixed `Some`/`None` pattern across every sigil-dispatched optional field to exercise the optional branch of the codegen.
fn all_sigils_optional_partial() {
    let original = AllSigilsOptional {
        count: Some(99),
        color: None,
        id: Some(0x0042),
        label: Some(String::from("partial")),
        blob: None,
    };
    let bytes = original.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = AllSigilsOptional::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(original, decoded);
}

// --------------------------------------------------
// `&` sigil on smart-pointer wrappers - EncodeAs dispatches to `&T`
// --------------------------------------------------
fn enc_inner_u32_ref(v: &u32) -> Vec<u8> {
    encb::be_u32(*v)
}

fn enc_inner_str_ref(s: &str) -> Vec<u8> {
    let mut out = encb::u8_from_usize(s.len());
    out.extend_from_slice(s.as_bytes());
    out
}

fn dec_box_u32(input: &mut &[u8]) -> tinyklv::Result<Box<u32>> {
    decb::be_u32(input).map(Box::new)
}

fn dec_rc_u32(input: &mut &[u8]) -> tinyklv::Result<Rc<u32>> {
    decb::be_u32(input).map(Rc::new)
}

fn dec_arc_str(input: &mut &[u8]) -> tinyklv::Result<Arc<str>> {
    let len = decb::u8(input)? as usize;
    let (head, rest) = input.split_at(len);
    *input = rest;
    Ok(Arc::from(String::from_utf8_lossy(head).as_ref()))
}

#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    sentinel = b"\x00\x00\x00\x03",
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct SmartPointerSigils {
    #[klv(
        key = 0x01,
        dec = dec_box_u32,
        enc = &enc_inner_u32_ref,
    )]
    boxed: Box<u32>,
    #[klv(
        key = 0x02,
        dec = dec_rc_u32,
        enc = &enc_inner_u32_ref,
    )]
    counted: Rc<u32>,
    #[klv(
        key = 0x03,
        dec = dec_arc_str,
        enc = &enc_inner_str_ref,
    )]
    shared: Arc<str>,
}

#[test]
/// Verifies that the `&` sigil dispatches through `EncodeAs` for smart-pointer fields (`Box<T>`, `Rc<T>`, `Arc<str>`) without cloning.
fn smart_pointer_sigils_roundtrip() {
    let original = SmartPointerSigils {
        boxed: Box::new(0xAABBCCDD),
        counted: Rc::new(0x11223344),
        shared: Arc::from("arc"),
    };
    let bytes = original.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = SmartPointerSigils::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(original, decoded);
}

// --------------------------------------------------
// Every sigil in one struct, mixed with multiple fields of each
// to confirm codegen emits one call-shape per attribute, not per-type
// --------------------------------------------------
#[derive(Klv, Debug, Clone, PartialEq)]
#[klv(
    sentinel = b"\x00\x00\x00\x04",
    stream = &[u8],
    key(dec = decb::u8, enc = encb::u8),
    len(dec = decb::u8_as_usize, enc = encb::u8_from_usize),
)]
struct SigilMixture {
    #[klv(
        key = 0x01,
        dec = decb::u8,
        enc = enc_u8_ref,
    )]
    a: u8,

    #[klv(
        key = 0x02,
        dec = decb::u8,
        enc = *encb::u8,
    )]
    b: u8,

    #[klv(
        key = 0x03,
        dec = Priority::decode_value,
        enc = Priority::encode_value,
    )]
    pri: Priority,

    #[klv(
        key = 0x04,
        dec = Color::decode_value,
        enc = Color::encode_value,
    )]
    col: Color,

    #[klv(
        key = 0x05,
        dec = dec_u8_len_string,
        enc = &enc_str_ref,
    )]
    label: String,
}

#[test]
/// Tests that a struct mixing both sigil shapes across multiple fields of each type codegens one call-shape per attribute and roundtrips correctly.
fn sigil_mixture_roundtrip() {
    let original = SigilMixture {
        a: 1,
        b: 2,
        pri: Priority::High,
        col: Color::Green,
        label: String::from("mix"),
    };
    let bytes = original.encode_frame();
    let mut slice = bytes.as_slice();
    let decoded = SigilMixture::decode_frame(&mut slice).expect("decode_frame");
    assert_eq!(original, decoded);
}
