use num_traits::ToBytes;
use std::convert::AsRef;

pub mod ber;
pub mod binary;
pub mod string;

/// Re-exports path from `codecs::name::dec/enc` -> `codecs::dec/enc::name`
/// 
/// For example: [`crate::codecs::binary::dec`] -> [`crate::codecs::dec::binary`]
macro_rules! re_export {
    ($($module:ident),*) => {
        pub mod dec {
            $(pub use super::$module::dec as $module;)*
        }
        pub mod enc {
            $(pub use super::$module::enc as $module;)*
        }
    };
}
re_export! {
    ber,
    string,
    binary
}

pub struct FixedLength {}
impl FixedLength {
    pub fn decode<P>(len: usize) -> impl Fn(&mut &[u8]) -> winnow::PResult<P>
    where
        P: From<u128>,
    {
        move |input: &mut &[u8]| {
            let val = crate::codecs::binary::dec::be_u128_lengthed(input, len)?;
            Ok(val.into())
        }
    }
    pub fn encode<P>(len: usize) -> impl Fn(&P) -> Vec<u8>
    where
        P: ToBytes
    {
        move |input: &P| input.to_be_bytes().as_ref()[..len].to_vec()
    }
}