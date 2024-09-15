//! Encoders for the Basic Encoding Rules (BER)
// --------------------------------------------------
// local
// --------------------------------------------------
// use crate::prelude::*;

/// See [`crate::codecs::ber::BerLength::encode_value`]
pub fn ber_length<T: super::OfBerLength>(input: &T) -> Vec<u8> {
    super::BerLength::<T>::encode_value(input)
}

/// See [`crate::codecs::ber::BerOid::encode_value`]
pub fn ber_oid<T: super::OfBerOid>(input: &T) -> Vec<u8> {
    super::BerOid::<T>::encode_value(input)
}