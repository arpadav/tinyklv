#![feature(generic_const_exprs)]
pub use tinyklv_macros::*;

pub trait Encoder<T> {
    fn encode(&self, input: T) -> Vec<u8>;
}

pub trait KeyEncoder<T> {
    fn key_encode(&self, input: T) -> Vec<u8>;
}

pub trait LenEncoder<T> {
    fn len_encode(&self, input: T) -> Vec<u8>;
}

pub trait KeyDecoder<T> {
    fn key_decode(&self, input: &[u8]) -> nom::IResult<&[u8], T>;
}

pub trait LenDecoder<T> {
    fn len_decode(&self, input: &[u8]) -> nom::IResult<&[u8], T>;
}

pub trait FixedDecoder<T> {
    const LEN: usize;
    fn decode(&self, input: &[u8; Self::LEN]) -> T;
}

pub fn check_encoder<T, F: Encoder<T>>(_func: F) {}
pub fn check_key_encoder<T, F: KeyEncoder<T>>(_func: F) {}
pub fn check_len_encoder<T, F: LenEncoder<T>>(_func: F) {}
pub fn check_key_decoder<T, F: KeyDecoder<T>>(_func: F) {}
pub fn check_len_decoder<T, F: LenDecoder<T>>(_func: F) {}
pub fn check_fixed_decoder<T, F: FixedDecoder<T>>(_func: F) {}

impl FixedDecoder<Vec<f32>> for fn(&[u8; 2]) -> Vec<f32> {
    const LEN: usize = 2;
    fn decode(&self, input: &[u8; 2]) -> Vec<f32> {
        (self)(input)
    }
}

impl Encoder<Vec<f32>> for fn(Vec<f32>) -> Vec<u8> {
    fn encode(&self, input: Vec<f32>) -> Vec<u8> {
        (self)(input)
    }
}

fn default_kl_decoder(input: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    let (n, _) = nom::bytes::complete::take(1usize)(input)?;
    match (n.len(), n) {
        (1, [1]) => Ok((n, n)),
        _ => Err(nom::Err::Error(nom::error::Error::new(n, nom::error::ErrorKind::Tag))),
    }
}