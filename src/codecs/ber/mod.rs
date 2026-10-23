//! Basic Encoding Rules (BER) length and OID codecs
//!
//! Provides the [`BerLength`] and [`BerOid`] types along with the sealed
//! [`OfBerCommon`] trait that constrains their type parameter to the
//! unsigned integers `u8`..=`u128` and `usize`
//!
//! BER length encoding (ITU-T X.690 §8.1.3) is used in the KLV "L" field. Short
//! form encodes lengths 0..=127 in a single byte. Long form encodes larger
//! lengths as `0x80 | num_bytes` followed by `num_bytes` big-endian bytes
//!
//! BER-OID encoding (X.690 §8.19.2, one subidentifier) is used for the KLV "K"
//! (key) field. Each value is packed 7 bits per byte, MSB-first, with the MSB of
//! each byte indicating whether more bytes follow (continuation = 1, terminator = 0)
//!
//! Free-function wrappers for direct use in `#[klv(...)]` attributes are
//! in [`dec`] and [`enc`]
//!
//! The module requires the `ber` feature, on by default
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
    AsPrimitive, FromPrimitive, PrimInt, ToBytes, ToPrimitive, Unsigned, bounds::UpperBounded,
};
use std::iter;
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

/// What a BER length whose first byte is `0x80`, the indefinite form, or `0xFF`, reserved, was
/// expected to be
const DEFINITE_FORM: &str = "a definite-form length";

/// What a BER length that runs out of bytes was expected to have
const LENGTH_BYTES: &str = "enough bytes in stream for length encoding";

/// Bit 8 of a BER length's first byte: clear in the short form, set in the long form
/// (X.690 §8.1.3.4, §8.1.3.5 a))
const LONG_FORM_BIT: u8 = 0x80;

/// Bits 7 to 1 of a long form's first byte, which count the length bytes that follow it
/// (X.690 §8.1.3.5 b))
const LENGTH_COUNT_MASK: u8 = !LONG_FORM_BIT;

/// The fewest length bytes a long form counts: "one or more subsequent octets" (X.690 §8.1.3.5);
/// a count of zero is the indefinite form (§8.1.3.6.1)
const MIN_LENGTH_BYTES: usize = 1;

/// The most length bytes a long form counts: bits 7 to 1 all ones is the first byte `0xFF`,
/// which X.690 §8.1.3.5 c) reserves for a future extension
const MAX_LENGTH_BYTES: usize = LENGTH_COUNT_MASK as usize - 1;

/// The bits in one byte, the unit of a BER length (X.690 §8.1.3.5)
const BYTE_BITS: usize = u8::BITS as usize;

/// The value bits one BER-OID byte carries, below its continuation bit: bits 7 to 1
/// (X.690 §8.19.2)
const OID_GROUP_BITS: usize = 7;

/// The continuation bit of one BER-OID byte, bit 8, set on every byte of a value but its last
/// (X.690 §8.19.2)
const OID_CONTINUATION_BIT: u8 = 1 << OID_GROUP_BITS;

/// The mask selecting the value bits of one BER-OID byte, bits 7 to 1 (X.690 §8.19.2)
const OID_GROUP_MASK: u8 = OID_CONTINUATION_BIT - 1;

/// The position of a BER-OID run's second byte, counted from one: the first byte
/// [`BerOid::decode_run`] reads, its caller having read the first
const OID_SECOND_BYTE: usize = 2;

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
///
/// Its bounds include `num-traits` 0.2 traits, so while the `ber` feature is on, `num-traits` 0.2
/// is part of tinyklv's public API: a new `num-traits` major version is a new tinyklv major version
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Struct representing Basic-Encoding-Rules (BER) Length Encoding
///
/// The form follows from the length: below 128 the short form, the length in one byte; else the
/// long form, `0x80 | num_bytes`, then the length in `num_bytes` big-endian bytes, leading zeros
/// stripped
///
/// Maximum precision: [`u128`]
///
/// * See: ITU-T X.690 §8.1.3, [https://www.itu.int/rec/T-REC-X.690](https://www.itu.int/rec/T-REC-X.690)
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
    #[must_use]
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
    #[must_use]
    pub fn as_u128(&self) -> u128 {
        self.value.as_()
    }

    /// Decodes the rest of a long form (X.690 §8.1.3.5) whose first byte counts `num_bytes`
    /// length bytes
    ///
    /// Kept out of line, so the short-form read [`crate::DecodeValue::decode_value`] makes before
    /// calling it stays small enough to inline at every call site
    ///
    /// # Arguments
    ///
    /// * `input` - The byte stream, still at the long form's first byte
    /// * `num_bytes` - Bits 7 to 1 of the first byte
    /// * `rest` - The bytes after the first
    ///
    /// # Errors
    ///
    /// As [`crate::DecodeValue::decode_value`]
    #[inline(never)]
    fn decode_long_form<'s>(
        input: &mut &'s [u8],
        num_bytes: usize,
        rest: &'s [u8],
    ) -> crate::Result<Self> {
        let checkpoint = input.checkpoint();
        // --------------------------------------------------
        // `0x80`, a count of zero, is the indefinite form (§8.1.3.6.1),
        // which carries no length; `0xFF` is reserved (§8.1.3.5 c))
        // --------------------------------------------------
        if !(MIN_LENGTH_BYTES..=MAX_LENGTH_BYTES).contains(&num_bytes) {
            return Err(value_error(
                input,
                &checkpoint,
                BER_LENGTH_LABEL,
                DEFINITE_FORM,
                ErrorKind::MalformedValue,
            ));
        }
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
        // the length bytes are the length, big-endian. A sender may use
        // more of them than the length needs (§8.1.3.5 NOTE 2), so it
        // fits `T` when all but the last `size_of::<T>()` are zero
        // --------------------------------------------------
        let Some((lead, tail)) = bytes.split_at_checked(num_bytes.saturating_sub(size_of::<T>()))
        else {
            // unreachable: the split point is at most `num_bytes`
            return Err(overflow_error(input, &checkpoint, BER_LENGTH_LABEL));
        };
        // `tail` holds at most `size_of::<T>()` bytes; the constant bound lets the compiler
        // unroll the fold, in one register when `T` is no wider than one
        let folded = (0..size_of::<T>())
            .map_while(|index| tail.get(index))
            .fold(0u128, |acc, &byte| (acc << u8::BITS) | u128::from(byte));
        let Some(output) = T::from_u128(folded).filter(|_| lead.iter().all(|&byte| byte == 0))
        else {
            return Err(overflow_error(input, &checkpoint, BER_LENGTH_LABEL));
        };
        *input = rest;
        Ok(Self::new(output))
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
        if let Some(short @ 0..LONG_FORM_BIT) = self.value.to_u8() {
            out.push(short);
            return;
        }
        match self.value.to_u64() {
            Some(len) => write_long_form(len, out),
            None => write_long_form::<u128>(self.value.as_(), out),
        }
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
    /// - The first byte is `0x80`, BER's indefinite form, or `0xFF`, which X.690 §8.1.3.5 c)
    ///   reserves: the cursor is rewound to where it started
    ///
    /// Running out of bytes is [`crate::ErrorKind::Truncated`], the others
    /// [`crate::ErrorKind::MalformedValue`]
    ///
    /// A long form is read whatever its byte count, as BER lets a sender choose (X.690 §8.1.3.3,
    /// §8.1.3.5 NOTE 2), so `[0x81, 0x05]` reads as `[0x05]` does
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
        match input.split_first() {
            // --------------------------------------------------
            // short form (X.690 §8.1.3.4), read in line: one byte, bit
            // 8 clear, the length in bits 7 to 1
            // --------------------------------------------------
            Some((&first_byte, rest)) if first_byte & LONG_FORM_BIT == 0 => {
                *input = rest;
                Ok(Self::new(T::from(first_byte)))
            }
            // --------------------------------------------------
            // long form (§8.1.3.5 b)): bits 7 to 1 count the length bytes
            // that follow
            // --------------------------------------------------
            Some((&first_byte, rest)) => {
                Self::decode_long_form(input, usize::from(first_byte & LENGTH_COUNT_MASK), rest)
            }
            None => Err(value_error(
                input,
                &input.checkpoint(),
                BER_LENGTH_LABEL,
                LENGTH_BYTES,
                ErrorKind::Truncated,
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Struct representing Basic Encoding Rules (BER) Object Identifier (OID) encoding
///
/// Maximum precision: [`u128`]
///
/// * See: ITU-T X.690 §8.19.2, [https://www.itu.int/rec/T-REC-X.690](https://www.itu.int/rec/T-REC-X.690)
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
    #[must_use]
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

    /// Decodes the rest of a run (X.690 §8.19.2) whose first byte is a continuation byte
    ///
    /// Kept out of line, so the one-byte read [`crate::DecodeValue::decode_value`] makes before
    /// calling it stays small enough to inline at every call site
    ///
    /// # Arguments
    ///
    /// * `input` - The byte stream, still at the run's first byte
    /// * `first_group` - The value bits of the first byte
    /// * `rest` - The bytes after the first
    ///
    /// # Errors
    ///
    /// As [`crate::DecodeValue::decode_value`]
    #[inline(never)]
    fn decode_run<'s>(
        input: &mut &'s [u8],
        first_group: u8,
        mut rest: &'s [u8],
    ) -> crate::Result<Self> {
        let checkpoint = input.checkpoint();
        // --------------------------------------------------
        // BER-OID grammar: `(msb-set)* (msb-unset)`
        //   - zero or more continuation bytes with MSB = 1
        //   - exactly one terminator byte with MSB = 0
        // each byte carries OID_GROUP_BITS value bits, so `T` holds at
        // most ceil(bits(T) / OID_GROUP_BITS) bytes, terminator included
        // --------------------------------------------------
        let max_bytes = (size_of::<T>() * BYTE_BITS).div_ceil(OID_GROUP_BITS);
        let radix = T::from(OID_GROUP_MASK) + T::one();
        let mut output = T::from(first_group);
        // --------------------------------------------------
        // fewer than `max_bytes` groups always fit `T`; the first is
        // already in `output`
        // --------------------------------------------------
        for _ in OID_SECOND_BYTE..max_bytes {
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
/// [`BerOid`] implementation of [`crate::traits::EncodeValue`]
impl<T: OfBerCommon> crate::EncodeValue for BerOid<T> {
    /// Appends the BER-OID-encoded bytes of this [`BerOid`] to `out`, the fewest X.690 §8.19.2
    /// allows
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
        // a value below 128 is one byte, its MSB clear
        // --------------------------------------------------
        if let Some(byte @ 0..OID_CONTINUATION_BIT) = self.value.to_u8() {
            out.push(byte);
            return;
        }
        match self.value.to_u64() {
            Some(value) => write_oid_run(value, out),
            None => write_oid_run::<u128>(self.value.as_(), out),
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
    /// Padding within that limit is read, though X.690 §8.19.2 forbids a sender to write it
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
        match input.split_first() {
            // --------------------------------------------------
            // a one-byte run, read in line: its MSB clear, the value in
            // the other seven bits
            // --------------------------------------------------
            Some((&byte, rest)) if !msb_is_set(byte) => {
                *input = rest;
                Ok(BerOid::new(T::from(byte)))
            }
            Some((&byte, rest)) => Self::decode_run(input, byte & OID_GROUP_MASK, rest),
            None => {
                let checkpoint = input.checkpoint();
                Err(truncated_run_error(input, &checkpoint))
            }
        }
    }
}

/// Appends a length of 128 or more as a long form in the fewest bytes (X.690 §8.1.3.5, and
/// §10.1 for the fewest): the count of its significant bytes, its MSB set, then those bytes,
/// big-endian. A count is at most 16, so the first byte is never the reserved `0xFF`
///
/// Generic so a `u64` length is written in one register
///
/// # Arguments
///
/// * `len` - The length, at least 128
/// * `out` - The buffer the bytes are appended to
fn write_long_form<V: PrimInt + AsPrimitive<u8>>(len: V, out: &mut Vec<u8>) {
    let significant_bits = size_of::<V>() * BYTE_BITS - len.leading_zeros() as usize;
    let num_bytes = significant_bits.div_ceil(BYTE_BITS);
    out.extend(
        iter::once(LONG_FORM_BIT | num_bytes as u8).chain(
            (0..num_bytes)
                .rev()
                .map(|byte| (len >> (byte * BYTE_BITS)).as_()),
        ),
    );
}

/// Appends a value of 128 or more as a BER-OID run in the fewest bytes (X.690 §8.19.2): one byte
/// per started group of its significant bits, the most significant group first, the MSB set on
/// every byte but the last. The first byte holds a significant bit, so it is never `0x80`
///
/// Generic so a `u64` value is written in one register
///
/// # Arguments
///
/// * `value` - The value, at least 128
/// * `out` - The buffer the bytes are appended to
fn write_oid_run<V: PrimInt + AsPrimitive<u8>>(value: V, out: &mut Vec<u8>) {
    let significant_bits = size_of::<V>() * BYTE_BITS - value.leading_zeros() as usize;
    let groups = significant_bits.div_ceil(OID_GROUP_BITS);
    out.extend((0..groups).rev().map(|group| {
        let bits = (value >> (group * OID_GROUP_BITS)).as_() & OID_GROUP_MASK;
        if group == 0 {
            bits
        } else {
            bits | OID_CONTINUATION_BIT
        }
    }));
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
