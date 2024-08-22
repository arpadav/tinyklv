use winnow::prelude::*;
use winnow::token::take;

/// Decodes a byte slice into a [String]
/// 
/// # Example
/// 
/// ```
/// use tinyklv::misb::to_string;
/// let mut val1: &[u8] = &[0x41, 0x46, 0x2D, 0x31, 0x30, 0x31];
/// let res = to_string(&mut val1, 6);
/// assert_eq!(res, Ok(String::from("AF-101")));
/// ```
pub fn to_string<'a>(input: &mut crate::Stream<'a>, len: usize) -> winnow::PResult<String> {
    take(len)
        .map(|slice| String::from_utf8_lossy(slice).to_string())
        .parse_next(input)
}

pub fn someting () {
    // winnow
}