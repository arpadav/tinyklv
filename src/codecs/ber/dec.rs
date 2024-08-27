//! Decoders for Basic Encoding Rules (BER)
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::prelude::*;

/// See [crate::codecs::ber::BerLength::decode]
pub fn ber_length(input: &mut &[u8]) -> winnow::PResult<usize> {
    super::BerLength::<u64>::decode
        .map(|value| value.as_u64() as usize)
        .parse_next(input)
}

/// See [crate::codecs::ber::BerOid::decode]
pub fn ber_oid<T: super::OfBerOid>(input: &mut &[u8]) -> winnow::PResult<T> {
    super::BerOid::<T>::decode
        .map(|value| value.value)
        .parse_next(input)
}