//! String decode codecs for KLV data
//!
//! Variable-length string decoders that consume `len` bytes from the wire
//! and produce a [`String`]. All decoders are compatible with the [`winnow`]
//! streaming parser framework and accept `&mut &[u8]` input.
//!
//! Includes:
//! * UTF-8 decoders (lossy and strict)
//! * UTF-16 decoders (little-endian and big-endian)
//! * ASCII decoder (gated behind the `ascii` feature)
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::token::take;

#[inline(always)]
/// Decodes a byte slice into a [`String`], using [`String::from_utf8_lossy`]
///
/// To decode in a more strict manner, please see [`to_string_utf8_strict`]
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::dec::to_string_utf8;
///
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
///
/// let res1 = to_string_utf8(6)(&mut val1);
/// let res2 = to_string_utf8(9)(&mut val2);
///
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string_utf8(len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<String> {
    move |input| {
        take(len)
            .map(|slice| String::from_utf8_lossy(slice).to_string())
            .parse_next(input)
    }
}

#[inline(always)]
/// Decodes a byte slice into a [`String`], using [`String::from_utf8`]
///
/// To decode in a more relaxed manner, please see [`to_string_utf8`]
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::dec::to_string_utf8_strict;
///
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
///
/// let res1 = to_string_utf8_strict(6)(&mut val1);
/// let res2 = to_string_utf8_strict(9)(&mut val2);
///
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string_utf8_strict(len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<String> {
    move |input| {
        let checkpoint = input.checkpoint();
        match String::from_utf8(take(len).parse_next(input)?.to_vec()) {
            Ok(s) => Ok(s),
            Err(_) => Err(winnow::error::ContextError::new().add_context(
                input,
                &checkpoint,
                winnow::error::StrContext::Label(
                    "Unable to decode string using `String::from_utf8`",
                ),
            )),
        }
    }
}

#[inline(always)]
/// Decodes a byte slice into a [`String`] assuming little-endian UTF-16, using [`String::from_utf16_lossy`]
///
/// **Endianness warning**: Using the wrong endianness variant will silently
/// produce corrupted string data. Verify the endianness of your KLV stream
/// before selecting a variant.
///
/// # Example
///
/// ```
/// let mut input: &[u8] = &[0x41, 0x00, 0x42, 0x00]; // "AB" in UTF-16 LE
/// let result = tinyklv::codecs::string::dec::to_string_utf16_le(4)(&mut input);
/// assert_eq!(result, Ok(String::from("AB")));
/// ```
pub fn to_string_utf16_le(len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<String> {
    move |input| {
        let checkpoint = input.checkpoint();
        if !len.is_multiple_of(2) {
            return Err(winnow::error::ContextError::new().add_context(
                input,
                &checkpoint,
                winnow::error::StrContext::Label("Invalid UTF-16 slice length"),
            ));
        }
        take(len)
            .map(|slice: &[u8]| {
                let utf16: Vec<u16> = slice
                    .chunks_exact(2)
                    .map(|chunk| {
                        #[allow(
                            clippy::unwrap_used,
                            reason = "safe to unwrap, since `chunks_exact` returns exactly 2 bytes"
                        )]
                        let array: [u8; 2] = chunk.try_into().unwrap();
                        u16::from_le_bytes(array)
                    })
                    .collect();
                String::from_utf16_lossy(&utf16)
            })
            .parse_next(input)
    }
}

#[inline(always)]
/// Decodes a byte slice into a [`String`] assuming big-endian UTF-16, using [`String::from_utf16_lossy`]
///
/// **Endianness warning**: Using the wrong endianness variant will silently
/// produce corrupted string data. Verify the endianness of your KLV stream
/// before selecting a variant.
///
/// # Example
///
/// ```
/// let mut input: &[u8] = &[0x00, 0x41, 0x00, 0x42]; // "AB" in UTF-16 BE
/// let result = tinyklv::codecs::string::dec::to_string_utf16_be(4)(&mut input);
/// assert_eq!(result, Ok(String::from("AB")));
/// ```
pub fn to_string_utf16_be(len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<String> {
    move |input| {
        let checkpoint = input.checkpoint();
        if !len.is_multiple_of(2) {
            return Err(winnow::error::ContextError::new().add_context(
                input,
                &checkpoint,
                winnow::error::StrContext::Label("Invalid UTF-16 slice length"),
            ));
        }
        take(len)
            .map(|slice: &[u8]| {
                let utf16: Vec<u16> = slice
                    .chunks_exact(2)
                    .map(|chunk| {
                        #[allow(
                            clippy::unwrap_used,
                            reason = "safe to unwrap, since `chunks_exact` returns exactly 2 bytes"
                        )]
                        let array: [u8; 2] = chunk.try_into().unwrap();
                        u16::from_be_bytes(array)
                    })
                    .collect();
                String::from_utf16_lossy(&utf16)
            })
            .parse_next(input)
    }
}

#[inline(always)]
#[cfg(feature = "ascii")]
/// Decodes a byte slice into a [`String`], using [`ascii::AsciiString::from_ascii`]
///
/// # Example
///
/// ```
/// use tinyklv::codecs::string::dec::to_string_ascii;
///
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
///
/// let res1 = to_string_ascii(6)(&mut val1);
/// let res2 = to_string_ascii(9)(&mut val2);
///
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string_ascii(len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<String> {
    move |input| {
        let checkpoint = input.checkpoint();
        match ascii::AsciiString::from_ascii(take(len).parse_next(input)?) {
            Ok(s) => Ok(s.to_string()),
            Err(_) => Err(winnow::error::ContextError::new().add_context(
                input,
                &checkpoint,
                winnow::error::StrContext::Label(
                    "Unable to decode string using `ascii::AsciiString::from_ascii`",
                ),
            )),
        }
    }
}
