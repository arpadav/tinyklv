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
/// `Ok(len)` where `len` is the decoded length as a [`usize`]
///
/// # Errors
///
/// - Runs out of bytes: the cursor is left at the end of the input
/// - The length does not fit a [`usize`]: the cursor is rewound to where it started
/// - The first byte is `0x80`, BER's indefinite form: the cursor is rewound to where it started
///
/// Running out of bytes is [`crate::ErrorKind::Truncated`], the others
/// [`crate::ErrorKind::MalformedValue`]
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
    super::BerLength::<usize>::decode_value(input).map(|length| length.value())
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
/// `Ok(value)` with the decoded integer
///
/// # Errors
///
/// - Runs out of bytes: the cursor is left at the end of the input
/// - The value does not fit `T`: the cursor is rewound to where it started. A run holds at most
///   `ceil(bits(T) / 7)` bytes, leading `0x80` padding included, so a run whose byte at that
///   position is still a continuation byte is rejected there, without reading further
///
/// Running out of bytes is [`crate::ErrorKind::Truncated`], the other
/// [`crate::ErrorKind::MalformedValue`]
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
