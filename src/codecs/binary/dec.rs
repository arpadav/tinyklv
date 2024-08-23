// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::prelude::*;
use winnow::token::take;

#[inline(always)]
/// Decodes a byte slice into a [String]
/// 
/// # Example
/// 
/// ```
/// use tinyklv::defaults::dec::to_string;
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
/// let res1 = to_string(&mut val1, 6);
/// let res2 = to_string(&mut val2, 9);
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string(input: &mut &[u8], len: usize) -> winnow::PResult<String> {
    take(len)
        .map(|slice| String::from_utf8_lossy(slice).to_string())
        .parse_next(input)
}

macro_rules! wrap {
    ($ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!("Wrapper for [winnow::binary::be_", stringify!($ty), "] with implied generics <&[u8], winnow::error::ContextError>")]
        pub fn [<be_ $ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::[<be_ $ty>].parse_next(input)
        }
        #[inline(always)]
        #[doc = concat!("Wrapper for [winnow::binary::le_", stringify!($ty), "] with implied generics <&[u8], winnow::error::ContextError>")]
        pub fn [<le_ $ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::[<le_ $ty>].parse_next(input)
        }
    }};
}
macro_rules! wrap_native {
    ($ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!("Wrapper for [winnow::binary::", stringify!($ty), "] with implied native-endianness generics <&[u8], winnow::error::ContextError>")]
        pub fn [<$ty>](input: &mut &[u8]) -> winnow::PResult<$ty> {
            winnow::binary::$ty(winnow::binary::Endianness::Native).parse_next(input)
        }
    }};
    (simple $ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!("Wrapper for [winnow::binary::", stringify!($ty), "] with implied native-endianness generics <&[u8], winnow::error::ContextError>")]
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