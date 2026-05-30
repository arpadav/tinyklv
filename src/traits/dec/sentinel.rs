//! Sentinel-seeking trait for locating KLV packet boundaries in a byte stream
//!
//! [`SeekSentinel`] is the stream-scanning half of the full decode pipeline
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
/// both specified in the `#[klv(...)]` attribute block
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
    /// Advances `input` past the sentinel, length prefix, and all body bytes
    /// Pre-sentinel bytes are consumed and discarded. Returns an error only when
    /// the stream ends before a sentinel is found or the declared body length
    /// cannot be satisfied
    ///
    /// # Arguments
    ///
    /// * `input` - The stream to scan; advanced past the complete framed packet on success
    ///
    /// # Returns
    ///
    /// `Ok(S)` - a sub-slice containing exactly the body bytes of the located packet -
    /// or a [`winnow::error::ContextError`] if no sentinel is found or the stream
    /// ends mid-frame
    fn seek_sentinel(input: &mut S) -> crate::Result<S>;
}
