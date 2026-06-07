//! String decode codecs for KLV data
//!
//! Variable-length string decoders that consume `len` bytes
//! and produce a [`String`]. All decoders are compatible with the [`winnow`]
//! streaming parser framework and accept `&mut &[u8]` input
//!
//! Includes:
//! * UTF-8 decoders (lossy and strict)
//! * UTF-16 decoders (little-endian and big-endian)
//! * ASCII string decoder (validates 7-bit ASCII)
//! * ASCII-text *value* decoders: base-10 ints (`u8`..`u128`, `i8`..`i128`),
//!   floats (`f32`/`f64`), base-16 ints (`hex_*`), and `alpha`/`digit`/
//!   `alphanumeric` run validators
//!
//! ASCII-encoded values are length-bounded: they consume exactly `len` bytes
//! and parse the whole field. The decimal integer decoders inherit winnow's
//! `dec_uint`/`dec_int` grammar (bare `0` or a non-zero-led run; leading zeros
//! rejected); the hex decoders tolerate leading zeros
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
#[cfg(feature = "ascii")]
use winnow::ascii::{alpha1, alphanumeric1, dec_int, dec_uint, digit1, float, hex_uint};
#[cfg(feature = "ascii")]
use winnow::combinator::{eof, terminated};
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
        take(len)
            .try_map(|slice: &[u8]| core::str::from_utf8(slice).map(str::to_owned))
            .context(winnow::error::StrContext::Label(
                "Unable to decode bytes as UTF-8",
            ))
            .parse_next(input)
    }
}

#[inline(always)]
/// Decodes a byte slice into a [`String`] assuming little-endian UTF-16, using [`String::from_utf16_lossy`]
///
/// **Endianness warning**: Using the wrong endianness variant will silently
/// produce corrupted string data. Verify the endianness of your KLV stream
/// before selecting a variant
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
        if !len.is_multiple_of(2) {
            let checkpoint = input.checkpoint();
            return Err(crate::__export::labeled_error(
                input,
                &checkpoint,
                "Invalid UTF-16 slice length",
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
/// before selecting a variant
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
        if !len.is_multiple_of(2) {
            let checkpoint = input.checkpoint();
            return Err(crate::__export::labeled_error(
                input,
                &checkpoint,
                "Invalid UTF-16 slice length",
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

#[cfg(feature = "ascii")]
#[inline(always)]
/// Decodes `len` bytes into a [`String`], validating that every byte is 7-bit ASCII
///
/// Rejects the field if any byte is `>= 0x80` (including multi-byte UTF-8), which
/// distinguishes this from [`to_string_utf8`]. Since ASCII is a subset of UTF-8,
/// the validated bytes convert to a [`String`] without loss
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
        take(len)
            .verify(<[u8]>::is_ascii)
            // bytes are verified ASCII above, so `from_utf8_lossy` never substitutes
            .map(|slice: &[u8]| String::from_utf8_lossy(slice).into_owned())
            .context(winnow::error::StrContext::Label(
                "Unable to decode bytes as ASCII",
            ))
            .parse_next(input)
    }
}

/// Generates a length-bounded base-10 unsigned integer decoder
macro_rules! ascii_uint {
    ($ty:ty) => {
        pastey::paste! {
            #[cfg(feature = "ascii")]
            #[inline(always)]
            #[doc = concat!("Decodes exactly `len` ASCII bytes as a base-10 [`", stringify!($ty), "`].")]
            #[doc = ""]
            #[doc = "The entire `len`-byte field must be the decimal numeral; any"]
            #[doc = "trailing or non-digit byte is rejected."]
            pub fn [<$ty>](len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<$ty> {
                move |input| {
                    take(len)
                        .and_then(terminated(dec_uint::<_, $ty, _>, eof))
                        .parse_next(input)
                }
            }
        }
    };
}

/// Generates a length-bounded base-10 signed integer decoder
macro_rules! ascii_int {
    ($ty:ty) => {
        pastey::paste! {
            #[cfg(feature = "ascii")]
            #[inline(always)]
            #[doc = concat!("Decodes exactly `len` ASCII bytes as a base-10 [`", stringify!($ty), "`].")]
            #[doc = ""]
            #[doc = "Accepts an optional leading `+`/`-` sign. The entire `len`-byte"]
            #[doc = "field must be the numeral; any trailing byte is rejected."]
            pub fn [<$ty>](len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<$ty> {
                move |input| {
                    take(len)
                        .and_then(terminated(dec_int::<_, $ty, _>, eof))
                        .parse_next(input)
                }
            }
        }
    };
}

/// Generates a length-bounded floating-point decoder
macro_rules! ascii_float {
    ($ty:ty) => {
        pastey::paste! {
            #[cfg(feature = "ascii")]
            #[inline(always)]
            #[doc = concat!("Decodes exactly `len` ASCII bytes as an [`", stringify!($ty), "`].")]
            #[doc = ""]
            #[doc = "Accepts standard textual float syntax (sign, exponent, `inf`,"]
            #[doc = "`nan`). The entire `len`-byte field must be the number."]
            pub fn [<$ty>](len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<$ty> {
                move |input| {
                    take(len)
                        .and_then(terminated(float::<_, $ty, _>, eof))
                        .parse_next(input)
                }
            }
        }
    };
}

/// Generates a length-bounded base-16 unsigned integer decoder
macro_rules! ascii_hex {
    ($ty:ty) => {
        pastey::paste! {
            #[cfg(feature = "ascii")]
            #[inline(always)]
            #[doc = concat!("Decodes exactly `len` ASCII hex bytes as a [`", stringify!($ty), "`].")]
            #[doc = ""]
            #[doc = "Accepts upper- or lower-case hex digits. The entire `len`-byte"]
            #[doc = "field must be hexadecimal; any trailing byte is rejected."]
            pub fn [<hex_ $ty>](len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<$ty> {
                move |input| {
                    take(len)
                        .and_then(terminated(hex_uint::<_, $ty, _>, eof))
                        .parse_next(input)
                }
            }
        }
    };
}

/// Generates a length-bounded ASCII character-class run validator
macro_rules! ascii_run {
    ($name:ident, $parser:expr, $class:literal) => {
        #[cfg(feature = "ascii")]
        #[inline(always)]
        #[doc = concat!("Decodes exactly `len` ASCII ", $class, " bytes into a [`String`].")]
        #[doc = ""]
        #[doc = concat!("Rejects the field unless every one of the `len` bytes is ", $class, ".")]
        pub fn $name(len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<String> {
            move |input| {
                take(len)
                    .and_then(terminated($parser, eof))
                    // the run parser only matches ASCII, so `from_utf8_lossy` never substitutes
                    .map(|run: &[u8]| String::from_utf8_lossy(run).into_owned())
                    .parse_next(input)
            }
        }
    };
}

ascii_uint!(u8);
ascii_uint!(u16);
ascii_uint!(u32);
ascii_uint!(u64);
ascii_uint!(u128);

ascii_int!(i8);
ascii_int!(i16);
ascii_int!(i32);
ascii_int!(i64);
ascii_int!(i128);

ascii_float!(f32);
ascii_float!(f64);

ascii_hex!(u8);
ascii_hex!(u16);
ascii_hex!(u32);
ascii_hex!(u64);
ascii_hex!(u128);

ascii_run!(alpha, alpha1, "alphabetic");
ascii_run!(digit, digit1, "decimal-digit");
ascii_run!(alphanumeric, alphanumeric1, "alphanumeric");
