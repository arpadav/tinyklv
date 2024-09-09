//! Encoders for the Basic Encoding Rules (BER)
// --------------------------------------------------
// local
// --------------------------------------------------
// use crate::prelude::*;

/// See [`crate::codecs::ber::BerLength::encode`]
pub fn ber_length<T: super::OfBerLength>(input: &T) -> Vec<u8> {
    super::BerLength::<T>::encode(input)
}

/// See [`crate::codecs::ber::BerOid::encode`]
pub fn ber_oid<T: super::OfBerOid>(input: &T) -> Vec<u8> {
    super::BerOid::<T>::encode(input)
}