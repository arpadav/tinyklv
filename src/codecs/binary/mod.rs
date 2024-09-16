// --------------------------------------------------
// local
// --------------------------------------------------
pub mod dec;
pub mod enc;

// --------------------------------------------------
// external
// --------------------------------------------------
use num_traits::ToBytes;
use winnow::Parser;
use std::convert::AsRef;

/// [`FixedLength`] encoder / decoder
pub struct FixedLength {
    pub len: usize,
}
/// [`FixedLength`] implementation
impl FixedLength {
    #[inline(always)]
    pub fn decode<P>(&self, input: &mut &[u8]) -> winnow::PResult<P>
    where
        P: From<u128>,
    {
        crate::codecs::binary::dec::be_u128_lengthed(self.len)
            .parse_next(input)
            .map(|res| res.into())
    }

    #[inline(always)]
    pub fn encode<P>(&self, input: &P) -> Vec<u8>
    where
        P: ToBytes
    {
        input.to_be_bytes().as_ref()[..self.len].to_vec()
    }
    
    #[inline(always)]
    pub fn decode_lengthed<P>(len: usize) -> impl Fn(&mut &[u8]) -> winnow::PResult<P>
    where
        P: From<u128>,
    {
        move |input: &mut &[u8]| {
            crate::codecs::binary::dec::be_u128_lengthed(len)
                .parse_next(input)
                .map(|res| res.into())
        }
    }

    #[inline(always)]
    pub fn encode_lengthed<P>(len: usize) -> impl Fn(&P) -> Vec<u8>
    where
        P: ToBytes
    {
        move |input: &P| input.to_be_bytes().as_ref()[..len].to_vec()
    }
}