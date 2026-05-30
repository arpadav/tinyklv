//! Basic Encoding Rules (BER) length and OID codecs
//!
//! Provides the [`BerLength`] and [`BerOid`] types along with the sealed
//! [`OfBerCommon`] trait that constrains their type parameter to the
//! fixed-width unsigned integers (`u8`..=`u128`).
//!
//! BER length encoding is used in the KLV "L" field. Short form encodes
//! lengths 0..=127 in a single byte. Long form encodes larger lengths as
//! `0x80 | num_bytes` followed by `num_bytes` big-endian bytes.
//!
//! BER-OID encoding is used for the KLV "K" (key) field. Each value is
//! packed 7 bits per byte, MSB-first, with the MSB of each byte indicating
//! whether more bytes follow (continuation = 1, terminator = 0).
//!
//! Free-function wrappers for direct use in `#[klv(...)]` attributes are
//! in [`dec`] and [`enc`].
//!
//! Author: aav
// --------------------------------------------------
// mods
// --------------------------------------------------
pub mod dec;
pub mod enc;

// --------------------------------------------------
// local
// --------------------------------------------------
use crate::prelude::*;

// --------------------------------------------------
// external
// --------------------------------------------------
use num_traits::{
    AsPrimitive, FromPrimitive, ToBytes, ToPrimitive, Unsigned, bounds::UpperBounded,
};
use winnow::error::ParserError;
use winnow::token::{take, take_while};

/// Sealed marker restricting [`OfBerCommon`] to the fixed-width unsigned
/// integer types, so external crates cannot add their own implementations
mod private {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
    impl Sealed for u128 {}
    impl Sealed for usize {}
}

/// The set of unsigned integer types usable as a BER length or BER-OID value.
///
/// This is a sealed trait: it is blanket-implemented for the fixed-width
/// unsigned integers (`u8`..=`u128`) and cannot be implemented downstream
pub trait OfBerCommon:
    Copy
    + ToBytes
    + Unsigned
    + UpperBounded
    + PartialOrd
    + ToPrimitive
    + FromPrimitive
    + AsPrimitive<u128>
    + private::Sealed
{
}
impl<T> OfBerCommon for T where
    T: Copy
        + ToBytes
        + Unsigned
        + UpperBounded
        + PartialOrd
        + ToPrimitive
        + FromPrimitive
        + AsPrimitive<u128>
        + private::Sealed
{
}

#[derive(Debug, PartialEq)]
/// Enum representing Basic-Encoding-Rules (BER) Length Encoding.
///
/// Maximum precision: [`u128`]
///
/// * See: [https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf](https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf)
/// * See: [https://upload.wikimedia.org/wikipedia/commons/1/19/MISB_Standard_0601.pdf](https://upload.wikimedia.org/wikipedia/commons/1/19/MISB_Standard_0601.pdf) page 7
///
/// # Example
///
/// ```
/// use tinyklv::prelude::*;
/// use tinyklv::codecs::ber::BerLength;
///
/// let mut v = Vec::new();
/// BerLength::new(8_500_738_u32).encode_value(&mut v);
/// assert_eq!(vec![128 + 3, 129, 182, 2], v);
/// assert_eq!(BerLength::new(8_500_738_u32), BerLength::decode_value(&mut &vec![128 + 3, 129, 182, 2][..]).unwrap());
/// ```
pub enum BerLength<T: OfBerCommon> {
    Short(u8),
    Long(T),
}
/// [`BerLength`] implementation
impl<T: OfBerCommon> BerLength<T> {
    #[inline(always)]
    /// Returns `true` when `val` can be represented in BER short form
    ///
    /// BER short form requires the value to be strictly less than 128 (0x80),
    /// so that the single length byte has its MSB clear. Values >= 128 must
    /// use long form encoding
    fn can_be_short(val: &T) -> bool {
        #![allow(
            clippy::expect_used,
            reason = "this should never panic, due to trait bounds"
        )]
        val < &T::from_u8(0x80).expect(
            "converting 128 -> u{8,16,32,64,128} should always be permissible, why did this panic?",
        )
    }

    /// Creates a new [`BerLength`] from an unsigned integer, choosing the correct BER form
    ///
    /// Inspects `len` against the short-form threshold (< 128). Values that fit
    /// in short form are stored as [`BerLength::Short`]; all others as
    /// [`BerLength::Long`]. This is the canonical constructor used by both
    /// [`BerLength::encode_value`] and the [`crate::traits::EncodeValue`] impl.
    ///
    /// # Arguments
    ///
    /// * `len` - The unsigned integer length to wrap
    ///
    /// # Returns
    ///
    /// A [`BerLength`] in either `Short` or `Long` variant depending on the magnitude of `len`
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::ber::BerLength;
    ///
    /// assert!(matches!(BerLength::new(47_u32), BerLength::Short(47)));
    /// assert!(matches!(BerLength::new(200_u32), BerLength::Long(200)));
    /// ```
    ///
    /// # Safety
    ///
    /// Uses `expect` internally when converting the value to `u8` for short form;
    /// this cannot panic because the value is verified to be < 128 before the cast,
    /// which is always representable as `u8`
    pub fn new(len: T) -> Self {
        if Self::can_be_short(&len) {
            BerLength::Short(len.to_u8().expect("if unsigned int is less than 128, then it can always fit into u8, why did this panic?"))
        } else {
            BerLength::Long(len)
        }
    }

    /// Convenience static entry point: constructs a [`BerLength`] and immediately encodes it
    ///
    /// Equivalent to `BerLength::new(len).encode_value()`. Useful when you need
    /// the encoded bytes without retaining the wrapper struct.
    ///
    /// # Arguments
    ///
    /// * `len` - The length value to encode
    ///
    /// # Returns
    ///
    /// A [`Vec<u8>`] containing the BER-encoded length
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::ber::BerLength;
    ///
    /// let mut v = Vec::new();
    /// BerLength::encode_value(47_u64, &mut v);
    /// assert_eq!(v, vec![47]);
    /// let mut v = Vec::new();
    /// BerLength::encode_value(201_u64, &mut v);
    /// assert_eq!(v, vec![128 + 1, 201]);
    /// ```
    pub fn encode_value(len: T, out: &mut Vec<u8>) {
        crate::EncodeValue::encode_value(&Self::new(len), out);
    }

    /// Returns the wrapped length value as a [`u128`], regardless of which BER form was used
    ///
    /// Both `Short` and `Long` variants are widened to [`u128`] without loss.
    /// Used by [`crate::codecs::ber::dec::ber_length`] to produce a
    /// uniform [`usize`] for `take(len)` calls.
    pub fn as_u128(&self) -> u128 {
        match self {
            BerLength::Short(len) => *len as u128,
            BerLength::Long(len) => len.as_(),
        }
    }
}
/// [`BerLength`] implementation of [`EncodeValue`]
impl<T: OfBerCommon> crate::EncodeValue for BerLength<T> {
    /// Appends the BER-encoded length bytes of this [`BerLength`] to `out`
    ///
    /// # Example
    ///
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::codecs::ber::BerLength;
    ///
    /// let mut buf = Vec::new();
    /// BerLength::new(47_u64).encode_value(&mut buf);
    /// assert_eq!(buf, vec![47]);
    ///
    /// let mut buf = Vec::new();
    /// BerLength::new(201_u64).encode_value(&mut buf);
    /// assert_eq!(buf, vec![128 + 1, 201]);
    ///
    /// let mut buf = Vec::new();
    /// BerLength::new(123891829038102_u64).encode_value(&mut buf);
    /// assert_eq!(buf, vec![128 + 6, 112, 173, 208, 117, 220, 22]);
    ///
    /// // the static entry point appends the same bytes:
    /// let mut buf = Vec::new();
    /// BerLength::encode_value(201_u64, &mut buf);
    /// assert_eq!(buf, vec![128 + 1, 201]);
    /// ```
    fn encode_value(&self, out: &mut Vec<u8>) {
        match self {
            BerLength::Short(len) => out.push(*len),
            BerLength::Long(len) => {
                // --------------------------------------------------
                // Edge case: If the length fits within a single byte, use the Short form.
                // --------------------------------------------------
                // This should never happen: upon creation, length is checked to be < 128
                // --------------------------------------------------
                if Self::can_be_short(len) {
                    #[allow(
                        clippy::expect_used,
                        reason = "this should never panic, due to trait bounds"
                    )]
                    out.push(len.to_u8().expect("if unsigned int is less than 128, then it can always fit into u8, why did this panic?"));
                    return;
                }
                // --------------------------------------------------
                // count significant big-endian bytes (leading zeros stripped), iterator-only
                // so the prefix byte can be written before the value bytes - no temp Vec, no
                // index-slicing
                // --------------------------------------------------
                let be = len.to_be_bytes();
                let bytes = be.as_ref();
                let leading_zeros = bytes.iter().take_while(|&&b| b == 0).count();
                let significant = bytes.len() - leading_zeros;
                // --------------------------------------------------
                // prefix byte with MSB set to 1, followed by the significant length bytes
                // --------------------------------------------------
                out.push(0b1000_0000 | (significant as u8));
                out.extend(bytes.iter().skip(leading_zeros).copied());
            }
        }
    }
}
/// [`BerLength`] implementation of [`crate::traits::DecodeValue`]
impl<T: OfBerCommon> crate::DecodeValue<&[u8]> for BerLength<T> {
    /// Decode a [`BerLength`] from a [`&[u8]`]
    ///
    /// # Example
    ///
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::codecs::ber::BerLength;
    ///
    /// let value0 = vec![47];
    /// let value1 = vec![128 + 1, 201];
    /// let value2 = vec![128 + 6, 112, 173, 208, 117, 220, 22];
    ///
    /// assert_eq!(BerLength::decode_value(&mut &value0[..]).unwrap(), BerLength::new(47_u64));
    /// assert_eq!(BerLength::decode_value(&mut &value1[..]).unwrap(), BerLength::new(201_u64));
    /// assert_eq!(BerLength::decode_value(&mut &value2[..]).unwrap(), BerLength::new(123891829038102_u64));
    /// ```
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let checkpoint = input.checkpoint();
        // --------------------------------------------------
        // err if no bytes
        // --------------------------------------------------
        let first_byte = take_one(input)?;
        #[allow(
            clippy::indexing_slicing,
            reason = "take_one yields a slice of exactly one byte, so [0] is always in-bounds"
        )]
        let first_byte = first_byte[0];
        // --------------------------------------------------
        // if MSB is not set, it's a short length (single byte)
        // --------------------------------------------------
        if first_byte & 0x80 == 0 {
            return Ok(BerLength::Short(first_byte));
        }
        // --------------------------------------------------
        // extract the number of bytes used for length encoding
        // --------------------------------------------------
        let num_bytes = (first_byte & 0x7F) as usize;
        // --------------------------------------------------
        // ensure there are enough bytes in the stream
        // --------------------------------------------------
        // since 1 was taken from input, this should be
        // `input.len() + 1 < num_bytes + 1`
        // but can be shortened
        // --------------------------------------------------
        if input.len() < num_bytes {
            return Err(winnow::error::ContextError::from_input(input)
                .add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Label("BER length value"),
                )
                .add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Expected(
                        winnow::error::StrContextValue::Description(
                            "enough bytes in stream for length encoding",
                        ),
                    ),
                ));
        }
        // --------------------------------------------------
        // decode the length from the specified number of bytes
        // --------------------------------------------------
        let Some(output) = T::from_u128(parse_length_u128(input, num_bytes)?) else {
            return Err(winnow::error::ContextError::from_input(input)
                .add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Label("BER length value"),
                )
                .add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Expected(
                        winnow::error::StrContextValue::Description("less than u128::MAX"),
                    ),
                ));
        };
        Ok(BerLength::Long(output))
    }
}

#[derive(Debug, PartialEq)]
/// Struct representing Basic Encoding Rules (BER) Object Identifier (OID) encoding.
///
/// Maximum precision: [`u128`]
///
/// * See: [https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf](https://www.itu.int/dms_pubrec/itu-r/rec/bt/R-REC-BT.1563-0-200204-S!!PDF-E.pdf)
/// * See: [https://upload.wikimedia.org/wikipedia/commons/1/19/MISB_Standard_0601.pdf](https://upload.wikimedia.org/wikipedia/commons/1/19/MISB_Standard_0601.pdf) page 7
///
/// # Example
///
/// ```
/// use tinyklv::prelude::*;
/// use tinyklv::codecs::ber::BerOid;
///
/// let mut v = Vec::new();
/// BerOid::encode_value(23298_u64, &mut v);
/// assert_eq!(vec![129, 182, 2], v);
/// assert_eq!(23298_u64, BerOid::decode_value(&mut &vec![129, 182, 2][..]).unwrap().value());
/// ```
pub struct BerOid<T: OfBerCommon> {
    value: T,
}
/// [`BerOid`] implementation
impl<T: OfBerCommon> BerOid<T> {
    /// Wraps an unsigned integer in a [`BerOid`] newtype
    ///
    /// This is the canonical constructor used by both [`BerOid::encode_value`]
    /// and the [`crate::traits::DecodeValue`] impl. The value is stored
    /// verbatim; no encoding happens at construction time.
    ///
    /// # Arguments
    ///
    /// * `value` - The unsigned integer OID value to wrap
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::ber::BerOid;
    ///
    /// let oid = BerOid::new(23298_u64);
    /// assert_eq!(oid.value(), 23298_u64);
    /// ```
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// Returns the wrapped OID value by copy
    ///
    /// Since `T: OfBerCommon` implies `Copy`, this returns the value without
    /// moving or cloning.
    #[must_use]
    pub fn value(&self) -> T {
        self.value
    }

    /// Convenience static entry point: wraps a value in [`BerOid`] and immediately encodes it
    ///
    /// Equivalent to `BerOid::new(value).encode_value()`. Useful when you need
    /// the BER-OID bytes without retaining the wrapper struct.
    ///
    /// # Arguments
    ///
    /// * `value` - The unsigned integer value to encode
    ///
    /// # Returns
    ///
    /// A [`Vec<u8>`] containing the BER-OID-encoded representation of the value
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::ber::BerOid;
    ///
    /// let mut v = Vec::new();
    /// BerOid::encode_value(23298_u64, &mut v);
    /// assert_eq!(v, vec![129, 182, 2]);
    /// ```
    pub fn encode_value(value: T, out: &mut Vec<u8>) {
        crate::EncodeValue::encode_value(&Self::new(value), out);
    }
}
/// [`BerOid`] implementation of [`crate::traits::EncodeValue`]
impl<T: OfBerCommon> crate::EncodeValue for BerOid<T> {
    /// Appends the BER-OID-encoded bytes of this [`BerOid`] to `out`
    ///
    /// # Example
    ///
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::codecs::ber::BerOid;
    ///
    /// let mut buf = Vec::new();
    /// BerOid::new(23298_u64).encode_value(&mut buf);
    /// assert_eq!(buf, vec![129, 182, 2]);
    /// ```
    ///
    /// Please use [`crate::codecs::ber::enc::ber_oid`] instead for
    /// all parsing needs. This struct is meant to be used as a development
    /// tool for encoding values to BER format.
    fn encode_value(&self, out: &mut Vec<u8>) {
        // --------------------------------------------------
        // emit 7-bit groups least-significant first, then reverse just the bytes we
        // appended so the most-significant group leads (matching the prior collect+reverse)
        // --------------------------------------------------
        let start = out.len();
        let mut value = self.value.as_();
        let mut first_byte = true;
        while value > 0 {
            // --------------------------------------------------
            // extract 7 bits at a time
            // --------------------------------------------------
            let byte = (value & 0x7F) as u8;
            value >>= 7;
            if first_byte {
                first_byte = false;
                out.push(byte);
            } else {
                out.push(byte | 0x80);
            }
        }
        if let Some(written) = out.get_mut(start..) {
            written.reverse();
        }
    }
}
/// [`BerOid`] implementation of [`crate::traits::DecodeValue`]
impl<T: OfBerCommon> crate::DecodeValue<&[u8]> for BerOid<T> {
    /// Decode a [`BerOid`] from a [`&[u8]`]
    ///
    /// # Example
    ///
    /// ```
    /// use tinyklv::prelude::*;
    /// use tinyklv::codecs::ber::BerOid;
    ///
    /// assert_eq!(23298_u64, BerOid::decode_value(&mut &vec![129, 182, 2][..]).unwrap().value());
    /// ```
    ///
    /// Please use [`crate::codecs::ber::dec::ber_oid`] instead for
    /// all parsing needs. This struct is meant to be used as a development
    /// tool for parsing BER encoded values.
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let checkpoint = input.checkpoint();
        // --------------------------------------------------
        // BER-OID grammar: `(msb-set)* (msb-unset)`
        //   - zero or more continuation bytes with MSB = 1
        //   - exactly one terminator byte with MSB = 0
        // --------------------------------------------------
        let prefix: &[u8] = take_while(0.., msb_is_set)
            .context(winnow::error::StrContext::Label(
                "BER-OID continuation bytes",
            ))
            .parse_next(input)?;
        let terminator = winnow::binary::be_u8
            .context(winnow::error::StrContext::Label(
                "BER-OID missing terminator byte (MSB unset) at end of input",
            ))
            .parse_next(input)?;
        // --------------------------------------------------
        // accumulate 7 bits per byte, prefix first then terminator
        // --------------------------------------------------
        let output = prefix
            .iter()
            .copied()
            .chain(std::iter::once(terminator))
            .fold(0u128, |acc, b| (acc << 7) | (b & 0x7F) as u128);
        let Some(output) = T::from_u128(output) else {
            return Err(winnow::error::ContextError::from_input(input)
                .add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Label("BER-OID value"),
                )
                .add_context(
                    input,
                    &checkpoint,
                    winnow::error::StrContext::Expected(
                        winnow::error::StrContextValue::Description("less than u128::MAX"),
                    ),
                ));
        };
        Ok(BerOid::new(output))
    }
}

#[inline(always)]
/// Consumes exactly one byte from the input and returns it as a 1-element slice
///
/// Used at the start of both [`BerLength`] and (implicitly) the BER-OID decoder
/// to read the first framing byte before branching on short vs long form.
/// Returns an error if the stream is empty.
fn take_one<'s>(input: &mut &'s [u8]) -> crate::Result<&'s [u8]> {
    take(1usize).parse_next(input)
}

#[inline(always)]
/// Returns `true` when the most-significant bit of `b` is set (i.e. `b >= 0x80`)
///
/// Used as the predicate for `take_while` in [`BerOid`]'s decoder to collect
/// all continuation bytes before the final terminator byte
fn msb_is_set(b: u8) -> bool {
    (b & 0x80) != 0
}

#[inline(always)]
/// Consumes `num_bytes` from the input and combines them into a big-endian [`u128`]
///
/// Each byte is shifted into the accumulator from the right: the first byte
/// becomes the most significant. Called from [`BerLength`]'s decoder after the
/// long-form prefix byte has already been consumed and its `num_bytes` field
/// extracted. Returns an error if fewer than `num_bytes` remain in the stream.
fn parse_length_u128(input: &mut &[u8], num_bytes: usize) -> crate::Result<u128> {
    take(num_bytes)
        .map(|bytes: &[u8]| {
            bytes
                .iter()
                .fold(0u128, |acc, &byte| (acc << 8) | byte as u128)
        })
        .parse_next(input)
}
