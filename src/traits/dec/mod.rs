// --------------------------------------------------
// mods
// --------------------------------------------------
mod breakcond;
mod frame;
mod partial;
mod sentinel;

// --------------------------------------------------
// local
// --------------------------------------------------
pub use crate::prelude::*;
pub use breakcond::*;
pub use frame::*;
pub use partial::*;
pub use sentinel::*;

/// Decodes the value portion of a KLV field from stream-type `S`
///
/// Encode counterpart: [`EncodeValue`](crate::traits::EncodeValue)
///
/// Common examples of stream types include `&[u8]` and `&str`
///
/// Automatically implemented for structs deriving the [`tinyklv::Klv`](crate::Klv) trait which have decoders for every field covered.
///
/// For custom decoding functions, ***no need to use this trait***. Instead, please ensure the functions signature matches the following:
///
/// * fixed length:     `fn <name>(input: &mut S)   -> tinyklv::Result<Self>;`
/// * variable length:  `fn <name>(len: usize)      -> impl Fn(&mut S) -> tinyklv::Result<Self>;`
pub trait DecodeValue<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode_value(input: &mut S) -> crate::Result<Self>;
}

/// Automatically implemented for `Vec<T>` where `T` implements [`DecodeValue`].
///
/// Semantics: repeated inner decode until failure. Appropriate for representing
/// a repeated inner field in one parent KLV packet. For streaming a sequence of
/// top-level packets across fragmented reads, use [`crate::Decoder`] instead.
///
/// Cursor safety: if inner `T::decode_value` fails without consuming any bytes,
/// the outer cursor is rewound to the pre-attempt checkpoint so surrounding
/// parsers see the un-eaten bytes. If inner consumed bytes then failed, the
/// progress is committed and the loop stops.
impl<S, T> DecodeValue<S> for Vec<T>
where
    S: winnow::stream::Stream,
    T: DecodeValue<S>,
{
    #[inline(always)]
    fn decode_value(input: &mut S) -> crate::Result<Self> {
        let mut acc = Vec::new();
        loop {
            let before = input.eof_offset();
            let cp = input.checkpoint();
            match T::decode_value(input) {
                Ok(val) => acc.push(val),
                Err(_) => {
                    if input.eof_offset() == before {
                        input.reset(&cp);
                    }
                    break;
                }
            }
        }
        Ok(acc)
    }
}
