// --------------------------------------------------
// external
// --------------------------------------------------
pub use winnow::prelude::*;
pub use winnow::stream::Stream;
pub use winnow::error::AddContext;

pub trait EncodedValue<T>: IntoIterator<Item = T> + FromIterator<T> + AsRef<[T]> {}
impl<T, S> EncodedValue<T> for S
where
    S: IntoIterator<Item = T> + FromIterator<T> + AsRef<[T]>,
    S::IntoIter: ExactSizeIterator
{}

/// Trait for encoding data to owned stream-type O, where O is an owned
/// stream-type of [`winnow::stream::Stream`], with elements T.
/// 
/// Common examples include `&[u8]` and `&str`. Note that due to borrowing rules, the
/// return type of encoding is likely going to be an owned value like [`Vec<u8>`] or
/// [`String`], but is requireed referenced as a slice upon decoding.
/// 
/// Automatically implemented for structs deriving the [`tinyklv::Klv`](crate::Klv) trait.
/// 
/// For custom encoding functions, ***no need to use this trait***. Instead, please ensure
/// the functions signature matches the following:
/// 
/// `fn <name>(&V) -> O;` or `fn <name>(V) -> O;`
pub trait Encode<T, O: EncodedValue<T>> {
    fn encode(&self) -> O;
}

pub trait EncodeKlv<T, O: EncodedValue<T>> {
    fn encode_klv(&self, encoded_key: impl Into<O>, len_encoder: fn(usize) -> O) -> O;
}
/// [`EncodeKlv`] implementation for all types V that implement [`Encode`]
impl<T, O, V> EncodeKlv<T, O> for V
where
    V: Encode<T, O>,
    O: EncodedValue<T>,
{
    fn encode_klv(&self, encoded_key: impl Into<O>, len_encoder: fn(usize) -> O) -> O {
        let encoded_value = self.encode();
        let len = encoded_value.as_ref().len();
        O::from_iter(encoded_key
            .into()
            .into_iter()
            .chain(len_encoder(len).into_iter())
            .chain(encoded_value.into_iter())
        )
    }
}

// /// [`Encode`] implementation for all values which are [`IntoIterator`], and 
// /// each element implements [`Encode`]
// impl<T, I> Encode<u8, Vec<u8>> for I
// where
//     I: IntoIterator<Item = T>,
//     for<'a> &'a I: IntoIterator<Item = &'a T>,
//     T: Encode<u8, Vec<u8>>,
// {
//     fn encode(&self) -> Vec<u8> {
//         self.into_iter()
//             .flat_map(|item| item.encode())
//             .collect()
//     }
// }

/// Trait for decoding from stream-type T, of type [`winnow::stream::Stream`]
/// 
/// Common examples of stream types include `&[u8]` and `&str`
/// 
/// Automatically implemented for structs deriving the [`tinyklv::Klv`](crate::Klv) trait
/// which have decoders for every field covered.
/// 
/// For custom decoding functions, ***no need to use this trait***. Instead, please ensure
/// the functions signature matches the following:
/// 
/// * static length: `fn <name>(input: &mut S) -> winnow::PResult<Self>;`
/// * dynamic length: `fn <name>(input: &mut S, len: usize) -> winnow::PResult<Self>;`
pub trait Decode<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn decode(input: &mut S) -> winnow::PResult<Self>;
}

/// Trait for seeking to the beginning of the prescribed type from a stream
pub trait Seek<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn seek(input: &mut S) -> winnow::PResult<S>;
}

/// Trait for extracting from stream-type T, of type [`winnow::stream::Stream`]
pub trait Extract<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn extract(input: &mut S) -> winnow::PResult<Self>;
}
/// [`Extract`] implementation for all types T that implement [`Seek`] and [`Decode`]
impl<S, T> Extract<S> for T
where
    S: winnow::stream::Stream,
    T: Seek<S> + Decode<S>,
{
    fn extract(input: &mut S) -> winnow::PResult<Self> {
        let mut sought = T::seek.parse_next(input)?;
        let result = T::then_decode(&mut sought).parse_next(input);
        result
    }
}

/// Internal trait for parsing and decoding embedded data
/// 
/// See [`Extract`] for more information
/// 
/// Idea is: 
/// 
/// * [`Decode`] decodes the data of the packet, without finding it
/// * [`Seek`] finds the data by the recognition sentinel
/// * [`Extract`] performs [`Seek`] -> [`Decode`]. But upon failure, it has to return the
///   checkpoint to the next item of input, rather than the checkpoint of the
///   sub-slice used in the [`Decode`] call
/// 
/// [`ThenDecode`] solves this issue by taking the sub-slice as an input, passing it 
/// to the [`Decode`] implementation, and upon failure, returning to the original 
/// input checkpoint.
trait ThenDecode<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn then_decode(subslice: &mut S) -> impl FnMut(&mut S) -> winnow::PResult<Self>;
}
/// [`ThenDecode`] implementation for all types T that implement [`Decode`]
impl<S, T> ThenDecode<S> for T
where
    S: winnow::stream::Stream,
    T: Decode<S>,
{
    fn then_decode(subslice: &mut S) -> impl FnMut(&mut S) -> winnow::PResult<Self> {
        move |input: &mut S| {
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

/// Decodes repeatedly, until it can no longer
/// 
/// Accumulates results in a [`Vec`] and returns
pub trait RepeatedDecode<S>: Sized
where
    S: winnow::stream::Stream,
{
    fn repeated(input: &mut S) -> winnow::PResult<Vec<Self>>;
}
/// [`RepeatedDecode`] implementation for all types T that implement [`Decode`]
impl<S, T> RepeatedDecode<S> for T
where
    S: winnow::stream::Stream,
    T: Decode<S>,
{
    fn repeated(input: &mut S) -> winnow::PResult<Vec<Self>> {
        winnow::combinator::repeat(0.., Self::decode).parse_next(input)
    }
}