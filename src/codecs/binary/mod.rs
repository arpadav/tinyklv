// --------------------------------------------------
// local
// --------------------------------------------------
pub mod dec;
pub mod enc;

// --------------------------------------------------
// external
// --------------------------------------------------
use num_traits::ToBytes;
use std::convert::AsRef;

/// [`FixedLength`] encoder/decoder
pub struct FixedLength {
    pub len: usize,
}
/// [`FixedLength`] implementation
impl FixedLength {
    pub fn decode<P>(&self) -> impl Fn(&mut &[u8]) -> winnow::PResult<P>
    where
        P: From<u128>,
    {
        let len = self.len;
        move |input: &mut &[u8]| {
            let val = crate::codecs::binary::dec::be_u128_lengthed(input, len)?;
            Ok(val.into())
        }
    }

    pub fn encode<P>(&self) -> impl Fn(&P) -> Vec<u8>
    where
        P: ToBytes
    {
        let len = self.len;
        move |input: &P| input.to_be_bytes().as_ref()[..len].to_vec()
    }
}