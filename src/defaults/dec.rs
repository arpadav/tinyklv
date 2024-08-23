use winnow::prelude::*;
use winnow::token::take;

use crate::prelude::*;
use super::codecs::ber;

/// See [ber::BerLength::decode]
pub fn ber_length(input: &mut &[u8]) -> winnow::PResult<usize> {
    ber::BerLength::<u64>::decode
        .map(|value| value.as_u64() as usize)
        .parse_next(input)
}

/// See [ber::BerOid::decode]
pub fn ber_oid<T: ber::OfBerOid>(input: &mut &[u8]) -> winnow::PResult<T> {
    ber::BerOid::<T>::decode
        .map(|value| value.value)
        .parse_next(input)
}

/// Decodes a byte slice into a [String]
/// 
/// # Example
/// 
/// ```
/// use tinyklv::defaults::dec::to_string_parser;
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
/// let res1 = to_string_parser(&mut val1, 6);
/// let res2 = to_string_parser(&mut val2, 9);
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string_parser(input: &mut &[u8], len: usize) -> winnow::PResult<String> {
    take(len)
        .map(|slice| String::from_utf8_lossy(slice).to_string())
        .parse_next(input)
}

/// Decodes a byte slice into a [String]
/// 
/// # Example
/// 
/// ```
/// use winnow::prelude::*;
/// use tinyklv::defaults::dec::to_string;
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let mut val2: &[u8] = &[0x4D, 0x49, 0x53, 0x53, 0x49, 0x4F, 0x4E, 0x30, 0x31];
/// let res1 = to_string(6).parse_next(&mut val1);
/// let res2 = to_string(9).parse_next(&mut val2);
/// assert_eq!(res1, Ok(String::from("AF-101")));
/// assert_eq!(res2, Ok(String::from("MISSION01")));
/// ```
pub fn to_string(len: usize) -> impl FnMut(&mut &[u8]) -> winnow::PResult<String> {
    move |input: &mut &[u8]| to_string_parser(input, len)
}