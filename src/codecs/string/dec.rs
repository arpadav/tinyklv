// --------------------------------------------------
// external
// --------------------------------------------------
use winnow::Parser;

macro_rules! wrap {
    ($ty:ty) => { paste::paste! {
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::be_", stringify!($ty), "`] with implied generics `<&str, winnow::error::ContextError>`")]
        pub fn [<be_ $ty>](input: &mut &str) -> winnow::PResult<$ty> {
            crate::codecs::binary::dec::[<be_ $ty>].parse_next(&mut input.as_bytes())
        }
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::le_", stringify!($ty), "`] with implied generics `<&str, winnow::error::ContextError>`")]
        pub fn [<le_ $ty>](input: &mut &str) -> winnow::PResult<$ty> {
            crate::codecs::binary::dec::[<le_ $ty>].parse_next(&mut input.as_bytes())
        }
        #[inline(always)]
        #[doc = concat!(" Wrapper for [`winnow::binary::", stringify!($ty), "`] with implied native-endianness generics `<&str, winnow::error::ContextError>`")]
        pub fn [<$ty>](input: &mut &str) -> winnow::PResult<$ty> {
            crate::codecs::binary::dec::$ty.parse_next(&mut input.as_bytes())
        }
    }};
}
wrap!(u8);
wrap!(u16);
wrap!(u32);
wrap!(u64);
wrap!(u128);
wrap!(i8);
wrap!(i16);
wrap!(i32);
wrap!(i64);
wrap!(i128);
wrap!(f32);
wrap!(f64);