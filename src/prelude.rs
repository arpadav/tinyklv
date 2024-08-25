// --------------------------------------------------
// external
// --------------------------------------------------
pub use winnow::prelude::*;
pub use winnow::stream::Stream;
pub use winnow::error::AddContext;

/// Trait for encoding types T to stream-type I, of type [winnow::stream::Stream]
/// 
/// Common examples include [`&[u8]`] and [`&[str]`]. Note that due to borrowing rules, the
/// return type of encoding is likely going to be an owned value like [`Vec<u8>`] or
/// [`String`], but is requireed referenced as a slice upon decoding.
/// 
/// Automatically implemented for structs deriving the [tinyklv::Klv](crate::Klv) trait.
/// 
/// For custom encoding functions, ***no need to use this trait***. Instead, please ensure
/// the functions signature matches the following:
/// 
/// `fn <name>(&T) -> I;` or `fn <name>(T) -> I;`
pub trait Encode<I> {
    fn encode(&self) -> I;
}

/// Trait for decoding from stream-type T, of type [winnow::stream::Stream]
/// 
/// Common examples of stream types include [`&[u8]`] and [`&[str]`]
/// 
/// Automatically implemented for structs deriving the [tinyklv::Klv](crate::Klv) trait
/// which have decoders for every field covered.
/// 
/// For custom decoding functions, ***no need to use this trait***. Instead, please ensure
/// the functions signature matches the following:
/// 
/// * static length: `fn <name>(input: &mut I) -> winnow::PResult<Self>;`
/// * dynamic length: `fn <name>(input: &mut I, len: usize) -> winnow::PResult<Self>;`
pub trait Decode<I>: Sized
where
    I: winnow::stream::Stream,
{
    fn decode(input: &mut I) -> winnow::PResult<Self>;
}

pub trait Seek<I>: Sized
where
    I: winnow::stream::Stream,
{
    fn seek<'s>(input: &'s mut I) -> winnow::PResult<&'s mut I>;
}

/// Trait for extracting from stream-type T, of type [winnow::stream::Stream]
pub trait Extract<I>: Sized
where
    I: winnow::stream::Stream,
{
    fn extract(input: &mut I) -> winnow::PResult<Self>;
}
/// [Extract] implementation for all types T that implement [Seek] and [Decode]
impl<I, T> Extract<I> for T
where
    I: winnow::stream::Stream,
    T: Seek<I> + Decode<I>,
{
    fn extract(input: &mut I) -> winnow::PResult<Self> {
        T::seek(input).and_then(T::decode)
    }
}