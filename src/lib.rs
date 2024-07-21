pub use tinyklv_macros::*;

pub trait Encoder<T, R> {
    fn encode(&self, input: T) -> R;
}

pub trait Decoder {
    fn decode(&self, input: &[u8]) -> nom::IResult<&[u8], &[u8]>;
}