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

/// A runtime-length encoder/decoder pair for big-endian numeric fields
///
/// Stores a byte-width `len` set at construction and applies it to every
/// encode and decode call. Values are always read and written in big-endian
/// order. When the encoded bytes are shorter than the native type width,
/// high bytes are zero-padded; when longer, the most-significant surplus
/// bytes are dropped (truncation on decode, leading-zero strip on encode).
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
pub struct FixedLength {
    /// The exact number of bytes used for each encode or decode operation
    pub len: usize,
}
/// [`FixedLength`] implementation
impl FixedLength {
    /// Decodes exactly `self.len` bytes from `input` as a big-endian integer, widening to `P`
    ///
    /// Reads `self.len` bytes and interprets them as a big-endian unsigned integer
    /// via a `u128` intermediate. The result is then converted to `P` via [`From<u128>`].
    /// If fewer than `self.len` bytes remain, an error is returned.
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
    #[inline(always)]
    pub fn decode<P>(&self, input: &mut &[u8]) -> crate::Result<P>
    where
        P: From<u128>,
    {
        crate::codecs::binary::dec::be_u128_lengthed(self.len)
            .parse_next(input)
            .map(std::convert::Into::into)
    }

    /// Encodes `input` as big-endian bytes, returning exactly `self.len` bytes
    ///
    /// Converts `input` to big-endian bytes via [`ToBytes::to_be_bytes`], then
    /// takes the last `self.len` bytes (truncating high-order bytes when the
    /// value's native width exceeds `self.len`). If `self.len` exceeds the
    /// native width, this will panic with an out-of-bounds slice.
    ///
    /// # Arguments
    ///
    /// * `input` - The numeric value to encode
    ///
    /// # Returns
    ///
    /// A [`Vec<u8>`] of exactly `self.len` bytes
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
    pub fn encode<P>(&self, input: &P) -> Vec<u8>
    where
        P: ToBytes,
    {
        input.to_be_bytes().as_ref()[..self.len].to_vec()
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
    #[inline(always)]
    pub fn decode_lengthed<P>(len: usize) -> impl Fn(&mut &[u8]) -> crate::Result<P>
    where
        P: From<u128>,
    {
        move |input: &mut &[u8]| {
            crate::codecs::binary::dec::be_u128_lengthed(len)
                .parse_next(input)
                .map(std::convert::Into::into)
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
    /// # Example
    ///
    /// ```rust
    /// use tinyklv::codecs::binary::FixedLength;
    ///
    /// let enc = FixedLength::encode_lengthed::<u32>(2);
    /// assert_eq!(enc(&0x01020304_u32), vec![0x03, 0x04]);
    /// ```
    #[inline(always)]
    pub fn encode_lengthed<P>(len: usize) -> impl Fn(&P) -> Vec<u8>
    where
        P: ToBytes,
    {
        move |input: &P| input.to_be_bytes().as_ref()[..len].to_vec()
    }
}
