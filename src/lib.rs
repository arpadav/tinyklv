#![feature(generic_const_exprs)]
pub use tinyklv_macros::*;

pub trait Encoder<V> {
    fn encode(&self, input: V) -> Vec<u8>;
}

pub trait KeyEncoder<V> {
    fn key_encode(&self, input: V) -> Vec<u8>;
}

pub trait LenEncoder<V> {
    fn len_encode(&self, input: V) -> Vec<u8>;
}

pub trait KeyDecoder<V> {
    fn key_decode(&self, input: &[u8]) -> nom::IResult<&[u8], V>;
}

pub trait LenDecoder<V> {
    fn len_decode(&self, input: &[u8]) -> nom::IResult<&[u8], V>;
}

pub trait FixedDecoder<V> {
    const LEN: usize;
    fn decode(&self, input: &[u8; Self::LEN]) -> V;
}

pub fn check_encoder<V, F: Encoder<V>>(_func: F) {}
pub fn check_key_encoder<V, F: KeyEncoder<V>>(_func: F) {}
pub fn check_len_encoder<V, F: LenEncoder<V>>(_func: F) {}
pub fn check_key_decoder<V, F: KeyDecoder<V>>(_func: F) {}
pub fn check_len_decoder<V, F: LenDecoder<V>>(_func: F) {}
pub fn check_fixed_decoder<V, F: FixedDecoder<V>>(_func: F) {}

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