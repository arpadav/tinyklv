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
//! Every decoder here that finds fewer than `len` bytes leaves the cursor at
//! the end of the input, a [`crate::ErrorKind::Truncated`] error. One that
//! rejects the bytes it read rewinds the cursor to where the value started, a
//! [`crate::ErrorKind::MalformedValue`] error
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
use crate::{
    __export::{kinded_error, relabeled_error},
    codecs::take_or_finish,
    prelude::*,
};

// --------------------------------------------------
// external
// --------------------------------------------------
#[cfg(feature = "ascii")]
use winnow::ascii::{alpha1, alphanumeric1, dec_int, dec_uint, digit1, float, hex_uint};
#[cfg(feature = "ascii")]
use winnow::combinator::{eof, terminated};
use winnow::error::{ContextError, FromExternalError};

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The label of a UTF-8 decoder's error
const UTF8_STRING: &str = "UTF-8 string";

/// The label of a UTF-16 decoder's short read
const UTF16_STRING: &str = "UTF-16 string";

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
        take_decoded(input, len, UTF8_STRING, |bytes| {
            Ok(String::from_utf8_lossy(bytes).to_string())
        })
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
        take_decoded(input, len, UTF8_STRING, |bytes| {
            core::str::from_utf8(bytes)
                .map(str::to_owned)
                .map_err(|cause| ContextError::from_external_error(&bytes, cause))
        })
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
            let err = crate::__export::labeled_error(
                input,
                &checkpoint,
                "UTF-16 string length (an odd number of bytes)",
            );
            return Err(crate::__export::with_kind(err, ErrorKind::WrongLength));
        }
        take_decoded(input, len, UTF16_STRING, |bytes| {
            let utf16: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_le_bytes(*pair))
                .collect();
            Ok(String::from_utf16_lossy(&utf16))
        })
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
            let err = crate::__export::labeled_error(
                input,
                &checkpoint,
                "UTF-16 string length (an odd number of bytes)",
            );
            return Err(crate::__export::with_kind(err, ErrorKind::WrongLength));
        }
        take_decoded(input, len, UTF16_STRING, |bytes| {
            let utf16: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u16::from_be_bytes(*pair))
                .collect();
            Ok(String::from_utf16_lossy(&utf16))
        })
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
        take_decoded(input, len, "ASCII string", |bytes| {
            bytes
                .is_ascii()
                // bytes are verified ASCII above, so `from_utf8_lossy` never substitutes
                .then(|| String::from_utf8_lossy(bytes).into_owned())
                .ok_or_else(ContextError::new)
        })
    }
}

/// Generates a length-bounded base-10 unsigned integer decoder
macro_rules! ascii_uint {
    ($ty:ty) => {
        pastey::paste! {
            #[cfg(feature = "ascii")]
            #[inline(always)]
            #[doc = concat!("Decodes exactly `len` ASCII bytes as a base-10 [`prim@", stringify!($ty), "`].")]
            #[doc = ""]
            #[doc = "The entire `len`-byte field must be the decimal numeral; any"]
            #[doc = "trailing or non-digit byte is rejected."]
            pub fn [<$ty>](len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<$ty> {
                move |input| {
                    take_decoded(
                        input,
                        len,
                        concat!("ASCII base-10 `", stringify!($ty), "`"),
                        |mut text| terminated(dec_uint::<_, $ty, _>, eof).parse_next(&mut text),
                    )
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
            #[doc = concat!("Decodes exactly `len` ASCII bytes as a base-10 [`prim@", stringify!($ty), "`].")]
            #[doc = ""]
            #[doc = "Accepts an optional leading `+`/`-` sign. The entire `len`-byte"]
            #[doc = "field must be the numeral; any trailing byte is rejected."]
            pub fn [<$ty>](len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<$ty> {
                move |input| {
                    take_decoded(
                        input,
                        len,
                        concat!("ASCII base-10 `", stringify!($ty), "`"),
                        |mut text| terminated(dec_int::<_, $ty, _>, eof).parse_next(&mut text),
                    )
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
            #[doc = concat!("Decodes exactly `len` ASCII bytes as an [`prim@", stringify!($ty), "`].")]
            #[doc = ""]
            #[doc = "Accepts standard textual float syntax (sign, exponent, `inf`,"]
            #[doc = "`nan`). The entire `len`-byte field must be the number."]
            pub fn [<$ty>](len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<$ty> {
                move |input| {
                    take_decoded(
                        input,
                        len,
                        concat!("ASCII `", stringify!($ty), "`"),
                        |mut text| terminated(float::<_, $ty, _>, eof).parse_next(&mut text),
                    )
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
            #[doc = concat!("Decodes exactly `len` ASCII hex bytes as a [`prim@", stringify!($ty), "`].")]
            #[doc = ""]
            #[doc = "Accepts upper- or lower-case hex digits. The entire `len`-byte"]
            #[doc = "field must be hexadecimal; any trailing byte is rejected."]
            pub fn [<hex_ $ty>](len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<$ty> {
                move |input| {
                    take_decoded(
                        input,
                        len,
                        concat!("ASCII base-16 `", stringify!($ty), "`"),
                        |mut text| terminated(hex_uint::<_, $ty, _>, eof).parse_next(&mut text),
                    )
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
                take_decoded(
                    input,
                    len,
                    concat!("ASCII ", $class, " string"),
                    |mut text| {
                        let run = terminated($parser, eof).parse_next(&mut text)?;
                        // the run parser only matches ASCII, so `from_utf8_lossy` never substitutes
                        Ok(String::from_utf8_lossy(run).into_owned())
                    },
                )
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

/// Reads the `len` bytes of a value, then decodes them with `content`
///
/// # Arguments
///
/// * `input` - the stream the value is read from
/// * `len` - the value's length in bytes
/// * `label` - what the value is, the label of either error
/// * `content` - the decode of the value's bytes
///
/// # Returns
///
/// What `content` decoded
///
/// # Errors
///
/// - fewer than `len` bytes: [`crate::ErrorKind::Truncated`], the cursor left at the end of the
///   input
/// - `content` fails: [`crate::ErrorKind::MalformedValue`], `content`'s error relabelled `label`
///   ([`relabeled_error`]), the cursor rewound to where the value started
// `inline(always)`: an outlined copy per decoder measured slower in derived decodes
#[inline(always)]
fn take_decoded<'i, O>(
    input: &mut &'i [u8],
    len: usize,
    label: &'static str,
    content: impl FnOnce(&'i [u8]) -> Result<O, ContextError>,
) -> crate::Result<O> {
    let start = input.checkpoint();
    let Ok(value) = take_or_finish(len).parse_next(input) else {
        return Err(kinded_error(input, &start, label, ErrorKind::Truncated));
    };
    content(value).map_err(|err| {
        input.reset(&start);
        relabeled_error(input, &start, label, err, ErrorKind::MalformedValue)
    })
}
