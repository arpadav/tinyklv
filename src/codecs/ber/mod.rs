//! Basic Encoding Rules (BER) length and OID codecs
//!
//! Provides the [`BerLength`] and [`BerOid`] types along with the sealed
//! [`OfBerCommon`] trait that constrains their type parameter to the
//! unsigned integers `u8`..=`u128` and `usize`
//!
//! BER length encoding is used in the KLV "L" field. Short form encodes
//! lengths 0..=127 in a single byte. Long form encodes larger lengths as
//! `0x80 | num_bytes` followed by `num_bytes` big-endian bytes
//!
//! BER-OID encoding is used for the KLV "K" (key) field. Each value is
//! packed 7 bits per byte, MSB-first, with the MSB of each byte indicating
//! whether more bytes follow (continuation = 1, terminator = 0)
//!
//! Free-function wrappers for direct use in `#[klv(...)]` attributes are
//! in [`dec`] and [`enc`]
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
use crate::{ErrorKind, prelude::*};

// --------------------------------------------------
// external
// --------------------------------------------------
use num_traits::{
    AsPrimitive, FromPrimitive, ToBytes, ToPrimitive, Unsigned, bounds::UpperBounded,
};
use winnow::stream::Stream;

// --------------------------------------------------
// constants
// --------------------------------------------------
/// The `StrContext::Label` naming a [`BerLength`] value in its decode errors
const BER_LENGTH_LABEL: &str = "BER length value";

/// The `StrContext::Label` naming a [`BerOid`] value in its decode errors
const BER_OID_LABEL: &str = "BER-OID value";

/// The `StrContext::Label` of a [`BerOid`] run that runs out of bytes before its terminator
const OID_TERMINATOR: &str = "BER-OID missing terminator byte (MSB unset) at end of input";

/// What a decoded BER value that overflows the target integer type was expected to be
const FITS_TARGET: &str = "a value that fits the target integer type";

/// What a BER length whose first byte is `0x80`, the indefinite form, was expected to be
const DEFINITE_FORM: &str = "a definite-form length";

/// What a BER length that runs out of bytes was expected to have
const LENGTH_BYTES: &str = "enough bytes in stream for length encoding";

/// The value bits one BER-OID byte carries, below its continuation bit
const OID_GROUP_BITS: usize = 7;

/// The continuation bit of one BER-OID byte, set on every byte of a value but its last
const OID_CONTINUATION_BIT: u8 = 1 << OID_GROUP_BITS;

/// The mask selecting the value bits of one BER-OID byte
const OID_GROUP_MASK: u8 = OID_CONTINUATION_BIT - 1;

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

/// The set of unsigned integer types usable as a BER length or BER-OID value
///
/// This is a sealed trait: it is blanket-implemented for the unsigned
/// integers `u8`..=`u128` and `usize`, and cannot be implemented downstream
pub trait OfBerCommon:
    Copy
    + From<u8>
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
        + From<u8>
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
/// Struct representing Basic-Encoding-Rules (BER) Length Encoding
///
/// The form follows from the length: below 128 the short form, the length in one byte; else the
/// long form, `0x80 | num_bytes`, then the length in `num_bytes` big-endian bytes, leading zeros
/// stripped
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
pub struct BerLength<T: OfBerCommon> {
    value: T,
}
/// [`BerLength`] implementation
impl<T: OfBerCommon> BerLength<T> {
    /// Wraps an unsigned integer length in a [`BerLength`]
    ///
    /// This is the canonical constructor used by both [`BerLength::encode_value`] and the
    /// [`crate::traits::DecodeValue`] impl. The length is stored verbatim; its form is chosen
    /// when it is encoded
    ///
    /// # Arguments
    ///
    /// * `len` - The unsigned integer length to wrap
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::ber::BerLength;
    ///
    /// let length = BerLength::new(200_u32);
    /// assert_eq!(length.value(), 200_u32);
    /// ```
    pub fn new(len: T) -> Self {
        Self { value: len }
    }

    /// Returns the wrapped length by copy
    ///
    /// Since `T: OfBerCommon` implies `Copy`, this returns the length without
    /// moving or cloning
    #[must_use]
    pub fn value(&self) -> T {
        self.value
    }

    /// Convenience static entry point: constructs a [`BerLength`] and immediately encodes it
    ///
    /// Equivalent to `BerLength::new(len).encode_value()`. Useful when you need
    /// the encoded bytes without retaining the wrapper struct
    ///
    /// # Arguments
    ///
    /// * `len` - The length value to encode
    /// * `out` - The buffer the BER-encoded length bytes are appended to
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

    /// Returns the wrapped length widened to a [`u128`], without loss
    pub fn as_u128(&self) -> u128 {
        self.value.as_()
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
        // --------------------------------------------------
        // short form: a length below 128 in one byte, its MSB clear
        // --------------------------------------------------
        if let Some(short @ 0..0x80) = self.value.to_u8() {
            out.push(short);
            return;
        }
        // --------------------------------------------------
        // count significant big-endian bytes (strip leading zeros)
        // --------------------------------------------------
        let be = self.value.to_be_bytes();
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
/// [`BerLength`] implementation of [`crate::traits::DecodeValue`]
impl<T: OfBerCommon> crate::DecodeValue<&[u8]> for BerLength<T> {
    /// Decode a [`BerLength`] from a [`&[u8]`]
    ///
    /// # Errors
    ///
    /// - Runs out of bytes: the cursor is left at the end of the input
    /// - The value does not fit `T`: the cursor is rewound to where it started. A long form may
    ///   carry more length bytes than `T` is wide, provided the leading ones are zero
    /// - The first byte is `0x80`, BER's indefinite form: the cursor is rewound to where it
    ///   started
    ///
    /// Running out of bytes is [`crate::ErrorKind::Truncated`], the others
    /// [`crate::ErrorKind::MalformedValue`]
    ///
    /// A long form is read whatever its byte count, so `[0x81, 0x05]` reads as `[0x05]` does
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
        let Some((&first_byte, rest)) = input.split_first() else {
            return Err(value_error(
                input,
                &checkpoint,
                BER_LENGTH_LABEL,
                LENGTH_BYTES,
                ErrorKind::Truncated,
            ));
        };
        // --------------------------------------------------
        // if MSB is not set, it's a short length (single byte)
        // --------------------------------------------------
        if first_byte & 0x80 == 0 {
            *input = rest;
            return Ok(Self::new(T::from(first_byte)));
        }
        // --------------------------------------------------
        // extract the number of bytes used for length encoding
        // --------------------------------------------------
        let num_bytes = usize::from(first_byte & 0x7F);
        // --------------------------------------------------
        // `0x80` alone is BER's indefinite form, which carries no length
        // --------------------------------------------------
        if num_bytes == 0 {
            return Err(value_error(
                input,
                &checkpoint,
                BER_LENGTH_LABEL,
                DEFINITE_FORM,
                ErrorKind::MalformedValue,
            ));
        }
        // --------------------------------------------------
        // ensure there are enough bytes in the stream
        // --------------------------------------------------
        let Some((bytes, rest)) = rest.split_at_checked(num_bytes) else {
            // consumed to the end, so a caller classes a split long form as truncation
            let _ = input.finish();
            return Err(value_error(
                input,
                &checkpoint,
                BER_LENGTH_LABEL,
                LENGTH_BYTES,
                ErrorKind::Truncated,
            ));
        };
        // --------------------------------------------------
        // decode the length from its last `size_of::<T>()` bytes, which
        // always fit `T`; any bytes ahead of them must be zero
        // --------------------------------------------------
        let (lead, tail) = bytes.split_at(bytes.len().saturating_sub(size_of::<T>()));
        let folded = tail
            .iter()
            .fold(0u128, |acc, &byte| (acc << u8::BITS) | u128::from(byte));
        let Some(output) = T::from_u128(folded).filter(|_| lead.iter().all(|&byte| byte == 0))
        else {
            return Err(overflow_error(input, &checkpoint, BER_LENGTH_LABEL));
        };
        *input = rest;
        Ok(Self::new(output))
    }
}

#[derive(Debug, PartialEq)]
/// Struct representing Basic Encoding Rules (BER) Object Identifier (OID) encoding
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
    /// verbatim; no encoding happens at construction time
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
    /// moving or cloning
    #[must_use]
    pub fn value(&self) -> T {
        self.value
    }

    /// Convenience static entry point: wraps a value in [`BerOid`] and immediately encodes it
    ///
    /// Equivalent to `BerOid::new(value).encode_value()`. Useful when you need
    /// the BER-OID bytes without retaining the wrapper struct
    ///
    /// # Arguments
    ///
    /// * `value` - The unsigned integer value to encode
    /// * `out` - The buffer the BER-OID-encoded bytes are appended to
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
    /// all encoding needs. This struct is meant to be used as a development
    /// tool for encoding values to BER format
    fn encode_value(&self, out: &mut Vec<u8>) {
        // --------------------------------------------------
        // emit groups least-significant first, then reverse just the
        // appended bytes so the most-significant group leads
        // --------------------------------------------------
        let start = out.len();
        let mut value = self.value.as_();
        // --------------------------------------------------
        // the loop writes no group for 0, whose one group is `00`
        // --------------------------------------------------
        if value == 0 {
            out.push(0);
            return;
        }
        let mut first_byte = true;
        while value > 0 {
            // --------------------------------------------------
            // extract one group at a time
            // --------------------------------------------------
            let byte = (value & u128::from(OID_GROUP_MASK)) as u8;
            value >>= OID_GROUP_BITS;
            if first_byte {
                first_byte = false;
                out.push(byte);
            } else {
                out.push(byte | OID_CONTINUATION_BIT);
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
    /// # Errors
    ///
    /// - Runs out of bytes: the cursor is left at the end of the input
    /// - The value does not fit `T`: the cursor is rewound to where it started. A run holds at
    ///   most `ceil(bits(T) / 7)` bytes, leading `0x80` padding included, so a run whose byte at
    ///   that position is still a continuation byte is rejected there, without reading further
    ///
    /// Running out of bytes is [`crate::ErrorKind::Truncated`], the other
    /// [`crate::ErrorKind::MalformedValue`]
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
    /// tool for parsing BER encoded values
    fn decode_value(input: &mut &[u8]) -> crate::Result<Self> {
        let checkpoint = input.checkpoint();
        // --------------------------------------------------
        // BER-OID grammar: `(msb-set)* (msb-unset)`
        //   - zero or more continuation bytes with MSB = 1
        //   - exactly one terminator byte with MSB = 0
        // each byte carries OID_GROUP_BITS value bits, so `T` holds at
        // most ceil(bits(T) / OID_GROUP_BITS) bytes, terminator included
        // --------------------------------------------------
        let max_bytes = (size_of::<T>() * 8).div_ceil(OID_GROUP_BITS);
        let radix = T::from(OID_GROUP_MASK) + T::one();
        let mut rest = *input;
        let mut output = T::zero();
        // --------------------------------------------------
        // fewer than `max_bytes` groups always fit `T`
        // --------------------------------------------------
        for _ in 1..max_bytes {
            let Some((&byte, tail)) = rest.split_first() else {
                return Err(truncated_run_error(input, &checkpoint));
            };
            output = output * radix + T::from(byte & OID_GROUP_MASK);
            rest = tail;
            if !msb_is_set(byte) {
                *input = rest;
                return Ok(BerOid::new(output));
            }
        }
        // --------------------------------------------------
        // byte `max_bytes` must end the run, and `T` must have room
        // for one more group
        // --------------------------------------------------
        let Some((&byte, tail)) = rest.split_first() else {
            return Err(truncated_run_error(input, &checkpoint));
        };
        if msb_is_set(byte) || output > T::max_value() / radix {
            return Err(overflow_error(input, &checkpoint, BER_OID_LABEL));
        }
        *input = tail;
        Ok(BerOid::new(output * radix + T::from(byte & OID_GROUP_MASK)))
    }
}

/// Moves `input` to its end and builds the error for a [`BerOid`] run that runs out of bytes
/// before its terminator
///
/// # Arguments
///
/// * `input` - The input, consumed to the end so a caller classes a split run as truncation
/// * `checkpoint` - The start of the value
///
/// # Returns
///
/// A [`winnow::error::ContextError`] labelled [`OID_TERMINATOR`], of kind
/// [`ErrorKind::Truncated`]
// `cold` keeps the error out of `BerOid`'s decode loop, which then spills fewer registers
#[cold]
fn truncated_run_error<I: Stream>(
    input: &mut I,
    checkpoint: &I::Checkpoint,
) -> winnow::error::ContextError {
    let _ = input.finish();
    let err = crate::__export::labeled_error(input, checkpoint, OID_TERMINATOR);
    crate::__export::with_kind(err, ErrorKind::Truncated)
}

/// Rewinds `input` to `checkpoint` and builds the error for a decoded value that overflows the
/// target integer type
///
/// # Arguments
///
/// * `input` - The input, rewound to `checkpoint` so the caller sees the cursor where the value
///   started
/// * `checkpoint` - The start of the value
/// * `label` - The `StrContext::Label` naming the value
///
/// # Returns
///
/// A [`winnow::error::ContextError`] labelled `label`, expecting [`FITS_TARGET`], of kind
/// [`ErrorKind::MalformedValue`]
fn overflow_error<I: Stream>(
    input: &mut I,
    checkpoint: &I::Checkpoint,
    label: &'static str,
) -> winnow::error::ContextError {
    input.reset(checkpoint);
    value_error(
        input,
        checkpoint,
        label,
        FITS_TARGET,
        ErrorKind::MalformedValue,
    )
}

/// Builds the error for a BER value that failed to decode
///
/// # Arguments
///
/// * `input` - The input the error is anchored to
/// * `checkpoint` - The start of the value
/// * `label` - The `StrContext::Label` naming the value
/// * `expected` - The `StrContext::Expected` description of what the value had to be
/// * `kind` - What the failure is
///
/// # Returns
///
/// A [`winnow::error::ContextError`] labelled `label`, expecting `expected`, of kind `kind`
fn value_error<I: Stream>(
    input: &I,
    checkpoint: &I::Checkpoint,
    label: &'static str,
    expected: &'static str,
    kind: ErrorKind,
) -> winnow::error::ContextError {
    let err = crate::__export::labeled_error(input, checkpoint, label).add_context(
        input,
        checkpoint,
        winnow::error::StrContext::Expected(winnow::error::StrContextValue::Description(expected)),
    );
    crate::__export::with_kind(err, kind)
}

#[inline(always)]
/// Returns `true` when the most-significant bit of `b` is set (i.e. `b >= 0x80`)
///
/// [`BerOid`]'s decoder reads a run's bytes until one has it clear, the run's terminator
fn msb_is_set(b: u8) -> bool {
    (b & OID_CONTINUATION_BIT) != 0
}
