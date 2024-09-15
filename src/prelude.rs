// --------------------------------------------------
// external
// --------------------------------------------------
pub use winnow::prelude::*;
pub use winnow::stream::Stream;
pub use winnow::error::AddContext;

pub trait EncodedOutput<T>: IntoIterator<Item = T> + FromIterator<T> + AsRef<[T]> {}
impl<T, S> EncodedOutput<T> for S
where
    S: IntoIterator<Item = T> + FromIterator<T> + AsRef<[T]>,
    S::IntoIter: ExactSizeIterator
{}

/// Trait for encoding ***data only*** to owned stream-type `O`, where `O` is an owned
/// stream-type of [`winnow::stream::Stream`], with elements `T`.
/// 
/// ```text
///                                  This is what is encoded
///                                 vvvvvvvvvvvvvvvvvvvvvvvvv
/// [ ... key ... | ... length ... | ........ value ........ ]
///                                 ^^^^^^^^^^^^^^^^^^^^^^^^^
/// ```
/// 
/// Common stream types include `&[u8]` and `&str`, therefore the return type of
/// encoding is likely an owned value like [`Vec<u8>`] or [`String`].
/// 
/// This trait is automatically implemented for structs deriving the [`tinyklv::Klv`](crate::Klv)
/// trait, in which every field has an associated encoder for it's type.
/// 
/// Otherwise, this trait can be implemented manually.
/// 
/// For custom encoding functions, ***no need to use this trait***. Instead, please ensure
/// the functions signature matches the following:
/// 
/// ```rust ignore
/// fn encoder_fn_name(..) -> O;
/// ```
/// 
/// # Example
/// 
/// ```rust ignore
/// use tinyklv::Klv;
/// use tinyklv::prelude::*;
/// 
/// struct InnerValue {}
/// 
/// fn ex01_encoder(input: &InnerValue) -> Vec<u8> {
///     return vec![0x65, 0x66, 0x67, 0x68];
/// }
/// 
/// fn ex02_encoder(input: &InnerValue) -> Vec<u8> {
///     return String::from("Y2K").into_bytes();
/// }
/// 
/// impl EncodeValue<u8, Vec<u8>> for InnerValue {
///     fn encode_value(&self) -> Vec<u8> {
///         return String::from("KLV").to_lowercase().into_bytes();
///     }
/// }
/// 
/// #[derive(Klv)]
/// #[klv(
///     stream = &[u8],
///     sentinel = 0x00,
///     key(enc = tinyklv::codecs::binary::enc::u8,
///         dec = tinyklv::codecs::binary::dec::u8),
///     len(enc = tinyklv::codecs::binary::enc::u8,
///         dec = tinyklv::codecs::binary::dec::u8),
/// )]
/// struct MyStruct {
///     #[klv(key = 0x07, enc = ex01_encoder)]
///     example_one: InnerValue,
/// 
///     #[klv(key = 0x0A, enc = ex02_encoder)]
///     example_two: InnerValue,
/// 
///     #[klv(key = 0x8A, enc = InnerValue::encode)]
///     example_three: InnerValue,
/// }
/// 
/// let my_struct_encoded = MyStruct::encode();
/// assert_eq!(my_struct_encoded, vec![
///     0x00,               // sentinel
///     0x10,               // total length
/// 
///     // example 1
///     0x07,               // example 1 key
///     0x04,               // example 1 length
///                         // example 1 value 
///     0x65, 0x66, 0x67, 0x68,
/// 
///     // example 2
///     0x0A,               // example 2 key
///     0x03,               // example 2 length
///     0x59, 0x32, 0x4B,   // example 2 value
/// 
///     // example 3
///     0x8A,               // example 3 key
///     0x03,               // example 3 length
///     0x6B, 0x6C, 0x76,   // example 3 value
/// ]);
/// ```
/// 
/// See [`EncodeManual`] for an example usage of this trait.
pub trait EncodeValue<T, O: EncodedOutput<T>> {
    fn encode_value(&self) -> O;
}

/// Trait for encoding data, with its length and key.
/// 
/// ```text
///                 This is what is encoded
///  vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv
/// [ ... key ... | ... length ... | ........ value ........ ]
///  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
/// ```
/// 
/// # Example 
/// 
/// ```rust
/// use tinyklv::prelude::*;
/// 
/// struct MyStruct {}
/// 
/// impl EncodeValue<u8, Vec<u8>> for MyStruct {
///     fn encode_value(&self) -> Vec<u8> {
///         return "a value".as_bytes().to_vec();
///     }
/// }
/// 
/// let my_struct = MyStruct {};
/// let key_len_val_of_my_struct = my_struct.encode_value().into_klv(
///     [0xFF, 0xBB],   // encoded key (must implement into iter)
///     |x: usize|      // length encoder
///         (x as u8).to_be_bytes().to_vec(), 
/// );
/// assert_eq!(key_len_val_of_my_struct, [
///     0xFF, 0xBB,                     // key
///     0x07,                           // length
///     97, 32, 118, 97, 108, 117, 101, // value
/// ]);
/// ```
/// 
/// See [`EncodeValue`] for more information.
pub trait IntoKlv<T, O: EncodedOutput<T>> {
    fn into_klv(self, encoded_key: impl Into<O>, len_encoder: fn(usize) -> O) -> O;
}
/// [`IntoKlv`] implementation for all types O that implement [`EncodedOutput<T>`]
impl<T, O> IntoKlv<T, O> for O
where
    O: EncodedOutput<T>,
{
    #[inline(always)]
    fn into_klv(self, encoded_key: impl Into<O>, len_encoder: fn(usize) -> O) -> O {
        O::from_iter(encoded_key
            .into()
            .into_iter()
            .chain(len_encoder(self.as_ref().len()).into_iter())
            .chain(self.into_iter())
        )
    }
}
/// [`IntoKlv`] implementation for all types [`Result<O>`] that implement [`EncodedOutput<T>`]
impl<T, O, E> IntoKlv<T, O> for Result<O, E>
where
    O: EncodedOutput<T>,
{
    #[inline(always)]
    fn into_klv(self, encoded_key: impl Into<O>, len_encoder: fn(usize) -> O) -> O {
        match self {
            Ok(x) => x.into_klv(encoded_key, len_encoder),
            Err(_) => O::from_iter(std::iter::empty::<T>())
        }
    }
}
/// [`IntoKlv`] implementation for all types [`Option<O>`] that implement [`EncodedOutput<T>`]
impl<T, O> IntoKlv<T, O> for Option<O>
where
    O: EncodedOutput<T>,
{
    #[inline(always)]
    fn into_klv(self, encoded_key: impl Into<O>, len_encoder: fn(usize) -> O) -> O {
        match self {
            Some(x) => x.into_klv(encoded_key, len_encoder),
            None => O::from_iter(std::iter::empty::<T>())
        }
    }
}

pub trait Encode<T, O: EncodedOutput<T>> {
    fn encode(&self) -> O;
}

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