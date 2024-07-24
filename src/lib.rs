#![feature(generic_const_exprs)]
pub use tinyklv_impl::*;

pub trait KeyEncoder<T> {
    fn key_encode(&self, input: T) -> Vec<u8>;
}

pub trait LenEncoder<T> {
    fn len_encode(&self, input: T) -> Vec<u8>;
}

pub trait KeyDecoder<T> {
    fn key_decode<'a>(&self, input: &'a[u8]) -> nom::IResult<&'a[u8], T>;
}

pub trait LenDecoder<T> {
    fn len_decode<'a>(&self, input: &'a[u8]) -> nom::IResult<&'a[u8], T>;
}

// fn default_kl_decoder(input: &[u8]) -> nom::IResult<&[u8], &[u8]> {
//     let (n, _) = nom::bytes::complete::take(1usize)(input)?;
//     match (n.len(), n) {
//         (1, [1]) => Ok((n, n)),
//         _ => Err(nom::Err::Error(nom::error::Error::new(n, nom::error::ErrorKind::Tag))),
//     }
// }