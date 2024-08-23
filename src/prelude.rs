/// Trait for encoding to bytes
pub trait Encode<T> {
    fn encode(&self) -> T;
}

/// Trait for decoding from bytes with a stream of input data
pub trait StreamDecode<I>: Sized
where
    I: winnow::stream::Stream,
{
    fn decode(input: &mut I) -> winnow::PResult<Self>;
}