//! Binary codec module for KLV data
//!
//! Re-exports the [`dec`] and [`enc`] sub-modules and provides
//! [`FixedLength`], a struct-based encoder/decoder pair for fields whose
//! byte width is known at construction time but not at compile time.
//!
//! For the common case where the field width is a Rust compile-time constant,
//! use the free functions in [`dec`] and [`enc`] directly (e.g.
//! `tinyklv::codecs::binary::dec::be_u32`). `FixedLength` is intended for
//! dynamic scenarios such as proc-macro-generated code that determines field
//! widths from attributes.
//!
//! Author: aav
// --------------------------------------------------
// local
// --------------------------------------------------
pub mod dec;
pub mod enc;

// --------------------------------------------------
// external
// --------------------------------------------------
use num_traits::ToBytes;
use std::convert::AsRef;
use winnow::Parser;

/// Infallible big-endian widening from a `u128` intermediate into a fixed-width integer
///
/// Implemented for the unsigned primitives `u8` through `u128`. [`FixedLength`] reads its bytes
/// through a `u128` (via [`dec::be_u128_lengthed`]) and then narrows to the requested target via
/// this trait, dropping any surplus high bytes: a `len` wider than the target keeps only the
/// low-order bytes, the same big-endian truncation [`FixedLength::encode`] applies in reverse.
/// The conversion is therefore infallible (never errors), and the trait is sealed - downstream
/// crates cannot implement it.
pub trait FromBeBytes: sealed::Sealed {
    /// Narrows a big-endian `u128` intermediate to `Self`, dropping any surplus high bytes
    ///
    /// # Arguments
    ///
    /// * `value` - The decoded big-endian value, held in a `u128` intermediate
    ///
    /// # Returns
    ///
    /// The value as `Self`; high bytes beyond `Self`'s native width are truncated
    #[must_use]
    fn from_be_u128(value: u128) -> Self;
}
/// Sealing module for [`FromBeBytes`]
mod sealed {
    /// Sealed marker preventing foreign [`super::FromBeBytes`] implementations
    pub trait Sealed {}
}
/// Implements [`FromBeBytes`] and its sealing marker for the given unsigned primitives
macro_rules! impl_from_be_bytes {
    ($($ty:ty),* $(,)?) => {$(
        impl sealed::Sealed for $ty {}
        /// [`FromBeBytes`] implementation
        impl FromBeBytes for $ty {
            #[inline]
            #[allow(
                clippy::unnecessary_cast,
                reason = "the cast is a no-op only for the u128 arm; it narrows for every smaller width"
            )]
            fn from_be_u128(value: u128) -> Self {
                value as $ty
            }
        }
    )*};
}
impl_from_be_bytes!(u8, u16, u32, u64, u128);

/// A runtime-length encoder/decoder pair for big-endian numeric fields
///
/// Stores a byte-width `len` set at construction and applies it to every
/// encode and decode call. Values are always read and written in big-endian
/// order. When the encoded bytes are shorter than the native type width,
/// high bytes are zero-padded; when longer, the most-significant surplus
/// bytes are dropped (truncation on decode, leading-zero strip on encode).
///
/// Decoding targets the unsigned primitives via [`FromBeBytes`] (`u8` through `u128`);
/// encoding accepts any [`ToBytes`] value, signed included. Decoding into a signed type
/// is therefore intentionally unsupported - decode into the unsigned counterpart and cast.
///
/// # Example
///
/// ```rust
/// use tinyklv::codecs::binary::FixedLength;
///
/// let codec = FixedLength { len: 2 };
/// let encoded = codec.encode(&0x0102_u16);
/// assert_eq!(encoded, vec![0x01, 0x02]);
///
/// let decoded: u32 = codec.decode(&mut encoded.as_slice()).unwrap();
/// assert_eq!(decoded, 0x0102_u32);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FixedLength {
    /// The exact number of bytes used for each encode or decode operation
    pub len: usize,
}
/// [`FixedLength`] implementation
impl FixedLength {
    /// Decodes exactly `self.len` bytes from `input` as a big-endian integer, widening to `P`
    ///
    /// Reads `self.len` bytes and interprets them as a big-endian unsigned integer
    /// via a `u128` intermediate, then narrows to `P` via [`FromBeBytes`], dropping any
    /// high bytes beyond `P`'s native width. If fewer than `self.len` bytes remain, an
    /// error is returned.
    ///
    /// # Arguments
    ///
    /// * `input` - The byte stream to decode from; advanced by `self.len` bytes on success
    ///
    /// # Returns
    ///
    /// `Ok(P)` with the decoded value, or an error if the stream is too short
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::binary::FixedLength;
    ///
    /// let codec = FixedLength { len: 2 };
    /// let decoded: u32 = codec.decode(&mut &[0x01, 0x02][..]).unwrap();
    /// assert_eq!(decoded, 0x0102_u32);
    /// ```
    #[inline] // not always inline: small generic wrapper (be_u128_lengthed + narrowing map); the optimizer inlines it at each monomorphization, so forcing it only risks downstream bloat
    pub fn decode<P>(&self, input: &mut &[u8]) -> crate::Result<P>
    where
        P: FromBeBytes,
    {
        crate::codecs::binary::dec::be_u128_lengthed(self.len)
            .parse_next(input)
            .map(P::from_be_u128)
    }

    /// Encodes `input` as big-endian bytes, returning exactly `self.len` bytes
    ///
    /// Converts `input` to big-endian bytes via [`ToBytes::to_be_bytes`], then
    /// takes the last `self.len` bytes (truncating high-order bytes when the
    /// value's native width exceeds `self.len`).
    ///
    /// # Arguments
    ///
    /// * `input` - The numeric value to encode
    ///
    /// # Returns
    ///
    /// A [`Vec<u8>`] of exactly `self.len` bytes
    ///
    /// # Panics
    ///
    /// Panics (slice-bounds underflow) when `self.len` exceeds the native byte width of `P`,
    /// since there are then fewer than `self.len` big-endian bytes to take
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::binary::FixedLength;
    ///
    /// let codec = FixedLength { len: 2 };
    /// assert_eq!(codec.encode(&0x01020304_u32), vec![0x03, 0x04]);
    /// ```
    #[inline(always)]
    #[allow(
        clippy::indexing_slicing,
        reason = "documented contract: returns the last `self.len` big-endian bytes; a `self.len` \
                  exceeding the value's native width panics on slice underflow, as the doc states"
    )]
    pub fn encode<P>(&self, input: &P) -> Vec<u8>
    where
        P: ToBytes,
    {
        let be = input.to_be_bytes();
        let bytes = be.as_ref();
        bytes[bytes.len() - self.len..].to_vec()
    }

    /// Returns a closure that decodes exactly `len` bytes as a big-endian integer, widening to `P`
    ///
    /// The returned closure captures `len` and behaves identically to
    /// [`FixedLength::decode`] with `self.len == len`. Useful when the length
    /// is known at the call site but a closure is needed (e.g. for `#[klv(dec = ...)]`).
    ///
    /// # Arguments
    ///
    /// * `len` - The number of bytes the closure will consume per call
    ///
    /// # Returns
    ///
    /// A `impl Fn(&mut &[u8]) -> crate::Result<P>` closure
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::binary::FixedLength;
    ///
    /// let dec = FixedLength::decode_lengthed::<u32>(3);
    /// let val: u32 = dec(&mut &[0x01, 0x02, 0x03][..]).unwrap();
    /// assert_eq!(val, 0x010203_u32);
    /// ```
    #[inline] // not always inline: closure factory whose body branches and copies into a pad buffer; let the cost model decide (in-crate LTO inlines it regardless)
    pub fn decode_lengthed<P>(len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<P>
    where
        P: FromBeBytes,
    {
        move |input: &mut &[u8]| {
            crate::codecs::binary::dec::be_u128_lengthed(len)
                .parse_next(input)
                .map(P::from_be_u128)
        }
    }

    /// Returns a closure that encodes values as `len`-byte big-endian byte slices
    ///
    /// The returned closure captures `len` and behaves identically to
    /// [`FixedLength::encode`] with `self.len == len`. Useful when the length
    /// is known at the call site but a closure is needed (e.g. for `#[klv(enc = ...)]`).
    ///
    /// # Arguments
    ///
    /// * `len` - The number of bytes each encoded output will contain
    ///
    /// # Returns
    ///
    /// A `impl Fn(&P) -> Vec<u8>` closure that truncates to the last `len` big-endian bytes
    ///
    /// # Panics
    ///
    /// The returned closure panics (slice-bounds underflow) when `len` exceeds the native byte
    /// width of `P`, since there are then fewer than `len` big-endian bytes to take
    ///
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::binary::FixedLength;
    ///
    /// let enc = FixedLength::encode_lengthed::<u32>(2);
    /// assert_eq!(enc(&0x01020304_u32), vec![0x03, 0x04]);
    /// ```
    #[inline(always)]
    #[allow(
        clippy::indexing_slicing,
        reason = "documented contract: returns the last `len` big-endian bytes; a `len` exceeding \
                  the value's native width panics on slice underflow, as the doc states"
    )]
    pub fn encode_lengthed<P>(len: usize) -> impl Fn(&P) -> Vec<u8>
    where
        P: ToBytes,
    {
        move |input: &P| {
            let be = input.to_be_bytes();
            let bytes = be.as_ref();
            bytes[bytes.len() - len..].to_vec()
        }
    }
}
