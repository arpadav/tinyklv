/// Trait for encoding to bytes
pub trait Encode {
    fn encode(&self) -> Vec<u8>;
}

/// Trait for decoding from bytes directly, without a stream of input data
pub trait FixedDecode: Sized {
    type Error;
    fn fixed_decode(input: crate::Stream) -> Result<Self, Self::Error>;
}

/// Trait for decoding from bytes with a stream of input data
pub trait StreamDecode: Sized {
    fn decode(input: &mut crate::Stream) -> winnow::PResult<Self>;
}