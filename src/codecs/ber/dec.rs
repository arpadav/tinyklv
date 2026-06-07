//! Decoders for Basic Encoding Rules (BER)
//!
//! Free-function wrappers around [`crate::codecs::ber::BerLength`] and
//! [`crate::codecs::ber::BerOid`] that expose a flat, winnow-compatible
//! function signature. Use these in `#[klv(dec = ...)]` attributes and
//! anywhere a `fn(&mut &[u8]) -> crate::Result<T>` is needed
//!
//! The encode counterparts live in [`crate::codecs::ber::enc`]
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
use crate::prelude::*;

#[inline(always)]
/// Decodes a BER-encoded length from the input stream, returning the length as [`usize`]
///
/// The decode side erases the type to [`usize`] because decoded lengths feed
/// directly into `winnow::token::take(len)`, which expects a [`usize`]. The
/// encode counterpart ([`super::enc::ber_length`]) is generic over
/// `T: OfBerCommon`. Use [`crate::codecs::ber::BerLength`] directly for a
/// type-preserving roundtrip that keeps the full `T` precision
///
/// # Arguments
///
/// * `input` - The byte stream to read from; advanced past the consumed BER-length bytes
///
/// # Returns
///
/// `Ok(len)` where `len` is the decoded length as a [`usize`], or an error if the
/// stream is empty or contains too few bytes for the declared long-form length
///
/// # Example
///
/// ```rust
/// use tinyklv::codecs::ber::dec::ber_length;
///
/// let mut short: &[u8] = &[47];
/// assert_eq!(ber_length(&mut short).unwrap(), 47_usize);
///
/// let mut long: &[u8] = &[128 + 1, 201];
/// assert_eq!(ber_length(&mut long).unwrap(), 201_usize);
/// ```
pub fn ber_length(input: &mut &[u8]) -> crate::Result<usize> {
    super::BerLength::<u128>::decode_value
        .map(|value| value.as_u128() as usize)
        .parse_next(input)
}

#[inline(always)]
/// Decodes a BER-OID-encoded value from the input stream, returning the raw integer `T`
///
/// Reads zero or more continuation bytes (MSB set) followed by exactly one
/// terminator byte (MSB clear), accumulating 7 bits per byte into the output
/// value. The type `T` must implement [`super::OfBerCommon`], which restricts
/// the set to fixed-width unsigned integers (`u8`..=`u128`)
///
/// Use [`crate::codecs::ber::BerOid`] directly when you need the full
/// newtype wrapper rather than the bare value
///
/// # Arguments
///
/// * `input` - The byte stream to read from; advanced past all consumed BER-OID bytes
///
/// # Returns
///
/// `Ok(value)` with the decoded integer, or an error if the stream ends before
/// a terminator byte is reached or the accumulated value overflows `T`
///
/// # Example
///
/// ```rust
/// use tinyklv::codecs::ber::dec::ber_oid;
///
/// let mut input: &[u8] = &[129, 182, 2];
/// let value: u64 = ber_oid(&mut input).unwrap();
/// assert_eq!(value, 23298_u64);
/// ```
pub fn ber_oid<T: super::OfBerCommon>(input: &mut &[u8]) -> crate::Result<T> {
    super::BerOid::<T>::decode_value
        .map(|oid| oid.value())
        .parse_next(input)
}
