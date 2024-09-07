use std::io::Read;

// --------------------------------------------------
// external
// --------------------------------------------------
use crate::prelude::*;
use winnow::token::take;

#[inline(always)]
/// Decodes a byte slice into a [`String`], using [`String::from_utf8_lossy`]
/// 
/// To decode in a more strict manner, please see [`to_string_utf8_strict`]
/// 
/// # Example
/// 
/// ```
/// use tinyklv::codecs::binary::dec::to_string_utf8;
/// 
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
/// 
/// let res1 = to_string_utf8(&mut val1, 6);
/// let res2 = to_string_utf8(&mut val2, 9);
/// 
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string_utf8(input: &mut &[u8], len: usize) -> winnow::PResult<String> {
    take(len)
        .map(|slice| String::from_utf8_lossy(slice).to_string())
        .parse_next(input)
}

#[inline(always)]
/// Decodes a byte slice into a [`String`], using [`String::from_utf8`]
/// 
/// To decode in a more relaxed manner, please see [`to_string_utf8`]
/// 
/// # Example
/// 
/// ```
/// use tinyklv::codecs::binary::dec::to_string_utf8_strict;
///
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
/// 
/// let res1 = to_string_utf8_strict(&mut val1, 6);
/// let res2 = to_string_utf8_strict(&mut val2, 9);
/// 
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string_utf8_strict(input: &mut &[u8], len: usize) -> winnow::PResult<String> {
    let checkpoint = input.checkpoint();
    match String::from_utf8(take(len).parse_next(input)?.to_vec()) {
        Ok(s) => Ok(s),
        Err(_) => Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new().add_context(
            input,
            &checkpoint,
            winnow::error::StrContext::Label("Unable to decode string using `String::from_utf8`")
        ))),
    }
}

#[inline(always)]
/// Decodes a byte slice into a [`String`], using [`String::from_utf16_lossy`]
pub fn to_string_utf16(input: &mut &[u8], len: usize) -> winnow::PResult<String> {
    let checkpoint = input.checkpoint();
    if len % 2 != 0 {
        return Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new().add_context(
            input,
            &checkpoint,
            winnow::error::StrContext::Label("Invalid UTF-16 slice length")
        )))
    }
    take(len).map(|slice: &[u8]| {
        let utf16: Vec<u16> = slice
            .chunks_exact(2)
            .map(|chunk| {
                // safe to unwrap, since `chunks_exact` returns exactly
                // 2 bytes
                let array: [u8; 2] = chunk.try_into().unwrap();
                u16::from_le_bytes(array)
            })
            .collect();
        String::from_utf16_lossy(&utf16)
    }).parse_next(input)
}

#[inline(always)]
/// Decodes a byte slice into a [`String`], using [`ascii::AsciiString::from_ascii`]
/// 
/// # Example
/// 
/// ```
/// use tinyklv::codecs::binary::dec::to_string_ascii;
/// 
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
/// 
/// let res1 = to_string_ascii(&mut val1, 6);
/// let res2 = to_string_ascii(&mut val2, 9);
/// 
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string_ascii(input: &mut &[u8], len: usize) -> winnow::PResult<String> {
    let checkpoint = input.checkpoint();
    match ascii::AsciiString::from_ascii(take(len).parse_next(input)?) {
        Ok(s) => Ok(s.to_string()),
        Err(_) => Err(winnow::error::ErrMode::Backtrack(winnow::error::ContextError::new().add_context(
            input,
            &checkpoint,
            winnow::error::StrContext::Label("Unable to decode string using `ascii::AsciiString::from_ascii`")
        ))),
    }
}

macro_rules! wrap {
    ($ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::be_", stringify!($ty), "`] with implied generics <&[prim@u8], winnow::error::ContextError>")]
        pub fn [<be_ $ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::[<be_ $ty>].parse_next(input)
        }
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::le_", stringify!($ty), "`] with implied generics <&[prim@u8], winnow::error::ContextError>")]
        pub fn [<le_ $ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::[<le_ $ty>].parse_next(input)
        }
    }};
}
macro_rules! wrap_native {
    ($ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::", stringify!($ty), "`] with implied native-endianness generics <&[prim@u8], winnow::error::ContextError>")]
        pub fn [<$ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::$ty(winnow::binary::Endianness::Native).parse_next(input)
        }
    }};
    (simple $ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::", stringify!($ty), "`] with implied native-endianness generics <&[prim@u8], winnow::error::ContextError>")]
        pub fn [<$ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::$ty.parse_next(input)
        }
    }}
}
wrap!(u8);      wrap_native!(simple u8);
wrap!(u16);     wrap_native!(u16);
wrap!(u32);     wrap_native!(u32);
wrap!(u64);     wrap_native!(u64);
wrap!(u128);    wrap_native!(u128);
wrap!(i8);      wrap_native!(simple i8);
wrap!(i16);     wrap_native!(i16);
wrap!(i32);     wrap_native!(i32);
wrap!(i64);     wrap_native!(i64);
wrap!(i128);    wrap_native!(i128);
wrap!(f32);     wrap_native!(f32);
wrap!(f64);     wrap_native!(f64);

macro_rules! as_usize {
    ($parser:ident) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::", stringify!($parser), "`] with implied generics <&[prim@u8], winnow::error::ContextError>")]
        #[doc = ""]
        #[doc = concat!(" See: [`", stringify!($parser), "()`] for the direct [`", stringify!($parser), "()`] implementation.")]
        pub fn [<$parser _as_usize>](input: &mut &[u8]) -> winnow::PResult<usize> {
            $parser(input).map(|val| val as usize)
        }
        #[inline(always)]
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::be_", stringify!($parser), "`] with implied generics <&[prim@u8], winnow::error::ContextError>")]
        #[doc = ""]
        #[doc = concat!(" See: [`be_", stringify!($parser), "`] for the direct [`", stringify!($parser), "()`] implementation.")]
        pub fn [<be_ $parser _as_usize>](input: &mut &[u8]) -> winnow::PResult<usize> {
            [<be_ $parser>](input).map(|val| val as usize)
        }
        #[inline(always)]
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::le_", stringify!($parser), "`] with implied generics <&[prim@u8], winnow::error::ContextError>")]
        #[doc = ""]
        #[doc = concat!(" See: [`le_", stringify!($parser), "`] for the direct [`", stringify!($parser), "()`] implementation.")]
        pub fn [<le_ $parser _as_usize>](input: &mut &[u8]) -> winnow::PResult<usize> {
            [<le_ $parser>](input).map(|val| val as usize)
        }
    }};
}
as_usize!(u8);
as_usize!(u16);
as_usize!(u32);
as_usize!(u64);
as_usize!(u128);