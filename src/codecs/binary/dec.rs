// --------------------------------------------------
// external
// --------------------------------------------------
use crate::prelude::*;
use winnow::token::take;

// --------------------------------------------------
// constants
// --------------------------------------------------
const B8_PADDED: &[u8; 1] = &[0];
const B16_PADDED: &[u8; 2] = &[0, 0];
const B32_PADDED: &[u8; 4] = &[0, 0, 0, 0];
const B64_PADDED: &[u8; 8] = &[0, 0, 0, 0, 0, 0, 0, 0];
const B128_PADDED: &[u8; 16] = &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

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
#[cfg(feature = "ascii")]
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
        #[doc = concat!(" Wrapper for [`winnow::binary::be_", stringify!($ty), "`] with implied generics `<&[prim@u8], winnow::error::ContextError>`")]
        pub fn [<be_ $ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::[<be_ $ty>].parse_next(input)
        }
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::le_", stringify!($ty), "`] with implied generics `<&[prim@u8], winnow::error::ContextError>`")]
        pub fn [<le_ $ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::[<le_ $ty>].parse_next(input)
        }
    }};
}
macro_rules! wrap_native {
    ($ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::", stringify!($ty), "`] with implied native-endianness generics `<&[prim@u8], winnow::error::ContextError>`")]
        pub fn [<$ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::$ty(winnow::binary::Endianness::Native).parse_next(input)
        }
    }};
    (simple $ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::", stringify!($ty), "`] with implied native-endianness generics `<&[prim@u8], winnow::error::ContextError>`")]
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
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::", stringify!($parser), "`] with implied generics `<&[prim@u8], winnow::error::ContextError>`")]
        #[doc = ""]
        #[doc = concat!(" See: [`", stringify!($parser), "()`] for the direct [`prim@", stringify!($parser), "`] implementation.")]
        pub fn [<$parser _as_usize>](input: &mut &[u8]) -> winnow::PResult<usize> {
            $parser(input).map(|val| val as usize)
        }
        #[inline(always)]
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::be_", stringify!($parser), "`] with implied generics `<&[prim@u8], winnow::error::ContextError>`")]
        #[doc = ""]
        #[doc = concat!(" See: [`be_", stringify!($parser), "`] for the direct [`prim@", stringify!($parser), "`] implementation.")]
        pub fn [<be_ $parser _as_usize>](input: &mut &[u8]) -> winnow::PResult<usize> {
            [<be_ $parser>](input).map(|val| val as usize)
        }
        #[inline(always)]
        #[doc = concat!(" [`usize`] wrapper for [`winnow::binary::le_", stringify!($parser), "`] with implied generics `<&[prim@u8], winnow::error::ContextError>`")]
        #[doc = ""]
        #[doc = concat!(" See: [`le_", stringify!($parser), "`] for the direct [`prim@", stringify!($parser), "`] implementation.")]
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

macro_rules! lengthed_be {
    ($type:ty, $len:expr, $pad:expr, $doc:expr) => { paste::paste! {
        #[inline(always)]
        #[doc = $doc]
        pub fn [<be_ $type _lengthed>](input: &mut &[u8], len: usize) -> winnow::PResult<$type> {
            let value = take(len).parse_next(input)?;
            match len > $len {
                true => {
                    let mut value = &value[value.len() - $len..];
                    [<be_ $type>](&mut value)
                },
                false => {
                    let pval = &mut $pad.clone();
                    pval[($len - len)..].copy_from_slice(&value);
                    Ok($type::from_be_bytes(*pval))
                },
            }
        }
    }};
    ($type:ty, $len:expr, $pad:expr) => { lengthed_be!($type, $len, $pad, ""); };
}
macro_rules! lengthed_le {
    ($type:ty, $precision_len:expr, $pad:expr, $doc:expr) => { paste::paste! {
        #[inline(always)]
        #[doc = $doc]
        pub fn [<le_ $type _lengthed>](input: &mut &[u8], len: usize) -> winnow::PResult<$type> {
            let value = take(len).parse_next(input)?;
            match len > $precision_len {
                true => {
                    let mut value = &value[..$precision_len];
                    [<le_ $type>](&mut value)
                },
                false => {
                    let pval = &mut $pad.clone();
                    pval[..len].copy_from_slice(&value);
                    Ok($type::from_le_bytes(*pval))
                },
            }
        }
    }};
    ($type:ty, $len:expr, $pad:expr) => { lengthed_le!($type, $len, $pad, ""); };
}

lengthed_be!(u8, 1, B8_PADDED);
lengthed_be!(u16, 2, B16_PADDED, "
Converts a [`prim@u8`] slice of any length into a [`prim@u16`] value
using big-endian encoding.

* len 8: [SKIP, SKIP, SKIP, SKIP, SKIP, SKIP, VAL, VAL] -> [VAL, VAL]
* len 1: [VAL] -> [0x00, VAL]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0x01, 0xE0, 0xFF, 0xFF];
let mut input2: &[u8] = &[0x01, 0xE0];
let mut input3: &[u8] = &[0xE0];
assert_eq!(tinyklv::dec::binary::be_u16_lengthed(&mut input1, 4), Ok(0xFFFF));
assert_eq!(tinyklv::dec::binary::be_u16_lengthed(&mut input2, 2), Ok(0x01E0));
assert_eq!(tinyklv::dec::binary::be_u16_lengthed(&mut input3, 1), Ok(0x00E0));
```
");
lengthed_be!(u32, 4, B32_PADDED, "
Converts a [`prim@u8`] slice of any length into a [`prim@u32`] value
using big-endian encoding.

* len 8: [SKIP, SKIP, SKIP, SKIP, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL]
* len 1: [VAL] -> [0x00, 0x00, 0x00, VAL]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0x00, 0x01, 0xE0, 0xFF, 0xFF];
let mut input2: &[u8] = &[0x00, 0x01, 0xE0, 0xFF];
let mut input3: &[u8] = &[0x01, 0xE0, 0xFF];
assert_eq!(tinyklv::dec::binary::be_u32_lengthed(&mut input1, 5), Ok(0x01E0FFFF));
assert_eq!(tinyklv::dec::binary::be_u32_lengthed(&mut input2, 4), Ok(0x0001E0FF));
assert_eq!(tinyklv::dec::binary::be_u32_lengthed(&mut input3, 3), Ok(0x0001E0FF));
```
");
lengthed_be!(u64, 8, B64_PADDED, "
Converts a [`prim@u8`] slice of any length into a [`prim@u64`] value
using big-endian encoding.

* len 10: [SKIP, SKIP, VAL, VAL, VAL, VAL, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL, VAL, VAL, VAL, VAL]
* len 1: [VAL] -> [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, VAL]
* len 5: [VAL, VAL, VAL, VAL, VAL] -> [0x00, 0x00, 0x00, VAL, VAL, VAL, VAL, VAL]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00, 0x00];
let mut input2: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00, 0x00];
let mut input3: &[u8] = &[0x00, 0x00, 0x01, 0xE0, 0xFF, 0xFF, 0x00];
assert_eq!(tinyklv::dec::binary::be_u64_lengthed(&mut input1, 9), Ok(0x00_01_E0_FF_FF_00_00_00));
assert_eq!(tinyklv::dec::binary::be_u64_lengthed(&mut input2, 8), Ok(0x00_00_01_E0_FF_FF_00_00));
assert_eq!(tinyklv::dec::binary::be_u64_lengthed(&mut input3, 7), Ok(0x00_00_00_01_E0_FF_FF_00));
```
");
lengthed_be!(u128, 16, B128_PADDED);
lengthed_be!(i8, 1, B8_PADDED);
lengthed_be!(i16, 2, B16_PADDED);
lengthed_be!(i32, 4, B32_PADDED);
lengthed_be!(i64, 8, B64_PADDED);
lengthed_be!(i128, 16, B128_PADDED);
lengthed_le!(u8, 1, B8_PADDED);
lengthed_le!(u16, 2, B16_PADDED, "
Converts a [`prim@u8`] slice of any length into a [`prim@u16`] value
using little-endian encoding.

* len 8: [VAL, VAL, SKIP, SKIP, SKIP, SKIP, SKIP, SKIP] -> [VAL, VAL]
* len 1: [VAL] -> [VAL, 0x00]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF];
let mut input2: &[u8] = &[0x01];
let num1 = tinyklv::dec::binary::le_u16_lengthed(&mut input1, 5);
let num2 = tinyklv::dec::binary::le_u16_lengthed(&mut input2, 1);
assert_eq!(num1, Ok(480));
assert_eq!(num2, Ok(1));
```
");
lengthed_le!(u32, 4, B32_PADDED, "
Converts a [`prim@u8`] slice of any length into a [`prim@u32`] value
using little-endian encoding.

* len 8: [VAL, VAL, VAL, VAL, SKIP, SKIP, SKIP, SKIP] -> [VAL, VAL, VAL, VAL]
* len 1: [VAL] -> [VAL, 0x00, 0x00, 0x00]
* len 3: [VAL, VAL, VAL] -> [VAL, VAL, VAL, 0x00]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF];
let mut input2: &[u8] = &[0x01];
let mut input3: &[u8] = &[0x01, 0x02, 0x03];
let num1 = tinyklv::dec::binary::le_u32_lengthed(&mut input1, 5);
let num2 = tinyklv::dec::binary::le_u32_lengthed(&mut input2, 1);
let num3 = tinyklv::dec::binary::le_u32_lengthed(&mut input3, 3);
assert_eq!(num1, Ok(4_294_902_240));
assert_eq!(num2, Ok(1));
assert_eq!(num3, Ok(197_121));
```
");
lengthed_le!(u64, 8, B64_PADDED, "
Converts a [`prim@u8`] slice of any length into a [`prim@u64`] value
using little-endian encoding.

* len 8: [VAL, VAL, VAL, VAL, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL, VAL, VAL, VAL, VAL]
* len 1: [VAL] -> [VAL, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
* len 7: [VAL, VAL, VAL, VAL, VAL, VAL, VAL] -> [VAL, VAL, VAL, VAL, VAL, VAL, VAL, 0x00]

# Example

```rust
use tinyklv::prelude::*;

let mut input1: &[u8] = &[0xE0, 0x01, 0xFF, 0xFF, 0xFF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
let mut input2: &[u8] = &[0x01];
let mut input3: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07];
let num1 = tinyklv::dec::binary::le_u64_lengthed(&mut input1, 16);
let num2 = tinyklv::dec::binary::le_u64_lengthed(&mut input2, 1);
let num3 = tinyklv::dec::binary::le_u64_lengthed(&mut input3, 7);
assert_eq!(num1, Ok(1_099_511_562_720));
assert_eq!(num2, Ok(1));
assert_eq!(num3, Ok(1_976_943_448_883_713));
```
");
lengthed_le!(u128, 16, B128_PADDED);
lengthed_le!(i8, 1, B8_PADDED);
lengthed_le!(i16, 2, B16_PADDED);
lengthed_le!(i32, 4, B32_PADDED);
lengthed_le!(i64, 8, B64_PADDED);
lengthed_le!(i128, 16, B128_PADDED);