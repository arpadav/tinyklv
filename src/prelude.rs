// --------------------------------------------------
// external
// --------------------------------------------------
pub use winnow::prelude::*;
pub use winnow::stream::Stream;
pub use winnow::error::AddContext;

/// Trait for encoding types T to stream-type I, of type [winnow::stream::Stream]
/// 
/// Common examples include [&[u8]] and [&[str]]. Note that due to borrowing rules, the
/// return type of encoding is likely going to be an owned value like [`Vec<u8>`] or
/// [String], but is requireed referenced as a slice upon decoding.
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
/// Common examples of stream types include [&[u8]] and [&[str]]
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
    fn seek(input: &mut I) -> winnow::PResult<I>;
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
        let mut sought = T::seek.parse_next(input)?;
        let result = T::then_decode(&mut sought).parse_next(input);
        result
    }
}

/// Internal trait for parsing and decoding embedded data
/// 
/// See [Extract] for more information
/// 
/// Idea is: 
/// 
/// * [Decode] decodes the data of the packet, without finding it
/// * [Seek] finds the data by the recognition sentinel
/// * [Extract] performs [Seek] -> [Decode]. But upon failure, it has to return the
///   checkpoint to the next item of input, rather than the checkpoint of the
///   sub-slice used in the [Decode] call
/// 
/// [ThenDecode] solves this issue by taking the sub-slice as an input, passing it 
/// to the [Decode] implementation, and upon failure, returning to the original 
/// input checkpoint.
trait ThenDecode<I>: Sized
where
    I: winnow::stream::Stream,
{
    fn then_decode(subslice: &mut I) -> impl FnMut(&mut I) -> winnow::PResult<Self>;
}
/// [ThenDecode] implementation for all types T that implement [Decode]
impl<I, T> ThenDecode<I> for T
where
    I: winnow::stream::Stream,
    T: Decode<I>,
{
    fn then_decode(subslice: &mut I) -> impl FnMut(&mut I) -> winnow::PResult<Self> {
        move |input: &mut I| {
            let checkpoint = input.checkpoint();
            match Self::decode(subslice) {
                Ok(parsed) => Ok(parsed),
                Err(e) => Err(e.backtrack().add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Label("Unable to parse data embedded in packet"),
                )),
            }
        }
    }
}