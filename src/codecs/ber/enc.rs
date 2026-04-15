//! Encoders for the Basic Encoding Rules (BER)

/// Encodes a value as a BER-encoded length.
///
/// Generic over `T: OfBerLength`. The decode counterpart
/// ([`super::dec::ber_length`]) returns `usize` (type-erased). Use
/// [`BerLength`](crate::codecs::ber::BerLength) directly for type-preserving roundtrip.
///
/// See [`crate::codecs::ber::BerLength::encode_value`]
pub fn ber_length<T: super::OfBerLength>(input: &T) -> Vec<u8> {
    super::BerLength::<T>::encode_value(input)
}

/// See [`crate::codecs::ber::BerOid::encode_value`]
pub fn ber_oid<T: super::OfBerOid>(input: &T) -> Vec<u8> {
    super::BerOid::<T>::encode_value(input)
}
