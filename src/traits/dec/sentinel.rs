//! Sentinel-seeking trait for locating KLV packet boundaries in a byte stream
//!
//! [`SeekSentinel`] is the stream-scanning half of the full decode pipeline.
//! It scans forward through `input` until the type's recognition sentinel bytes
//! are found, then consumes the sentinel, reads the declared body length, takes
//! that many bytes, and returns them as a sub-slice for [`super::DecodeValue`] to
//! decode. Pre-sentinel garbage bytes are silently skipped
//!
//! This trait is automatically implemented by the [`crate::Klv`] derive macro
//! when a `sentinel = ...` attribute is present. Manual impls are rarely needed
//!
//! Author: aav

/// Locates the next KLV packet boundary in `input` and returns the framed body as a sub-slice
///
/// Encode counterpart: sentinel bytes are prepended by [`crate::traits::EncodeFrame`]
///
/// Scans `input` for the type's recognition sentinel, consumes the sentinel
/// bytes and the length prefix that follows them, takes exactly `length` bytes
/// from the stream, and returns those bytes as a new sub-slice. The outer
/// `input` cursor is advanced past the entire framed packet (sentinel + length +
/// body). Pre-sentinel bytes are skipped without error
///
/// This trait is automatically implemented when `sentinel` is set in the
/// [`crate::Klv`] attribute. The sentinel bytes and the length decoder are
/// both specified in the `#[klv(...)]` attribute block. The derived impl never
/// skips a found sentinel: when its length or body fails, `input` is left at
/// that sentinel, where the packet starts
///
/// # Example
///
/// ```rust
/// use tinyklv::prelude::*;
/// use tinyklv::Klv;
///
/// #[derive(Debug, PartialEq, Klv)]
/// #[klv(
///     stream = &[u8],
///     sentinel = b"\xFF",
///     key(enc = tinyklv::codecs::binary::enc::u8, dec = tinyklv::codecs::binary::dec::u8),
///     len(enc = tinyklv::codecs::binary::enc::u8_from_usize, dec = tinyklv::codecs::binary::dec::u8_as_usize),
/// )]
/// struct Packet {
///     #[klv(
///         key = 0x01,
///         dec = tinyklv::codecs::binary::dec::u8,
///         enc = *tinyklv::codecs::binary::enc::u8,
///     )]
///     id: u8,
/// }
///
/// // junk bytes before the sentinel are skipped
/// let bytes: &[u8] = &[0x00, 0x00, 0xFF, 0x02, 0x01, 0x42];
/// let mut input = bytes;
/// let body = Packet::seek_sentinel(&mut input).unwrap();
/// assert_eq!(body, &[0x01, 0x42]);
/// ```
pub trait SeekSentinel<S>: Sized
where
    S: winnow::stream::Stream,
{
    /// Scans `input` for the recognition sentinel, then returns the framed body bytes
    ///
    /// Advances `input` past the sentinel, length prefix, and all body bytes.
    /// Pre-sentinel bytes are consumed and discarded
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to scan; advanced past the complete framed packet on success
    ///
    /// # Returns
    ///
    /// `Ok(S)` - a sub-slice containing exactly the body bytes of the located packet
    ///
    /// # Errors
    ///
    /// - The stream ends before a sentinel is found, or before the declared body length is
    ///   satisfied: `input` is left past only the bytes that cannot begin a packet, which
    ///   [`crate::Decoder`] drops
    /// - The packet length is malformed, an error of kind
    ///   [`ErrorKind::MalformedValue`](crate::ErrorKind::MalformedValue): `input` is left at the
    ///   sentinel, which [`crate::Decoder`] then steps one byte into
    fn seek_sentinel(input: &mut S) -> crate::Result<S>;

    /// [`Self::seek_sentinel`] without building its error, for [`crate::Decoder`], which waits on
    /// every failed seek but one whose packet length is malformed
    ///
    /// The derived impl builds no error of its own here, so a buffer that holds part of a packet
    /// costs a [`crate::Decoder`] no allocation; its [`Self::seek_sentinel`] labels the failure
    /// this returns. Not part of the public API - may change without notice
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to scan, left where [`Self::seek_sentinel`] leaves it
    ///
    /// # Returns
    ///
    /// `Ok(S)` - the body bytes [`Self::seek_sentinel`] returns
    ///
    /// # Errors
    ///
    /// - The kind of the error [`Self::seek_sentinel`] fails with, or
    ///   [`ErrorKind::Truncated`](crate::ErrorKind::Truncated) for an error it does not tag, which
    ///   is waited on as a cut read is
    #[doc(hidden)]
    fn seek_unlabeled(input: &mut S) -> Result<S, crate::ErrorKind> {
        Self::seek_sentinel(input)
            .map_err(|error| crate::ErrorKind::of(&error).unwrap_or(crate::ErrorKind::Truncated))
    }
}
