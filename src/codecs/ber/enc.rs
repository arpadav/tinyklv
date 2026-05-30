//! Encoders for the Basic Encoding Rules (BER)
//!
//! Free-function wrappers around [`crate::codecs::ber::BerLength`] and
//! [`crate::codecs::ber::BerOid`] that expose a flat function signature
//! compatible with `#[klv(enc = ...)]` attributes. Use these anywhere a
//! `fn(T) -> Vec<u8>` encoder is required.
//!
//! The decode counterparts live in [`crate::codecs::ber::dec`].
//!
//! Author: aav

/// Encodes a value as a BER-encoded length, returning the encoded bytes
///
/// Generic over `T: OfBerCommon`, which covers `u8`..=`u128`. Values less
/// than 128 are encoded in short form (1 byte); larger values use long form
/// where the first byte carries `0x80 | num_bytes`, followed by the value's
/// significant bytes in big-endian order, leading zeros stripped.
///
/// The decode counterpart ([`super::dec::ber_length`]) erases the type to
/// [`usize`]. Use [`crate::codecs::ber::BerLength`] directly for a
/// type-preserving roundtrip that preserves the full `T` precision.
///
/// # Arguments
///
/// * `input` - The unsigned integer length to encode
///
/// # Returns
///
/// A [`Vec<u8>`] containing the BER-encoded representation of the length
///
/// # Example
///
/// ```rust
/// use tinyklv::codecs::ber::enc::ber_length;
///
/// // short form: value < 128
/// let mut short = Vec::new();
/// ber_length(47_u64, &mut short);
/// assert_eq!(short, vec![47]);
///
/// // long form: value >= 128
/// let mut long = Vec::new();
/// ber_length(201_u64, &mut long);
/// assert_eq!(long, vec![128 + 1, 201]);
/// ```
pub fn ber_length<T: super::OfBerCommon>(input: T, out: &mut Vec<u8>) {
    super::BerLength::<T>::encode_value(input, out);
}

/// Encodes a value as BER-OID variable-length bytes, returning the encoded bytes
///
/// Each 7 bits of the input value occupy one output byte, with the most
/// significant group first. All bytes except the last have their MSB set to 1
/// (continuation); the last byte has MSB 0 (terminator). A value of 0 encodes
/// to a single zero byte.
///
/// # Arguments
///
/// * `input` - The unsigned integer value to encode as a BER-OID
///
/// # Returns
///
/// A [`Vec<u8>`] containing the BER-OID-encoded representation of the value
///
/// # Example
///
/// ```rust
/// use tinyklv::codecs::ber::enc::ber_oid;
///
/// let mut out = Vec::new();
/// ber_oid(23298_u64, &mut out);
/// assert_eq!(out, vec![129, 182, 2]);
/// ```
pub fn ber_oid<T: super::OfBerCommon>(input: T, out: &mut Vec<u8>) {
    super::BerOid::<T>::encode_value(input, out);
}
