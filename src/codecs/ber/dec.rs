//! Decoders for Basic Encoding Rules (BER)
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::prelude::*;

#[inline(always)]
/// Decodes a BER-encoded length, returning `usize` (type-erased).
///
/// The decode side erases to `usize` because lengths feed directly into
/// `winnow::token::take(len)`. The encode side ([`super::enc::ber_length`])
/// is generic over `T: OfBerLength`. Use [`BerLength`](crate::codecs::ber::BerLength)
/// directly for type-preserving roundtrip.
///
/// See [`crate::codecs::ber::BerLength::decode`]
pub fn ber_length(input: &mut &[u8]) -> crate::Result<usize> {
    super::BerLength::<u128>::decode_value
        .map(|value| value.as_u128() as usize)
        .parse_next(input)
}

#[inline(always)]
/// See [`crate::codecs::ber::BerOid::decode`]
pub fn ber_oid<T: super::OfBerOid>(input: &mut &[u8]) -> crate::Result<T> {
    super::BerOid::<T>::decode_value
        .map(|value| value.value)
        .parse_next(input)
}
